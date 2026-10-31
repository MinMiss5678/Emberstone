# Emberstone

用 Rust 寫的爐石標準模式對戰引擎，用來練習 AI 工程：對戰 AI（規則、搜尋、LLM、學習式）、牌組勝率矩陣、自動構築牌組。規劃見 [docs/PLAN.md](docs/PLAN.md)。

## 結構

| crate | 類型 | 內容 |
|---|---|---|
| `emberstone-data` | 函式庫 | 卡片資料（數值、關鍵字、台版卡名）與產生卡表的工具 |
| `emberstone-core` | 函式庫 | 對局狀態與規則、事件、觸發、光環、亂數、合法行動 |
| `emberstone-cards` | 函式庫 | 卡片效果 |
| `emberstone-ai` | 函式庫 | 規則 AI、搜尋 AI、學習式評估 |
| `emberstone-sim` | 執行檔（`emberstone`） | 對戰模擬、牌組勝率矩陣、速度量測與報告 |

相依只有一個方向：data → core → cards → ai → sim。

## 常用指令

```bash
cargo build --release
cargo test
cargo run --release -p emberstone-sim
```

Rust 版本由 `rust-toolchain.toml` 固定（1.99.1）。
