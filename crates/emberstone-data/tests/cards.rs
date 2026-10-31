//! 卡表內容（case D1–D8、D11）。期望值取自 HearthstoneJSON 現行資料（2026-10-19 下載）。

use emberstone_data::{
    CardClass, CardType, Keyword, Rarity, RuneCost, all_cards, card_by_dbf_id, card_by_id,
    is_standard_set,
};

const EXCLUDED_SETS: [&str; 3] = ["BATTLEGROUNDS", "LETTUCE", "TB"];

fn source_cards() -> Vec<serde_json::Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/tw_all.json");
    let text = std::fs::read_to_string(path).expect("讀不到 data/tw_all.json");
    serde_json::from_str(&text).expect("tw_all.json 格式錯誤")
}

// D1：排除英雄戰場、傭兵、旅店大亂鬥，其餘全部收錄
#[test]
fn d1_card_count_matches_source_minus_excluded_sets() {
    let expected = source_cards()
        .iter()
        .filter(|c| !EXCLUDED_SETS.contains(&c["set"].as_str().unwrap()))
        .count();
    assert_eq!(all_cards().len(), expected);
    assert!(all_cards().iter().all(|c| !EXCLUDED_SETS.contains(&c.set)));
}

// D2：英雄戰場的卡查不到
#[test]
fn d2_battlegrounds_card_not_found() {
    let bg = source_cards()
        .into_iter()
        .find(|c| c["set"] == "BATTLEGROUNDS")
        .expect("來源裡應該有英雄戰場的卡");
    assert!(card_by_id(bg["id"].as_str().unwrap()).is_none());
    assert!(card_by_dbf_id(bg["dbfId"].as_u64().unwrap() as u32).is_none());
}

// D3：不存在的卡號、dbfId 查不到
#[test]
fn d3_unknown_id_and_dbf_id_not_found() {
    assert!(card_by_id("NO_SUCH_CARD_000").is_none());
    assert!(card_by_dbf_id(0).is_none());
    assert!(card_by_dbf_id(u32::MAX).is_none());
}

// D4：血法師薩爾諾斯
#[test]
fn d4_bloodmage_thalnos() {
    let c = card_by_id("CORE_EX1_012").expect("查不到 CORE_EX1_012");
    assert_eq!(card_by_dbf_id(69655).map(|x| x.id), Some("CORE_EX1_012"));
    assert_eq!(c.dbf_id, 69655);
    assert_eq!(c.name, "血法師薩爾諾斯");
    assert_eq!(c.card_type, CardType::Minion);
    assert_eq!(c.class, Some(CardClass::Neutral));
    assert_eq!((c.cost, c.attack, c.health), (Some(2), Some(1), Some(1)));
    assert_eq!(c.rarity, Some(Rarity::Legendary));
    assert_eq!(c.races, &["UNDEAD"]);
    assert!(c.keywords.contains(&Keyword::Deathrattle));
    assert!(c.keywords.contains(&Keyword::SpellPower));
    assert_eq!(c.spell_damage, 1);
    assert!(c.collectible);
    assert_eq!(c.set, "CORE");
}

// D5：火球術
#[test]
fn d5_fireball() {
    let c = card_by_id("CS2_029").expect("查不到 CS2_029");
    assert_eq!(card_by_dbf_id(315).map(|x| x.id), Some("CS2_029"));
    assert_eq!(c.card_type, CardType::Spell);
    assert_eq!(c.class, Some(CardClass::Mage));
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.spell_school, Some("FIRE"));
    assert_eq!(c.set, "LEGACY");
    assert_eq!(c.text, "造成6點傷害");
}

// D6：幸運幣
#[test]
fn d6_the_coin() {
    let c = card_by_id("GAME_005").expect("查不到 GAME_005");
    assert_eq!(c.card_type, CardType::Spell);
    assert_eq!(c.class, Some(CardClass::Neutral));
    assert_eq!(c.cost, Some(0));
    assert!(!c.collectible);
    assert_eq!(c.set, "CORE");
    assert_eq!(c.text, "在本回合獲得額外1顆法力水晶");
}

// D7：吞噬（死亡騎士符文）
#[test]
fn d7_devour_rune_cost() {
    let c = card_by_id("CORE_CATA_007").expect("查不到 CORE_CATA_007");
    assert_eq!(c.class, Some(CardClass::DeathKnight));
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.card_type, CardType::Spell);
    assert_eq!(c.rarity, Some(Rarity::Rare));
    assert_eq!(
        c.rune_cost,
        RuneCost {
            blood: 2,
            frost: 0,
            unholy: 0
        }
    );
    assert_eq!(
        c.text,
        "對兩個隨機敵方手下造成3點傷害。每有一個手下死亡便抽一張牌"
    );
}

// D8：圖騰魔像（超載、重印卡合併計算）
#[test]
fn d8_totem_golem() {
    let c = card_by_id("CORE_AT_052").expect("查不到 CORE_AT_052");
    assert_eq!(c.class, Some(CardClass::Shaman));
    assert_eq!((c.cost, c.attack, c.health), (Some(2), Some(3), Some(4)));
    assert_eq!(c.races, &["TOTEM"]);
    assert_eq!(c.overload, 1);
    assert_eq!(c.rarity, Some(Rarity::Common));
    assert_eq!(c.count_as_copy_of, Some(2610));
    assert_eq!(c.text, "超載：(1)");
}

// D11：標準模式系列判斷（清單在 data/standard_sets.txt）
#[test]
fn d11_standard_sets() {
    for set in [
        "CORE",
        "EMERALD_DREAM",
        "THE_LOST_CITY",
        "TIME_TRAVEL",
        "CATACLYSM",
        "ESCAPEFROM_VIOLET_HOLD",
        "BE",
        "EVENT",
    ] {
        assert!(is_standard_set(set), "{set} 應該是標準模式");
    }
    for set in ["LEGACY", "VANILLA"] {
        assert!(!is_standard_set(set), "{set} 不應該是標準模式");
    }
    assert!(card_by_id("CORE_EX1_012").unwrap().is_standard());
    assert!(!card_by_id("CS2_029").unwrap().is_standard());
}

// 第 1 階段 C4a：奧達奇戰刃 3 費 2/2，耐久度 2
#[test]
fn stage1_c4a_aldrachi_warblades_durability() {
    let c = card_by_id("CORE_BT_921").expect("查不到 CORE_BT_921");
    assert_eq!(c.name, "奧達奇戰刃");
    assert_eq!(c.card_type, CardType::Weapon);
    assert_eq!(
        (c.cost, c.attack, c.durability),
        (Some(3), Some(2), Some(2))
    );
}
