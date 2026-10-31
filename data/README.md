# data

卡表產生工具（`emberstone-cardgen`）與差分測試工具（`tools/difftest`）的輸入與輸出。

| 檔案 | 內容 | 納入 git |
|---|---|---|
| `tw_all.json` | HearthstoneJSON 繁體中文完整卡表（含衍生物、附魔等非收藏卡），約 10 MB | **否**（依 `hsjson_build.txt` 下載） |
| `en_all.json` | 同一版本的英文完整卡表，只有重新產生 `batches.tsv` 時需要 | **否** |
| `hsjson_build.txt` | 第 1 行 build 號碼；第 2、3 行為 `tw_all.json`、`en_all.json` 的 SHA-1 | 是 |
| `standard_sets.txt` | 目前標準模式的系列清單，人工維護 | 是 |
| `batches.tsv` | 核心系列卡片的批次分類（A／B／C）與對應的 RosettaStone 卡號，由 `difftest batches` 產生 | 是 |

產生出來的 `crates/emberstone-data/src/generated/cards.rs` 納入 git，編譯不需要 `tw_all.json`；只有重新產生卡表和 `emberstone-data` 的測試需要它。

## 下載固定版本（測試、CI 用）

```bash
BUILD=$(sed -n 1p data/hsjson_build.txt)
curl -fL "https://api.hearthstonejson.com/v1/$BUILD/zhTW/cards.json" -o data/tw_all.json
curl -fL "https://api.hearthstonejson.com/v1/$BUILD/enUS/cards.json" -o data/en_all.json   # 需要時
sha1sum data/tw_all.json data/en_all.json   # 要和 hsjson_build.txt 第 2、3 行相同
```

## 更新到最新版

1. 下載 `https://api.hearthstonejson.com/v1/latest/zhTW/cards.json`（與 `enUS`）到 `data/`，並到 `https://api.hearthstonejson.com/v1/` 查最新的 build 號碼。
2. 更新 `hsjson_build.txt`（build 號碼、SHA-1）；必要時更新 `standard_sets.txt`。
3. `cargo run -p emberstone-cardgen --bin gen-cards` 重新產生卡表。
4. `cargo run -p emberstone-difftest --bin difftest -- batches --rosetta <RosettaStone 原始碼目錄>` 重新產生批次表。
5. 跑全部測試。
