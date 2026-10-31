//! 卡表產生工具：讀 HearthstoneJSON（`tw_all.json`），產生 `emberstone-data` 的卡表原始碼。

use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::Value;

/// 不收錄的系列：英雄戰場、傭兵、旅店大亂鬥。
pub const EXCLUDED_SETS: [&str; 3] = ["BATTLEGROUNDS", "LETTUCE", "TB"];

/// 產生結果。
pub struct Generated {
    /// `emberstone-data/src/generated/cards.rs` 的內容。
    pub cards_rs: String,
    /// 沒有對應列舉、收成 `Keyword::Other` 的關鍵字（排序、不重複）。
    pub unknown_keywords: Vec<String>,
    /// 給人看的產生報告。
    pub report: String,
}

/// 由卡片 JSON 與標準模式系列清單產生卡表。
pub fn generate(cards_json: &str, standard_sets: &str) -> Result<Generated, String> {
    let root: Value =
        serde_json::from_str(cards_json).map_err(|e| format!("卡片 JSON 格式錯誤：{e}"))?;
    let all = root.as_array().ok_or("卡片 JSON 應該是陣列")?;

    let mut cards: Vec<&Value> = Vec::with_capacity(all.len());
    let mut excluded = 0usize;
    for c in all {
        let set = str_field(c, "set").ok_or_else(|| format!("缺少 set：{c}"))?;
        if EXCLUDED_SETS.contains(&set) {
            excluded += 1;
        } else {
            cards.push(c);
        }
    }
    for c in &cards {
        str_field(c, "id").ok_or_else(|| format!("缺少 id：{c}"))?;
        c["dbfId"]
            .as_u64()
            .ok_or_else(|| format!("缺少 dbfId：{c}"))?;
    }
    cards.sort_by(|a, b| str_field(a, "id").cmp(&str_field(b, "id")));

    let sets: Vec<&str> = standard_sets
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let mut unknown = BTreeSet::new();
    let mut out = String::new();
    out.push_str("// 由 emberstone-cardgen 產生，請勿手動修改。重新產生：cargo run -p emberstone-cardgen --bin gen-cards\n");
    out.push_str("use crate::{Card, CardClass, CardType, Keyword, Rarity, RuneCost};\n\n");
    out.push_str("pub(crate) static CARDS: &[Card] = &[\n");
    for c in &cards {
        write_card(&mut out, c, &mut unknown);
    }
    out.push_str("];\n\n");

    let mut dbf: Vec<(u64, usize)> = cards
        .iter()
        .enumerate()
        .map(|(i, c)| (c["dbfId"].as_u64().unwrap(), i))
        .collect();
    dbf.sort();
    out.push_str("pub(crate) static DBF_INDEX: &[(u32, u32)] = &[\n");
    for (d, i) in &dbf {
        let _ = writeln!(out, "    ({d}, {i}),");
    }
    out.push_str("];\n\n");

    out.push_str("pub(crate) static STANDARD_SETS: &[&str] = &[");
    out.push_str(
        &sets
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str("];\n");

    let unknown_keywords: Vec<String> = unknown.into_iter().collect();
    let mut report = String::new();
    let _ = writeln!(
        report,
        "收錄 {} 張；排除 {} 張（{}）",
        cards.len(),
        excluded,
        EXCLUDED_SETS.join("、")
    );
    let _ = writeln!(report, "標準模式系列：{}", sets.join("、"));
    if unknown_keywords.is_empty() {
        let _ = writeln!(report, "未知關鍵字：無");
    } else {
        let _ = writeln!(
            report,
            "未知關鍵字（收成 Keyword::Other，共 {} 種）：{}",
            unknown_keywords.len(),
            unknown_keywords.join("、")
        );
    }

    Ok(Generated {
        cards_rs: out,
        unknown_keywords,
        report,
    })
}

fn str_field<'a>(c: &'a Value, key: &str) -> Option<&'a str> {
    c.get(key).and_then(Value::as_str)
}

fn opt_i64(c: &Value, key: &str) -> String {
    match c.get(key).and_then(Value::as_i64) {
        Some(v) => format!("Some({v})"),
        None => "None".to_string(),
    }
}

