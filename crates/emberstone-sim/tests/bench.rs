//! 第 1 階段 H1、H2：速度量測命令（只輸出數字、不設門檻）。

use std::process::Command;

fn number_after(text: &str, label: &str) -> f64 {
    let rest = text
        .split(label)
        .nth(1)
        .unwrap_or_else(|| panic!("輸出沒有「{label}」：{text}"));
    let num: String = rest
        .trim_start_matches(['：', ' '])
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    num.parse()
        .unwrap_or_else(|_| panic!("「{label}」後面不是數字：{text}"))
}

// H1：用卡池隨機組牌、隨機對局 N 局，輸出每局平均時間與第 10 回合複製一次的時間
#[test]
fn h1_bench_reports_game_and_clone_times() {
    let out = Command::new(env!("CARGO_BIN_EXE_emberstone"))
        .args(["bench", "--games", "20", "--seed", "1"])
        .output()
        .expect("執行 emberstone bench 失敗");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(number_after(&stdout, "對局數") as usize, 20);
    assert!(number_after(&stdout, "每局平均（ms）") > 0.0, "{stdout}");
    assert!(
        number_after(&stdout, "第 10 回合複製一次（μs）") > 0.0,
        "{stdout}"
    );
}

// H1：同一個種子，對局結果（勝負統計）相同
#[test]
fn h1_bench_is_reproducible() {
    let run = || {
        let out = Command::new(env!("CARGO_BIN_EXE_emberstone"))
            .args(["bench", "--games", "10", "--seed", "7"])
            .output()
            .expect("執行 emberstone bench 失敗");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        stdout
            .lines()
            .find(|l| l.starts_with("勝負"))
            .unwrap_or_else(|| panic!("輸出沒有勝負統計：{stdout}"))
            .to_string()
    };
    assert_eq!(run(), run());
}

// 參數錯誤回報用法、結束碼非 0
#[test]
fn h1_bench_rejects_bad_arguments() {
    let out = Command::new(env!("CARGO_BIN_EXE_emberstone"))
        .args(["bench", "--games", "abc"])
        .output()
        .expect("執行 emberstone 失敗");
    assert!(!out.status.success());
}
