# Emberstone

用 Rust 寫的爐石標準模式對戰引擎，用來練習 AI 工程：對戰 AI（規則、搜尋、LLM、學習式）、牌組勝率矩陣、自動構築牌組。規劃見 [docs/PLAN.md](docs/PLAN.md)。

## 結構

| crate | 類型 | 內容 |
|---|---|---|
| `emberstone-data` | 函式庫 | 卡片資料（數值、關鍵字、台版卡名）與查詢 |
| `emberstone-cardgen` | 執行檔（`gen-cards`） | 由 HearthstoneJSON 產生 `emberstone-data` 的卡表 |
| `emberstone-core` | 函式庫 | 對局狀態與規則、事件、觸發、光環、亂數、合法行動 |
| `emberstone-cards` | 函式庫 | 卡片效果 |
| `emberstone-ai` | 函式庫 | 規則 AI、搜尋 AI、學習式評估 |
| `emberstone-sim` | 執行檔（`emberstone`） | 對戰模擬、牌組勝率矩陣、速度量測與報告 |

相依只有一個方向：data → core → cards → ai → sim。

## 常用指令

```bash
cargo build --release
cargo test        # emberstone-data 的測試需要 data/tw_all.json，下載方式見 data/README.md
cargo run --release -p emberstone-sim
```

Rust 版本由 `rust-toolchain.toml` 固定（1.99.1）。

## 授權

本專案的程式碼以 [GNU Affero General Public License v3.0 或更新版本](LICENSE)（AGPL-3.0-or-later）授權。

卡片名稱、敘述等遊戲內容（包括 `crates/emberstone-data/src/generated/cards.rs` 裡的資料，來源為 [HearthstoneJSON](https://hearthstonejson.com/)）屬於 Blizzard Entertainment，不在上述授權範圍內。本專案是非官方的研究專案，與 Blizzard Entertainment 無關。
