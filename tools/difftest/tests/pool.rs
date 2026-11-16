//! 第 1 階段 G1：純關鍵字手下卡池。

use std::collections::BTreeSet;

use emberstone_data::{CardType, Keyword, card_by_id};
use emberstone_difftest::pool::{KEYWORDS, keyword_pool, parse_pool};

const EN: &str = r#"[
  {"id": "V_001", "type": "MINION", "collectible": true, "cost": 1, "attack": 2, "health": 1},
  {"id": "T_001", "type": "MINION", "collectible": true, "cost": 4, "attack": 3, "health": 5,
   "mechanics": ["TAUNT"], "text": "<b>Taunt</b>"},
  {"id": "M_001", "type": "MINION", "collectible": true, "cost": 2, "attack": 1, "health": 4,
   "mechanics": ["RUSH", "WINDFURY"], "text": "<b><b>Rush</b>\nWindfury</b>"},
  {"id": "B_001", "type": "MINION", "collectible": true, "cost": 1, "attack": 2, "health": 1,
   "mechanics": ["BATTLECRY"], "text": "<b>Battlecry:</b> Restore #2 Health."},
  {"id": "F_001", "type": "MINION", "collectible": true, "cost": 4, "attack": 3, "health": 6,
   "mechanics": ["FREEZE"], "text": "<b>Freeze</b> any character damaged by this minion."},
  {"id": "R_001", "type": "MINION", "collectible": true, "cost": 2, "attack": 3, "health": 1,
   "mechanics": ["RUSH"], "text": "<b>Rush</b>. Costs (1) less."},
  {"id": "S_001", "type": "MINION", "collectible": true, "cost": 3, "attack": 4, "health": 2,
   "mechanics": ["STEALTH"], "text": "<b>Stealth</b>"},
  {"id": "N_001", "type": "MINION", "collectible": true, "cost": 1, "attack": 1, "health": 1},
  {"id": "U_001", "type": "MINION", "collectible": false, "cost": 1, "attack": 1, "health": 1},
  {"id": "W_001", "type": "WEAPON", "collectible": true, "cost": 2, "attack": 3, "health": 2}
]"#;

const ROSETTA: &str = r#"[
  {"id": "V_001", "cost": 1, "attack": 2, "health": 1},
  {"id": "T_001", "cost": 4, "attack": 3, "health": 5, "mechanics": ["TAUNT"]},
  {"id": "M_001", "cost": 2, "attack": 1, "health": 4, "mechanics": ["WINDFURY", "RUSH"]},
  {"id": "B_001", "cost": 1, "attack": 2, "health": 1, "mechanics": ["BATTLECRY"]},
  {"id": "F_001", "cost": 4, "attack": 3, "health": 6, "mechanics": ["FREEZE"]},
  {"id": "R_001", "cost": 2, "attack": 3, "health": 1, "mechanics": ["RUSH"]},
  {"id": "S_001", "cost": 3, "attack": 4, "health": 3, "mechanics": ["STEALTH"]},
  {"id": "U_001", "cost": 1, "attack": 1, "health": 1},
  {"id": "W_001", "cost": 2, "attack": 3, "durability": 2}
]"#;

// G1：已實作、數值與關鍵字一致、敘述只有關鍵字（或沒有敘述）的可收藏手下
#[test]
fn g1_keyword_pool_selects_keyword_only_minions() {
    let implemented: BTreeSet<String> = [
        "V_001", "T_001", "M_001", "B_001", "F_001", "R_001", "S_001", "U_001", "W_001",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let pool = keyword_pool(EN, ROSETTA, &implemented).expect("產生卡池失敗");
    // 排除：戰吼、凍結（不在卡池的關鍵字）、有其他敘述、數值不同、RosettaStone 沒有、非收藏、武器
    assert_eq!(pool, vec!["M_001", "T_001", "V_001"]);
}

// G1：納入 git 的卡池清單：134 張，全是只有卡池關鍵字的手下
#[test]
fn g1_committed_keyword_pool() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/keyword_pool.txt");
    let text = std::fs::read_to_string(path).expect("讀不到 data/keyword_pool.txt");
    let ids = parse_pool(&text);
    assert_eq!(ids.len(), 134);
    for id in &ids {
        let c = card_by_id(id).unwrap_or_else(|| panic!("卡表沒有 {id}"));
        assert_eq!(c.card_type, CardType::Minion, "{id}");
        assert!(
            c.keywords.iter().all(|k| KEYWORDS.contains(k)),
            "{id} 有卡池以外的關鍵字：{:?}",
            c.keywords
        );
    }
    let allowed: Vec<Keyword> = KEYWORDS.to_vec();
    assert_eq!(allowed.len(), 8);
}

