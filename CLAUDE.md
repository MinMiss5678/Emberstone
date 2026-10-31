# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 專案概要

Emberstone：用 Rust 寫的爐石標準模式對戰引擎，用來練習 AI 工程（規則、搜尋、LLM、學習式 AI），產出對戰 AI、牌組勝率矩陣與自動構築牌組。完整規劃、卡片批次、階段與驗收標準見 `docs/PLAN.md`——動手前先看目前進行到哪個階段。

目前只有骨架（各 crate 只有模組說明註解）。巫妖王與單人副本已移出範圍；前身 Python 模擬器（lichsim）的紀錄在 `docs/背景研究.md`，只供參考（搜尋 AI、速度數據仍有用）。

## 常用指令

```bash
cargo build --release
cargo test                              # 全部
cargo test -p emberstone-core           # 單一 crate
cargo test -p emberstone-core --test combat  # 單一整合測試檔（tests/combat.rs）
cargo test -p emberstone-core 測試名稱片段     # 單一測試
cargo clippy --workspace --all-targets
cargo fmt
cargo run --release -p emberstone-sim   # 執行檔名為 emberstone
```

Rust 版本由 `rust-toolchain.toml` 固定為 1.99.1（Windows 原生 MSVC，用 RustRover 開發；效能分析在 WSL 做）。edition 2024。

## 架構

Cargo workspace，**相依只能單向**：data → core → cards → ai → sim／py。不要反向引用（例如 core 不能知道具體卡片效果，效果由 cards 註冊進 core）。

- `emberstone-data`：卡表由產生工具從 HearthstoneJSON `tw_all.json`（放在 `data/`）產生並**納入版本控制**，編譯時不依賴外部路徑。數值不手動校正，要改就重新產生。
- `emberstone-core`：對局狀態、規則、事件佇列、觸發、光環、附魔、亂數、合法行動列舉；也負責穩定的行動編號、狀態特徵（給學習用）與文字描述（給 LLM 用）。只認得卡片數值與關鍵字。
- `emberstone-cards`：卡片效果。效果盡量寫成資料（效果列舉組合），特殊卡才寫函式。
- `emberstone-ai`：隨機、規則 AI、搜尋 AI（決定化＋MCTS）、學習式評估的推論端，只透過 core 的狀態與合法行動決策。
- `emberstone-sim`：對戰模擬、牌組勝率矩陣、速度量測、繁體中文報告。
- `emberstone-py`（之後建立）：PyO3 的 Gym 風格環境；訓練與 LLM 代理在 Python 端。
- `tools/`：和 RosettaStone（C++，WSL 編譯）的差分測試工具。加入程式碼時要同步更新 `.claude/tdd.json` 的 globs，否則不受稽核。

### 核心設計約束
- 實體存在 `Vec<Entity>`、以編號互相參照；手牌／牌庫／戰場是編號清單。整份狀態 `#[derive(Clone)]`——複製速度是搜尋 AI 的命脈，避免引入讓複製變慢的結構（`Rc`、大量小陣列配置等）。
- 每局一個可複製的亂數產生器，同種子必須可重現；亂數來源要能換成「外部指定」，差分測試與實戰紀錄重播都靠它。搜尋 AI 用決定化（重抽對手的隱藏資訊）避免偷看；每位玩家只能取得自己視角的觀測。
- 事件時序照爐石階段規則：出牌 → 戰吼 → 召喚後觸發 → 死亡階段 → 亡語依上場順序。這是最容易寫錯的地方。
- 未實作的隨機生成卡退化成「只有數值和關鍵字」，並在報告統計發生次數。
- 外部套件盡量少：serde／serde_json 只給卡表產生工具；sim 可用 clap、mimalloc；py 用 pyo3；core、cards、ai 不用外部套件。
- 速度驗收門檻：一局 ≤ 0.5 ms、複製 ≤ 5 μs（批次 A 卡片組成的牌組、Windows 量測）。

## 測試

- 規格來源：規則 case 表（使用者核可）、和 RosettaStone 的**差分測試**（卡片批次 A、B）、實戰紀錄（Power.log）**重播**（批次 C、D）。差分測試不一致時不預設任一方正確，逐案列給使用者裁定。
- 整合測試放各 crate 的 `tests/`。
- 單元測試寫在獨立檔：`foo.rs` 內 `#[cfg(test)] mod tests;`，測試放 `foo/tests.rs`。稽核 hook 靠路徑分辨測試與實作，寫在實作檔內的測試會被當成實作。
- 每個階段走 test-first：case 表核可 → 測試 → 紅燈 → 實作 → 綠燈。稽核已啟用（`.claude/tdd.json`）。

## 編輯與測試路徑（hook 強制）

- `.rs` 一律透過 **RustRover MCP**（`mcp__rustrover__create_new_file`／`apply_patch`）編輯；Edit/Write 與 Bash 寫 `.rs` 會被擋。
- `cargo test` 一律透過 `mcp__rustrover__execute_terminal_command` 執行（Bash 跑會被擋）。不要加 `2>&1`（會少掉 Doc-tests）。
- 這個工具偶爾掉 stderr 行或夾雜 NUL（RustRover 端問題），hook 會用 exit code 交叉檢查；結果判成 invalid 就重跑。
- 跑測試時**不要用 `maxLinesCount`／`truncateMode` 截斷輸出**（用 `truncateMode: NONE`）：截掉 `test result:` 行會讓 hook 把綠燈判成 invalid。
- `data/tw_all.json` 不在 git 裡；測試需要它，缺檔時照 `data/README.md` 下載固定版本。
- `cargo build`／`clippy` 等不改檔的指令可用 Bash。`cargo fmt` 會改寫 `.rs` 而不經 hook 記錄，Stop 時會被列為未記錄變更，執行前先跟使用者確認。
- Bash 指令裡同時出現「寫檔的腳本」和 `.rs` 路徑字樣時，hook 會保守地擋下（即使只是改 `.md`）；改文件用 Edit/Write 工具。

## 慣例

文件、註解、報告一律繁體中文（台灣用語），使用台版卡名。
