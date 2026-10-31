# Emberstone

用 Rust 寫的爐石模擬引擎，目標是取代 Fireplace 版的巫妖王模擬器，並讓搜尋 AI 真的跑得動。規劃見 [PLAN.md](PLAN.md)。

## 結構

| crate | 類型 | 內容 |
|---|---|---|
| `emberstone-data` | 函式庫 | 卡片資料（數值、關鍵字、台版卡名）與產生卡表的工具 |
| `emberstone-core` | 函式庫 | 對局狀態與規則、事件、觸發、光環、亂數、合法行動 |
| `emberstone-cards` | 函式庫 | 卡片效果、巫妖王等頭目卡 |
| `emberstone-ai` | 函式庫 | 巫妖王 AI、規則 AI、搜尋 AI |
| `emberstone-sim` | 執行檔（`emberstone`） | 勝率模擬命令列與報告 |

相依只有一個方向：data → core → cards → ai → sim。

## 常用指令

```bash
cargo build --release
cargo test
cargo run --release -p emberstone-sim
```

Rust 版本由 `rust-toolchain.toml` 固定（1.99.1）。
