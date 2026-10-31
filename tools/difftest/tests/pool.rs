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
