//! 產生器行為（case D9、D10）。

use emberstone_cardgen::generate;

const STANDARD: &str = "# 註解\nCORE\n";

const FIXTURE: &str = r#"[
  {"id": "TEST_002", "dbfId": 2, "name": "測試手下乙", "set": "CORE", "type": "MINION",
   "cardClass": "NEUTRAL", "cost": 3, "attack": 2, "health": 4,
   "mechanics": ["TAUNT", "FOO_BAR"], "text": "<b>嘲諷</b>"},
  {"id": "TEST_001", "dbfId": 1, "name": "測試法術甲", "set": "LEGACY", "type": "SPELL",
   "cardClass": "MAGE", "cost": 1, "text": "造成$2點傷害"},
  {"id": "BG_TEST", "dbfId": 3, "name": "英雄戰場卡", "set": "BATTLEGROUNDS", "type": "MINION",
   "cost": 1, "attack": 1, "health": 1}
]"#;

// D9：同一份輸入產生兩次，輸出逐位元組相同
#[test]
fn d9_generation_is_deterministic() {
    let a = generate(FIXTURE, STANDARD).expect("產生失敗");
    let b = generate(FIXTURE, STANDARD).expect("產生失敗");
    assert!(!a.cards_rs.is_empty());
    assert_eq!(a.cards_rs, b.cards_rs);
}

// D10：沒見過的關鍵字不中斷，收成「其他」並列在報告
#[test]
fn d10_unknown_keyword_becomes_other_and_is_reported() {
    let out = generate(FIXTURE, STANDARD).expect("遇到未知關鍵字不應該失敗");
    assert!(out.cards_rs.contains(r#"Keyword::Other("FOO_BAR")"#));
    assert!(out.cards_rs.contains("Keyword::Taunt"));
    assert_eq!(out.unknown_keywords, vec!["FOO_BAR".to_string()]);
    assert!(out.report.contains("FOO_BAR"));
}

// 第 1 階段 C4a：來源把武器耐久度放在 health、durability 是 0，產生時要改放到 durability
#[test]
fn stage1_c4a_weapon_durability_comes_from_health() {
    const WEAPON: &str = r#"[
  {"id": "TEST_W01", "dbfId": 9, "name": "測試武器", "set": "CORE", "type": "WEAPON",
   "cardClass": "WARRIOR", "cost": 3, "attack": 2, "health": 2, "durability": 0}
]"#;
    let out = generate(WEAPON, STANDARD).expect("產生失敗");
    assert!(
        out.cards_rs.contains("durability: Some(2)"),
        "{}",
        out.cards_rs
    );
}
