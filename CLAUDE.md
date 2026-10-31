# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 專案概要

Emberstone：用 Rust 寫的爐石模擬引擎，要取代 Fireplace（Python）版巫妖王模擬器（`D:\HearthstoneCollection\sim\lichsim\`），速度提升 1000 倍以上，讓搜尋 AI 跑得動。完整規劃、階段與驗收標準見 `PLAN.md`——動手前先看目前進行到哪個階段。

目前只有骨架（各 crate 只有模組說明註解）。

## 常用指令

```bash
cargo build --release
cargo test                              # 全部
cargo test -p emberstone-core           # 單一 crate
cargo test -p emberstone-cards --test boss   # 單一整合測試檔（tests/boss.rs）
cargo test -p emberstone-core 測試名稱片段     # 單一測試
cargo clippy --workspace --all-targets
cargo fmt
cargo run --release -p emberstone-sim   # 執行檔名為 emberstone
```

Rust 版本由 `rust-toolchain.toml` 固定為 1.99.1（Windows 原生 MSVC，用 RustRover 開發；效能分析在 WSL 做）。edition 2024。

## 架構

Cargo workspace，五個 crate，**相依只能單向**：data → core → cards → ai → sim。不要反向引用（例如 core 不能知道具體卡片效果，效果由 cards 註冊進 core）。

- `emberstone-data`：卡表由產生工具從 HearthstoneJSON `tw_all.json`（放在 `data/`）產生並**納入版本控制**，編譯時不依賴外部路徑。數值不手動校正，要改就重新產生。
- `emberstone-core`：對局狀態、規則、事件佇列、觸發、光環、附魔、亂數、合法行動列舉。只認得卡片數值與關鍵字。
- `emberstone-cards`：卡片效果與巫妖王等頭目卡。效果盡量寫成資料（效果列舉組合），特殊卡才寫函式。
- `emberstone-ai`：巫妖王 AI、規則 AI、搜尋 AI（決定化＋MCTS），只透過 core 的狀態與合法行動決策。
- `emberstone-sim`：勝率模擬命令列、繁體中文報告、速度量測。

### 核心設計約束
- 實體存在 `Vec<Entity>`、以編號互相參照；手牌／牌庫／戰場是編號清單。整份狀態 `#[derive(Clone)]`——複製速度是搜尋 AI 的命脈，避免引入讓複製變慢的結構（`Rc`、大量小陣列配置等）。
- 每局一個可複製的亂數產生器，同種子必須可重現。搜尋 AI 用決定化（重洗對手手牌與牌庫）避免偷看。
- 事件時序照爐石階段規則：出牌 → 戰吼 → 召喚後觸發 → 死亡階段 → 亡語依上場順序。這是最容易寫錯的地方。
- 未實作的隨機生成卡退化成「只有數值和關鍵字」，並在報告統計發生次數。
- 外部套件盡量少：serde／serde_json 只給卡表產生工具；sim 可用 clap、mimalloc；core、cards、ai 不用外部套件。
- 速度驗收門檻：一局 ≤ 0.5 ms、複製 ≤ 5 μs（巫妖王牌組、Windows 量測）。

## 測試

- 小型單元測試放各 crate 的 `src`；從 Python 搬來的 300 個測試放各 crate 的 `tests/`，檔名對照 Python（例如 `emberstone-cards/tests/boss.rs` ↔ `test_boss.py`）。
- 搬移時**期望值照抄 Python 測試，不迎合實作**——這些測試就是規格。
- 單元測試寫在獨立檔：`foo.rs` 內 `#[cfg(test)] mod tests;`，測試放 `foo/tests.rs`。稽核 hook 靠路徑分辨測試與實作，寫在實作檔內的測試會被當成實作。
- 每個階段走 test-first：case 表核可 → 測試 → 紅燈 → 實作 → 綠燈。稽核已啟用（`.claude/tdd.json`）。

## 編輯與測試路徑（hook 強制）

- `.rs` 一律透過 **RustRover MCP**（`mcp__rustrover__create_new_file`／`apply_patch`）編輯；Edit/Write 與 Bash 寫 `.rs` 會被擋。
- `cargo test` 一律透過 `mcp__rustrover__execute_terminal_command` 執行（Bash 跑會被擋）。不要加 `2>&1`（會少掉 Doc-tests）。
- 這個工具偶爾掉 stderr 行或夾雜 NUL（RustRover 端問題），hook 會用 exit code 交叉檢查；結果判成 invalid 就重跑。
- `cargo build`／`clippy` 等不改檔的指令可用 Bash。`cargo fmt` 會改寫 `.rs` 而不經 hook 記錄，Stop 時會被列為未記錄變更，執行前先跟使用者確認。

## 慣例

文件、註解、報告一律繁體中文（台灣用語），使用台版卡名。
