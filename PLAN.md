# Emberstone：Rust 爐石引擎規劃

狀態：骨架已建立（2026-10-31）。速度試驗見 `D:\HearthstoneCollection\rustspike\`（26 張卡）。

## 1. 目標與範圍

**取代 Fireplace 版模擬器（`D:\HearthstoneCollection\sim\lichsim\`）**，保留現有功能，速度提升 1000 倍以上，讓搜尋 AI 真的能用。

- 第一版範圍：巫妖王（頭目機制、巫妖王 AI、各職業開場法術）＋我們用過的牌組（薩滿 v6/v7、魚人、獲勝的薩滿／獵人／法師牌組）＋地城探險已有的部分。
- 現有 Python 模擬器的 300 個測試（23 個檔）就是規格：全部搬過來、在 Rust 上通過，才算「功能對等」。
- 不做：英雄戰場、競技場選牌、天梯全卡池（之後有需要再擴充）。

## 2. 為什麼可行（試驗數據，2026-10-31）

條件：基本法師對基本戰士、完全隨機出牌，各引擎平均約 32 回合。

| | Fireplace | SabberStone | RosettaStone | Rust 試驗 |
|---|---|---|---|---|
| 一局隨機對戰 | 196 ms | 2.54 ms | 1.98 ms | 0.041–0.045 ms（WSL 0.030） |
| 第 10 回合複製／分支 | 26,572 μs | 28.4 μs | 436 μs（從頭重播） | 0.29–0.85 μs（WSL 0.11） |

以上除註明 WSL 外，都是 Windows 原生。Rust 在 Windows 複製較慢，主因是記憶體配置；正式版可改用較快的配置器或減少小陣列配置。

完整引擎加上事件、光環等通用機制後預估慢 2–5 倍，仍比 SabberStone 快約 10 倍。驗收門檻：**一局 ≤ 0.5 ms、複製 ≤ 5 μs**（以巫妖王牌組、Windows 量測）。

## 3. 架構

Cargo workspace，五個 crate，相依只有一個方向：data → core → cards → ai → sim。

| crate | 內容 |
|---|---|
| `emberstone-data` | 卡片資料：由 HearthstoneJSON 現行版 `tw_all.json`（複製到 `data/`）產生 Rust 表格並納入版本控制，含台版卡名。不用手動校正數值 |
| `emberstone-core` | 對局狀態、規則、事件佇列、觸發、光環、附魔、亂數、合法行動列舉、複製 |
| `emberstone-cards` | 每張卡的效果（戰吼、亡語、觸發、光環），以及巫妖王等頭目卡 |
| `emberstone-ai` | 巫妖王 AI、我方規則 AI、搜尋 AI（MCTS／決定化） |
| `emberstone-sim` | 勝率模擬命令列工具（執行檔 `emberstone`）、報告（繁體中文）、速度量測 |

選配：`emberstone-py`（PyO3 包成 Python 模組，沿用現有分析腳本）。

外部套件盡量少：serde／serde_json 只給卡表產生工具用；sim 用 clap（命令列）、mimalloc（選用）。core、cards、ai 不用外部套件。

測試：小型單元測試放各 crate 的 `src`；搬過來的 300 個 Python 測試放各 crate 的 `tests\`，檔名對照 Python（例如 `emberstone-cards\tests\boss.rs` ↔ `test_boss.py`）。

### 核心設計
- **狀態**：實體存在 `Vec<Entity>`，用編號互相參照；手牌、牌庫、戰場是編號清單。整份狀態 `#[derive(Clone)]`，複製不用特別處理。
- **事件與時序**：照爐石的「階段」規則處理（出牌 → 戰吼 → 召喚後觸發 → 死亡階段 → 亡語依上場順序）。這是最容易寫錯的地方，第 2 階段專門處理。
- **卡片效果**：以效果列舉（傷害、召喚、增益、摧毀、變形、抽牌、凍結……）組合；特殊卡用函式。
- **亂數**：每局一個可複製的亂數產生器，同種子結果可重現；搜尋 AI 用「決定化」（重洗對手手牌與牌庫）避免偷看。
- **隨機生成的卡**（進化、退化、發現、骷髏護衛送的卡）：有實作就照實作；沒實作的手下退化成「只有數值和關鍵字」，並在報告裡統計發生次數，讓誤差看得見。

## 4. 階段（每個階段都照 test-first：case 表核可 → 測試 → 紅燈 → 實作 → 綠燈）

| 階段 | 內容 | 搬過來的現有測試 | 粗估 |
|---|---|---|---|
| 0 工具 | `emberstone-data` 產生卡表、cargo test 環境、速度量測指令 | 新寫 | 1 次對話 |
| 1 基本規則 | 回合、法力、超載、出牌、攻擊、嘲諷、聖盾、劇毒、風怒、衝鋒／突襲、凍結、武器、護甲、疲勞、手牌／戰場上限、死亡處理 | test_engine 等 | 2–3 次 |
| 2 事件框架 | 戰吼、亡語、回合開始／結束觸發、光環、單回合增益、變形、衍生物、二選一、發現 | test_choose_one、test_deathrattle_shaman 部分 | 3–4 次 |
| 3 卡片 | 巫妖王牌組 17 種、頭目卡、各職業開場法術、我們牌組的卡（約 150 張） | test_boss、test_lk_deck、test_cards、test_lk_calib | 4–6 次 |
| 4 AI 與模擬器 | 巫妖王 AI、薩滿／魚人／通用規則 AI、換牌、命令列與報告 | test_lk_ai、test_my_ai、test_v7_ai、test_murloc_*、test_generic_ai、test_run 等 | 4–6 次 |
| 5 搜尋 AI | 決定化＋MCTS，善用複製速度 | test_search_* | 2–3 次 |
| 6 驗收 | 同牌組和 Python 版比勝率、和實戰血量曲線比對 | — | 1–2 次 |
| 7（選配） | 地城探險頭目、PyO3 | test_dungeon | 視需要 |

合計約 **17–25 次對話**。每個階段結束都能單獨使用，不必等全部完成。

## 5. 驗收方式
1. 300 個現有測試全部搬到 Rust 並通過（期望值照抄，不迎合實作）。
2. 同一副牌、各跑 1,000 局：Rust 版和 Python 版的勝率、平均回合、平均造成傷害，差距在統計誤差內。差太多就找規則差異。
3. 速度達到第 2 節的門檻。

## 6. 風險與對策
| 風險 | 對策 |
|---|---|
| 規則時序寫錯，勝率失真 | 第 2 階段專做事件框架；Fireplace 已有的 300 個測試當回歸檢查；第 6 階段雙引擎對照 |
| 隨機生成卡無法全部實作 | 退化成白板並統計次數；常見的再補實作 |
| 新擴充一直出卡 | 數值由 `emberstone-data` 自動產生；效果按需要補 |
| 維護 Rust 的成本 | 卡片效果盡量寫成資料（效果列舉），少寫自訂程式；文件與註解用繁體中文 |

## 7. 已決定
- 名稱：Emberstone；位置：`C:\Users\jerem\RustroverProjects\Emberstone`。
- 環境：Windows 原生（MSVC 工具鏈、Rust 1.99.1），用 RustRover 開發；效能分析在 WSL 做。
- 結構：如第 3 節。

## 8. 待決定
- 是否 `git init`（只在本機）。
- test-first 稽核：是否建 `tdd.json` 啟用 hook（需讓 hook 認得 `cargo test`）。
