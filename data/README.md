# data

卡表產生工具（`emberstone-cardgen`）的輸入。

| 檔案 | 內容 | 納入 git |
|---|---|---|
| `tw_all.json` | HearthstoneJSON 繁體中文完整卡表（含衍生物、附魔等非收藏卡），約 10 MB | **否**（依 `hsjson_build.txt` 下載） |
| `hsjson_build.txt` | 目前卡表對應的 HearthstoneJSON 版本（build 號碼）與檔案 SHA-1 | 是 |
| `standard_sets.txt` | 目前標準模式的系列清單，人工維護 | 是 |

產生出來的 `crates/emberstone-data/src/generated/cards.rs` 納入 git，編譯不需要 `tw_all.json`；只有重新產生卡表和 `emberstone-data` 的測試需要它。

## 下載固定版本（測試、CI 用）

```bash
BUILD=$(sed -n 1p data/hsjson_build.txt)
curl -fL "https://api.hearthstonejson.com/v1/$BUILD/zhTW/cards.json" -o data/tw_all.json
sha1sum data/tw_all.json   # 要和 hsjson_build.txt 第 2 行相同
```

## 更新到最新版

1. 下載 `https://api.hearthstonejson.com/v1/latest/zhTW/cards.json` 到 `data/tw_all.json`，並到 `https://api.hearthstonejson.com/v1/` 查最新的 build 號碼。
2. 更新 `hsjson_build.txt`（build 號碼、SHA-1）；必要時更新 `standard_sets.txt`。
3. `cargo run -p emberstone-cardgen --bin gen-cards` 重新產生卡表，再跑全部測試。
