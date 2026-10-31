//! 驅動 RosettaStone 橋接程式（tools/rosetta-bridge）：JSON lines，一行請求一行回應。

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::actions::Action;
use crate::rng::RngEvent;
use crate::snapshot::Snapshot;

/// 一方的職業與牌組（照抽牌順序，用 RosettaStone 卡號）。
#[derive(Debug, Clone, Serialize)]
pub struct PlayerSetup {
    pub class: String,
    pub deck: Vec<String>,
}

/// 亂數模式：記錄（給種子）或重播（給事件序列）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum RngMode {
    Record { seed: u64 },
    Replay { events: Vec<RngEvent> },
}

#[derive(Debug, Clone, Serialize)]
pub struct GameSetup {
    pub p1: PlayerSetup,
    pub p2: PlayerSetup,
    pub rng: RngMode,
}

/// 橋接程式回報的錯誤：kind 為 illegal_action、rng_desync、engine_exception、bad_request 等。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BridgeError {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Version {
    pub rosetta_commit: String,
    pub patches: Vec<String>,
}

/// 單一請求的回應等待上限；RosettaStone 偶有無窮迴圈（例如兩邊都打不死的「戰鬥到死」）。
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
/// 第一個請求要等橋接程式載入卡表，多個測試同時啟動時可能要好幾十秒。
const STARTUP_TIMEOUT: Duration = Duration::from_secs(180);

/// 一個執行中的橋接程式行程。回應由背景執行緒逐行讀取，逾時就結束行程。
pub struct Bridge {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    alive: bool,
    started: bool,
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if self.alive {
            let _ = writeln!(self.stdin, "{}", json!({"cmd": "quit"}));
            let _ = self.child.wait();
        } else {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn protocol_error(message: String) -> BridgeError {
    BridgeError {
        kind: "protocol".to_string(),
        message,
    }
}

impl Bridge {
    /// 預設的橋接程式位置。
    pub fn default_path() -> PathBuf {
        std::env::var_os("EMBERSTONE_ROSETTA_BRIDGE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../rosetta-bridge/build/rosetta-bridge.exe")
            })
    }

    /// 啟動預設位置（環境變數 EMBERSTONE_ROSETTA_BRIDGE，否則 tools/rosetta-bridge/build/rosetta-bridge.exe）的橋接程式。
    pub fn spawn_default() -> Result<Self, String> {
        Self::spawn(Self::default_path())
    }

    pub fn spawn(path: PathBuf) -> Result<Self, String> {
        let mut child = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("啟動不了 {}：{e}", path.display()))?;
        let stdin = child.stdin.take().ok_or("拿不到橋接程式的 stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("拿不到橋接程式的 stdout")?);
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in stdout.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            lines,
            alive: true,
            started: false,
        })
    }

    /// 行程是否還能用（逾時或提前結束後為 false，要重新啟動）。
    pub fn is_alive(&self) -> bool {
        self.alive
    }

    /// 送出一個請求，回傳成功回應（`ok: true`）的內容。
    fn request(&mut self, request: Value) -> Result<Value, BridgeError> {
        if !self.alive {
            return Err(protocol_error("橋接程式已經結束".to_string()));
        }
        if let Err(e) = writeln!(self.stdin, "{request}").and_then(|_| self.stdin.flush()) {
            self.alive = false;
            return Err(protocol_error(format!("寫入橋接程式失敗：{e}")));
        }
        let timeout = if self.started {
            RESPONSE_TIMEOUT
        } else {
            STARTUP_TIMEOUT
        };
        let line = match self.lines.recv_timeout(timeout) {
            Ok(l) => l,
            Err(RecvTimeoutError::Timeout) => {
                self.alive = false;
                let _ = self.child.kill();
                return Err(BridgeError {
                    kind: "timeout".to_string(),
                    message: format!("{} 秒內沒有回應，已結束橋接程式", timeout.as_secs()),
                });
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.alive = false;
                let status = self
                    .child
                    .wait()
                    .map(|s| format!("{s}"))
                    .unwrap_or_else(|e| e.to_string());
                return Err(protocol_error(format!("橋接程式提前結束（{status}）")));
            }
        };
        self.started = true;
        let response: Value = serde_json::from_str(&line)
            .map_err(|e| protocol_error(format!("回應不是 JSON：{e}：{line}")))?;
        if response.get("ok") == Some(&Value::Bool(true)) {
            Ok(response)
        } else {
            let err = response
                .get("error")
                .cloned()
                .ok_or_else(|| protocol_error(format!("回應格式錯誤：{line}")))?;
            Err(serde_json::from_value(err)
                .map_err(|e| protocol_error(format!("錯誤格式不符：{e}：{line}")))?)
        }
    }

    fn field<T: DeserializeOwned>(response: Value, key: &str) -> Result<T, BridgeError> {
        let v = response
            .get(key)
            .cloned()
            .ok_or_else(|| protocol_error(format!("回應缺少 {key}")))?;
        serde_json::from_value(v).map_err(|e| protocol_error(format!("{key} 格式不符：{e}")))
    }

    pub fn version(&mut self) -> Result<Version, BridgeError> {
        let r = self.request(json!({"cmd": "version"}))?;
        serde_json::from_value(r).map_err(|e| protocol_error(format!("版本格式不符：{e}")))
    }

    pub fn new_game(&mut self, setup: &GameSetup) -> Result<Snapshot, BridgeError> {
        let r = self.request(json!({
            "cmd": "new",
            "p1": setup.p1,
            "p2": setup.p2,
            "rng": setup.rng,
        }))?;
        Self::field(r, "snapshot")
    }

    pub fn options(&mut self) -> Result<Vec<Action>, BridgeError> {
        let r = self.request(json!({"cmd": "options"}))?;
        Self::field(r, "options")
    }

    pub fn act(&mut self, action: &Action) -> Result<Snapshot, BridgeError> {
        let r = self.request(json!({"cmd": "act", "action": action}))?;
        Self::field(r, "snapshot")
    }

    pub fn snapshot(&mut self) -> Result<Snapshot, BridgeError> {
        let r = self.request(json!({"cmd": "snapshot"}))?;
        Self::field(r, "snapshot")
    }

    pub fn events(&mut self) -> Result<Vec<RngEvent>, BridgeError> {
        let r = self.request(json!({"cmd": "events"}))?;
        Self::field(r, "events")
    }
}