// 第 2 階段 G2：範例卡池：每張都是可收藏的卡，而且 emberstone-cards 有實作效果
#[test]
fn stage2_g2_committed_example_pool() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/stage2_pool.txt");
    let text = std::fs::read_to_string(path).expect("讀不到 data/stage2_pool.txt");
    let ids = parse_pool(&text);
    assert!(ids.len() >= 25, "卡池太小：{}", ids.len());
    for id in &ids {
        let c = card_by_id(id).unwrap_or_else(|| panic!("卡表沒有 {id}"));
        assert!(c.collectible, "{id} 不是可收藏的卡");
        assert!(emberstone_cards::lookup(id).is_some(), "{id} 沒有實作效果");
    }
}

/// 第 3a 階段的 58 張卡（docs/第3a階段-case表.md）。
const STAGE3A: [&str; 58] = [
    "CORE_AT_037",
    "CORE_AT_055",
    "CORE_AT_064",
    "CORE_BAR_311",
    "CORE_BAR_801",
    "CORE_BAR_812",
    "CORE_BRM_013",
    "CORE_BT_035",
    "CORE_CS2_023",
    "CORE_CS2_028",
    "CORE_CS2_032",
    "CORE_CS2_053",
    "CORE_CS2_072",
    "CORE_CS2_074",
    "CORE_CS2_108",
    "CORE_EX1_169",
    "CORE_EX1_197",
    "CORE_EX1_238",
    "CORE_EX1_278",
    "CORE_EX1_391",
    "CORE_EX1_619",
    "CORE_ICC_055",
    "CORE_NEW1_031",
    "CORE_OG_047",
    "CORE_SW_088",
    "CORE_SW_108",
    "CORE_TRL_307",
    "CORE_TSC_650",
    "CORE_ULD_152",
    "CORE_BT_120",
    "CORE_BT_701",
    "CORE_CS2_179",
    "CORE_CS2_042",
    "CORE_EX1_010",
    "CORE_EX1_011",
    "CORE_EX1_012",
    "CORE_EX1_028",
    "CORE_EX1_043",
    "CORE_EX1_059",
    "CORE_EX1_103",
    "CORE_EX1_414",
    "CORE_EX1_507",
    "CORE_GIL_622",
    "CORE_GVG_085",
    "CORE_ICC_038",
    "CORE_NEW1_018",
    "CORE_NEW1_027",
    "CORE_ONY_018",
    "CORE_ULD_165",
    "CORE_ULD_191",
    "CORE_ULD_271",
    "CORE_ULD_280",
    "CORE_UNG_928",
    "CORE_WC_042",
    "CS3_007",
    "CS3_020",
    "CS3_022",
    "CS3_038",
];

// 第 3a 階段 G1：卡池 = 第 2 階段範例卡池 + 第 3a 階段的 58 張，都是可收藏的卡；
// 有敘述（不只關鍵字）的卡都要有實作效果
#[test]
fn stage3a_g1_committed_pool() {
    let read = |name: &str| {
        let path = format!("{}/../../data/{name}", env!("CARGO_MANIFEST_DIR"));
        parse_pool(&std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("讀不到 {path}")))
    };
    let ids: BTreeSet<String> = read("stage3a_pool.txt").into_iter().collect();
    let mut expected: BTreeSet<String> = read("stage2_pool.txt").into_iter().collect();
    expected.extend(STAGE3A.iter().map(|s| s.to_string()));
    assert_eq!(ids, expected);
    let vanilla = [
        "CORE_BT_701",
        "CORE_CS2_179",
        "CORE_EX1_010",
        "CORE_EX1_028",
        "CORE_GVG_085",
        "CORE_ICC_038",
        "CS3_007",
        "CS3_038",
    ];
    for id in &ids {
        let c = card_by_id(id).unwrap_or_else(|| panic!("卡表沒有 {id}"));
        assert!(c.collectible, "{id} 不是可收藏的卡");
        if !vanilla.contains(&id.as_str()) {
            assert!(emberstone_cards::lookup(id).is_some(), "{id} 沒有實作效果");
        }
    }
}
