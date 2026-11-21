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
- **`patches/`**（`build.bat` 依檔名順序自動套用目錄裡全部的 `.patch`，已套用的略過；新增 patch 只要新增檔案，不必改 `build.bat`。改同一段程式碼的修正要合併成一個 patch，否則「已套用」的檢查會認不出前一個）：
  - `0001-build-options.patch`：略過 Python 擴充與單元測試，加入 shim 的 include 路徑
  - `0002-rng-call-sites.patch`：3 處直接拿亂數引擎洗牌的地方改走 `Random::shuffle`，才記錄得到
  - `0003-fix-adjacent-aura-use-after-free.patch`：修 RosettaStone 的記憶體錯誤（用 AddressSanitizer 找到）：相鄰光環被移除後，戰場仍留著已釋放的指標
  - `0004-coin-temp-mana.patch`：RosettaStone 的幸運幣（`GAME_005`）沒有定義效果，打出去什麼都不做，補上「本回合 +1 暫時法力」；並照現行規則（29.0 版，2024 年 3 月）讓暫時法力可以超過 10（例如 11/10），RosettaStone 原本限制在 10
  - `0005-totemic-call.patch`：召喚圖騰照現行規則：候選是灼熱、石爪、風懲、治療（29.0 版換回風懲圖騰）；優先召喚場上沒有的，4 種都在時從四種中隨機（27.0 版起可以使用）
  - `0006-deathrattle-summon-position.patch`：死亡之聲召喚的位置照實戰紀錄驗證的規則：同一批死亡的手下記下移除時的位置，每召喚一隻，位置在插入點或右邊的其他死亡手下往右推一格（RosettaStone 原本不推）
  - `0007-abusive-sergeant-target.patch`：嚴厲的士官在沒有其他手下時也能打出、戰吼不發動（case C2；RosettaStone 原本要求一定要有目標）
  - `0008-weapon-only-on-own-turn.patch`：武器只在擁有者的回合計入英雄攻擊力，所以英雄在對手回合被攻擊時不會用武器反擊（使用者裁定 F5）
  - `0009-poisonous-not-hero.patch`：致命劇毒只消滅手下，英雄拿武器攻擊劇毒手下不會死（第 1 階段 case D5；RosettaStone 原本連英雄一起消滅、直接結束對局）
  - `0010-aura-stat-fixes.patch`：被動加成的兩個問題：
    - 加攻擊／生命的附魔（嚴厲的士官、野性印記等）以「目前數值」為基準加上去，會把被動加成算進基礎值，多算一次；改成扣掉被動加成再加（case E2、E5、E6）
    - 移除生命被動加成時，已經死亡的手下又被判定死亡一次，死亡之聲結算兩次；改成只調整場上的手下（case D1、E4）
  - `0011-secret-silence-bomber-fixes.patch`：
    - 寒冰護體（`CORE_EX1_289`）只在對手攻擊時觸發（case I2；RosettaStone 原本自己的手下打對手英雄也會觸發）
    - 沉默後目前生命只在超過新上限時才降（case H2；RosettaStone 原本保留傷害，受傷的手下會變成 0 以下的生命卻留在場上）
    - 「隨機分配給其他角色」（瘋狂炸彈客）不再挑已經打到 0 生命的英雄（case F4a）
  - `0012-bloodhoof-brave-enrage.patch`：血蹄勇戰者（`CORE_OG_218`）改用受傷加成的被動效果，回滿生命時 +3 攻擊會失效（case E7；RosettaStone 原本只在受傷時加上、不會移除）
  - `0013-secret-then-death-phase.patch`：攻擊時的秘密結算完先進入死亡階段（含死亡之聲），攻擊者或目標因此死亡就取消攻擊；秘密打死任一方英雄時遊戲直接結束，不再進行攻擊（case I3b、I3c；RosettaStone 原本先戰鬥才處理死亡，可能變成平手）
  - `0014-returned-minion-reset.patch`：手下離場（例如被冰凍陷阱送回手牌）時完整重設：
    - 沉默跟著消失，重新打出後死亡之聲恢復（case H1）
    - 本回合加成不再於回合結束時從手牌上的卡扣掉（case E6；RosettaStone 原本會讓重新打出的手下攻擊力變 0）
    - 受傷加成（血蹄勇戰者）在沉默或離場時不再多扣一次攻擊力（case E7、H1）
  - `0015-random-split-no-target.patch`：「隨機分配給其他角色」沒有可選的角色時，該發不造成傷害（case F4b；RosettaStone 原本會打在上一發選中的目標上）
  - `0016-explosive-runes-divine-shield.patch`：爆炸符文打到聖盾手下時，聖盾只吸收等於手下生命的傷害，其餘打英雄（case I4a，依 Hearthstone Wiki；RosettaStone 原本只在手下死亡時才溢出）；6 點加上法術傷害、溢出照總量算（第 3a 階段 A1，實測 R10；RosettaStone 原本用 6 − 生命 再加法術傷害，生命正好 6 時少算溢出）
  - （`0017` 已撤掉：原本依 Wiki 的解讀讓戰吼後不先處理死亡；使用者實測證明戰吼後會先進入死亡階段，RosettaStone 原本的做法才對，case C1、C6）
  - `0018-freeze-thaw-attacks-left.patch`：在自己回合被凍結、攻擊次數還沒用完（例如風怒只攻擊一次）的手下，本回合結束就解凍（第 1 階段 case D9a，依 Wiki Freeze）；剛上場的衝刺手下被凍結時，看回合結束當下對手有沒有手下，沒有就到下一個自己的回合結束才解凍（第 3b 階段 B4b，實測 R11、R15、R16）
  - `0019-zero-cost-with-overload.patch`：超載超過水晶上限讓可用法力變成負的時候，0 費的牌照樣能打出（第 1 階段 case B3a，依 Wiki Mana）
  - `0022-silence-removes-poisonous.patch`：沉默也移除致命劇毒與凍結效果（case H1；RosettaStone 原本漏了這兩個關鍵字，被沉默的劇毒手下仍會消滅攻擊者、被沉默的水元素仍會凍結）；剛上場的衝刺手下被沉默後當回合不能攻擊（case H1、第 1 階段 C3；RosettaStone 原本沉默後仍可攻擊手下）
  - `0021-aura-update-second-pass.patch`：更新被動加成時多跑一輪，讓「受傷時」的加成看到其他加成移除後的生命（例如暴風城勇士在戰鬥中死亡，剛好回滿的血蹄勇戰者立刻失去 +3 攻擊；case E4、E7）
  - `0020-transform-keeps-play-order.patch`：變形後的手下保留原本的上場順序（case F1b，使用者實測；Wiki 寫「排到最後面」與實測不符，RosettaStone 原本照 Wiki）
  - `0023-transform-removes-trigger.patch`：變形時移除原本手下的觸發效果（case F1；RosettaStone 原本留著，例如被變成綿羊的治療圖騰每回合結束仍會治療）
  - `0024-silenced-charge-exhausted.patch`：剛上場的衝鋒手下也標記為「剛上場」（衝鋒讓它照樣能攻擊），這回合被沉默失去衝鋒後就不能攻擊（case H1；RosettaStone 原本沉默後仍可攻擊）  - `0025-aoe-hits-mortally-wounded.patch`：全場傷害（全部角色／全部手下／敵方／友方）也打到生命已歸零、還沒移出戰場的手下，例如同一批的第二個死亡之聲照樣打到第一個打死的手下，白沫狂戰士也照算（case D6，使用者實測；RosettaStone 原本列戰場時一律排除，隨機目標維持排除）
  - `0026-fogsail-target-needs-weapon.patch`：霧帆掠劫者（`CS3_022`）只有裝備武器時才要選戰吼目標（第 3a 階段 B5；RosettaStone 原本沒有武器也要選，新增 `REQ_TARGET_IF_AVAILABLE_AND_WEAPON_EQUIPPED`）
  - `0027-alchemist-swap-current-health.patch`：瘋狂的鍊金師對調後清掉傷害（目前生命成為新的上限），本回合的加成併入對調後的數值、回合結束不再扣；焦油蠕行者「對手回合 +2」併入後再套用一次；激怒的加成併入後不再扣掉（第 3a 階段 C7、H1；RosettaStone 原本保留傷害，受傷的手下會變成 0 以下，+2 也不再套用，激怒中的葛羅瑪許對調後再被扣 6 攻擊）
  - `0028-wailing-vapor-friendly-only.patch`：哀嚎水氣（`WC_042`）只在自己打出元素後觸發（第 3a 階段 D4；RosettaStone 原本對手打出也觸發）
  - `0029-pressure-plate-after-spell.patch`：踏板陷阱（`ULD_152`）在對手的法術結算完才觸發（第 3a 階段 E2；RosettaStone 原本在施放時、結算前觸發）；對手場上只有休眠中的手下時不觸發（第 3b 階段 H3，實測 R19）
  - `0030-adaptive-effect-silence.patch`：「依情況變化」的數值效果（焦油蠕行者）被沉默時立刻扣回已加的值（第 3a 階段 D2；RosettaStone 原本在沉默重設數值後才扣，攻擊力變成 0）；離場時數值已重設，不再扣（否則回到手牌再打出會變成 0 攻）
  - `0031-forced-attack-not-exhausted.patch`：效果要求的攻擊（伊利達瑞審判官跟著英雄攻擊、戰槌挑戰者互打）不會用掉攻擊次數（第 3a 階段 D5，使用者實測 R4；RosettaStone 原本攻擊後就不能再攻擊）
  - `0032-warmaul-challenger-duel.patch`：戰槌挑戰者（`BT_120`）沒有目標時戰吼不發動；互打最多 30 次，任一方死亡或離場就停止（第 3a 階段 D6；RosettaStone 原本沒有目標會內部錯誤，一方離場會無窮迴圈）
  - `0033-boomkin-damage-not-spell.patch`：平衡梟獸二選一的「造成 4 點傷害」（`ONY_018t2`）是手下效果，不加法術傷害（第 3a 階段 A1；RosettaStone 原本當成法術傷害）
  - `0035-discover-pool-override.patch`：發現（`Cards::GetDiscoverCards`）的候選可由橋接程式的 `random_pool` 指令指定，依清單順序、只留玩家職業與中立的卡；差分測試用來讓兩個引擎的候選相同（第 3 階段 Q2）
  - `0036-discover-defers-rest-of-play.patch`：法術的發現出現選擇畫面時，之後的死亡階段、「施放法術後」與「打出卡牌後」的觸發都等選完才結算；已觸發的效果（白沫狂戰士 +1）在選擇畫面出現前就生效（第 3b 階段 C3、C3b、H2，實測 R5、R13、R18；RosettaStone 原本選擇畫面出現時就全部結算完）
  - `0037-dormant-untouchable.patch`：休眠中的手下（實體上的 `UNTOUCHABLE`，進場前就設好）不能被指定、不能被攻擊、不受全體／隨機效果與被動加成影響；以休眠上場不觸發「召喚」「打出手下後」「召喚手下後」（含爆炸符文）；送回手牌再打出時重新倒數；甦醒時補上被動加成（含相鄰），有衝刺的當回合照樣能攻擊手下（第 3b 階段 B2–B5；RosettaStone 原本只看卡片上的標記，休眠中照樣被指定、被打、吃加成、觸發秘密，再打出的不會甦醒，甦醒後一律不能攻擊）
  - `0034-riftcleaver-target-if-available.patch`：惡魔斬隙者（`ULD_165`）沒有手下可以摧毀時照樣能打出，戰吼不發動（第 3a 階段 B4；RosettaStone 原本必須有目標才能打出）
  - `0038-kragwa-random-order.patch`：『巨蛙』奎格瓦放回手牌的法術順序隨機（第 3b 階段 D7，實測 R8；RosettaStone 原本照打出順序）
  - `0039-casts-when-drawn-draw-trigger-order.patch`：抽到抽中時施放的牌時，補抽的那張先觸發「抽到牌」，再輪到抽中時施放的牌（第 3b 階段 D6，實測 R7：阿拉巴斯特先複製補抽的牌；RosettaStone 原本順序相反）
  - `0040-eye-beam-outcast-cost-floor.patch`：魔眼光束流放時「設為 (1)」後再套其他減費、最低 0（第 3b 階段 A6；RosettaStone 原本被視界術減到 0 以下時反而變成 1）
  - `0041-choice-defers-game-over.patch`：選擇畫面出現時，英雄死亡的勝負判定也等選完才做（第 3b 階段 C3b：死亡階段等選完才進行；RosettaStone 原本行動結束就判定）
  - `0042-draw-keeps-stack.patch`：抽牌時觸發的效果不會把自己的牌留在堆疊上：對手有阿拉巴斯特時，視界術的減費只給抽到的那張（第 3b 階段 D 的複製規則、D4；RosettaStone 原本把 −3 給了阿拉巴斯特的複製品、抽到的那張沒減）

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
- `BT_120`（戰槌挑戰者，「戰鬥到死」）：有時無窮迴圈（`difftest` 20 秒逾時後重啟橋接程式）或內部錯誤。差分測試把這些局記為「錯誤」，不算不一致。
