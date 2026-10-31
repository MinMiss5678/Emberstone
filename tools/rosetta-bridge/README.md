# rosetta-bridge

差分測試用的 RosettaStone 橋接程式（C++）。由 `tools/difftest`（Rust）啟動並以 JSON lines 溝通。

## 編譯

```bat
tools\rosetta-bridge\build.bat
```

需要：
- RosettaStone 原始碼，固定在 commit `1690208`（`16902087454f8055c991474774922fd39f7d0875`），放在 `%ROSETTA_ROOT%\RosettaStone`
- vcpkg（`%VCPKG_ROOT%`，預設 `%ROSETTA_ROOT%\vcpkg`）、CMake、Ninja
- Visual Studio 2022 C++ 工具（`%VCVARS%`，未設定時用 vswhere 找）

輸出 `tools/rosetta-bridge/build/rosetta-bridge.exe`（不納入 git）。RosettaStone 函式庫以 `/GL` 編譯，連結很慢（約 5–10 分鐘）。

## 編碼規則

- 檔案一律 UTF-8、不加 BOM。
- **會被 RosettaStone include 的檔案（`shim/`）整個只用 ASCII，註解也一樣**：RosettaStone 不以 `/utf-8` 編譯，MSVC 會用主機的程式碼頁讀原始碼（本機 950、GitHub 主機 1252）。在 950 下，沒有 BOM 的 UTF-8 中文註解會被當成雙位元組字元，吃掉後面的字元，造成語法錯誤；在 1252 下，中文字串常值會編譯失敗。`bridge.cpp` 以 `/utf-8` 編譯，不受此限。

## 對 RosettaStone 的修改

不改上游原始碼的檔案內容，改用：
- **`shim/effolkronium/random.hpp`**：放在 include 路徑最前面，取代 RosettaStone 用的亂數函式庫。記錄模式記下每次抽亂數（範圍／浮點數／洗牌與結果），重播模式照序列回傳；不同步時不丟例外（RosettaStone 可能在 noexcept 的地方抽亂數），改由橋接程式在每個行動後檢查並回報 `rng_desync`。
- **`patches/`**（`build.bat` 會自動套用，已套用的略過）：
  - `0001-build-options.patch`：略過 Python 擴充與單元測試，加入 shim 的 include 路徑
  - `0002-rng-call-sites.patch`：3 處直接拿亂數引擎洗牌的地方改走 `Random::shuffle`，才記錄得到
  - `0003-fix-adjacent-aura-use-after-free.patch`：修 RosettaStone 的記憶體錯誤（用 AddressSanitizer 找到）：相鄰光環被移除後，戰場仍留著已釋放的指標
  - `0004-coin-temp-mana.patch`：RosettaStone 的幸運幣（`GAME_005`）沒有定義效果，打出去什麼都不做，補上「本回合 +1 暫時法力」；並照現行規則（29.0 版，2024 年 3 月）讓暫時法力可以超過 10（例如 11/10），RosettaStone 原本限制在 10

## 協定

stdin 每行一個請求，stdout 每行一個回應：`{"ok":true,...}` 或 `{"ok":false,"error":{"kind":...,"message":...}}`。

| 請求 | 回應 |
|---|---|
| `{"cmd":"version"}` | `rosetta_commit`、`patches` |
| `{"cmd":"new","p1":{"class","deck"},"p2":{...},"rng":{"mode":"record","seed"}\|{"mode":"replay","events"}}` | `snapshot` |
| `{"cmd":"options"}` | `options`（目前所有合法行動） |
| `{"cmd":"act","action":{...}}` | `snapshot`；不合法時 `illegal_action`（狀態不變） |
| `{"cmd":"snapshot"}` / `{"cmd":"events"}` | 目前快照／記錄的亂數事件 |
| `{"cmd":"quit"}` | 結束 |

格式定義見 `tools/difftest/src/{actions,snapshot,rng}.rs`。除錯時設定 `EMBERSTONE_BRIDGE_LOG=<檔案>` 會把每個請求寫進該檔，可餵回橋接程式重現問題。

## 已知的 RosettaStone 問題

- `GetValidAttackTargets` 沒排除「剛出場的衝刺手下打英雄」（執行時 `IsValidAttackTarget` 才擋下、什麼都不做）。橋接程式列合法行動時再用 `IsValidAttackTarget` 過濾。
- 英雄的攻擊力一律加上武器攻擊力，在對手回合被攻擊時也會用武器反擊；現行規則是武器只在自己的回合有作用。Emberstone 照現行規則，第 1 階段的差分卡池沒有武器，所以還沒影響。
- `BT_120`（戰槌挑戰者，「戰鬥到死」）：有時無窮迴圈（`difftest` 20 秒逾時後重啟橋接程式）或內部錯誤。差分測試把這些局記為「錯誤」，不算不一致。