fn write_card(out: &mut String, c: &Value, unknown: &mut BTreeSet<String>) {
    let s = |k: &str| str_field(c, k).unwrap_or("");
    let class = str_field(c, "cardClass")
        .map_or("None".to_string(), |v| format!("Some({})", class_expr(v)));
    let classes: Vec<String> = c
        .get("classes")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(class_expr).collect())
        .unwrap_or_default();
    let rarity =
        str_field(c, "rarity").map_or("None".to_string(), |v| format!("Some({})", rarity_expr(v)));
    let races: Vec<String> = match c.get("races").and_then(Value::as_array) {
        Some(a) => a
            .iter()
            .filter_map(Value::as_str)
            .map(|r| format!("{r:?}"))
            .collect(),
        None => str_field(c, "race")
            .map(|r| vec![format!("{r:?}")])
            .unwrap_or_default(),
    };
    let school = str_field(c, "spellSchool").map_or("None".to_string(), |v| format!("Some({v:?})"));
    let keywords: Vec<String> = c
        .get("mechanics")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|m| {
                    keyword_expr(m).map(str::to_string).unwrap_or_else(|| {
                        unknown.insert(m.to_string());
                        format!("Keyword::Other({m:?})")
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let rune = |k: &str| {
        c.get("runeCost")
            .and_then(|r| r.get(k))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let copy_of = c
        .get("countAsCopyOfDbfId")
        .and_then(Value::as_u64)
        .map_or("None".to_string(), |v| format!("Some({v})"));

    let _ = writeln!(
        out,
        "    Card {{ id: {:?}, dbf_id: {}, name: {:?}, card_type: {}, class: {}, classes: &[{}], cost: {}, attack: {}, health: {}, durability: {}, armor: {}, rarity: {}, set: {:?}, collectible: {}, races: &[{}], spell_school: {}, keywords: &[{}], overload: {}, spell_damage: {}, rune_cost: RuneCost {{ blood: {}, frost: {}, unholy: {} }}, count_as_copy_of: {}, text: {:?} }},",
        s("id"),
        c["dbfId"].as_u64().unwrap(),
        s("name"),
        type_expr(s("type")),
        class,
        classes.join(", "),
        opt_i64(c, "cost"),
        opt_i64(c, "attack"),
        opt_i64(c, "health"),
        opt_i64(c, "durability"),
        opt_i64(c, "armor"),
        rarity,
        s("set"),
        c.get("collectible")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        races.join(", "),
        school,
        keywords.join(", "),
        c.get("overload").and_then(Value::as_i64).unwrap_or(0),
        c.get("spellDamage").and_then(Value::as_i64).unwrap_or(0),
        rune("blood"),
        rune("frost"),
        rune("unholy"),
        copy_of,
        clean_text(s("text")),
    );
}

fn type_expr(v: &str) -> String {
    match v {
        "MINION" => "CardType::Minion".into(),
        "SPELL" => "CardType::Spell".into(),
        "WEAPON" => "CardType::Weapon".into(),
        "HERO" => "CardType::Hero".into(),
        "HERO_POWER" => "CardType::HeroPower".into(),
        "LOCATION" => "CardType::Location".into(),
        "ENCHANTMENT" => "CardType::Enchantment".into(),
        other => format!("CardType::Other({other:?})"),
    }
}

fn class_expr(v: &str) -> String {
    let name = match v {
        "NEUTRAL" => "Neutral",
        "DEATHKNIGHT" => "DeathKnight",
        "DEMONHUNTER" => "DemonHunter",
        "DRUID" => "Druid",
        "HUNTER" => "Hunter",
        "MAGE" => "Mage",
        "PALADIN" => "Paladin",
        "PRIEST" => "Priest",
        "ROGUE" => "Rogue",
        "SHAMAN" => "Shaman",
        "WARLOCK" => "Warlock",
        "WARRIOR" => "Warrior",
        other => return format!("CardClass::Other({other:?})"),
    };
    format!("CardClass::{name}")
}

fn rarity_expr(v: &str) -> String {
    let name = match v {
        "FREE" => "Free",
        "COMMON" => "Common",
        "RARE" => "Rare",
        "EPIC" => "Epic",
        "LEGENDARY" => "Legendary",
        other => return format!("Rarity::Other({other:?})"),
    };
    format!("Rarity::{name}")
}

fn keyword_expr(m: &str) -> Option<&'static str> {
    Some(match m {
        "TAUNT" => "Keyword::Taunt",
        "DIVINE_SHIELD" => "Keyword::DivineShield",
        "CHARGE" => "Keyword::Charge",
        "RUSH" => "Keyword::Rush",
        "WINDFURY" => "Keyword::Windfury",
        "POISONOUS" => "Keyword::Poisonous",
        "LIFESTEAL" => "Keyword::Lifesteal",
        "STEALTH" => "Keyword::Stealth",
        "FREEZE" => "Keyword::Freeze",
        "BATTLECRY" => "Keyword::Battlecry",
        "DEATHRATTLE" => "Keyword::Deathrattle",
        "SPELLPOWER" => "Keyword::SpellPower",
        "OVERLOAD" => "Keyword::Overload",
        "SECRET" => "Keyword::Secret",
        "SILENCE" => "Keyword::Silence",
        "DISCOVER" => "Keyword::Discover",
        "CHOOSE_ONE" => "Keyword::ChooseOne",
        "COMBO" => "Keyword::Combo",
        "REBORN" => "Keyword::Reborn",
        _ => return None,
    })
}

/// 敘述轉純文字：拿掉 `[x]`、HTML 標記，以及數字前的 `$`（法術傷害）與 `#`（治療）標記。
fn clean_text(text: &str) -> String {
    let text = text.strip_prefix("[x]").unwrap_or(text);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_tag = false;
    while let Some(ch) = chars.next() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            '$' | '#' if chars.peek().is_some_and(|n| n.is_ascii_digit()) => {}
            _ => out.push(ch),
        }
    }
    out
}
