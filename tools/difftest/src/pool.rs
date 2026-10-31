//! 第 1 階段差分測試的卡池：RosettaStone 已實作、數值與關鍵字一致、敘述只有關鍵字的手下。

use std::collections::BTreeSet;
use std::collections::HashMap;

use emberstone_data::Keyword;
use serde_json::Value;

/// 卡池允許的關鍵字。
pub const KEYWORDS: [Keyword; 8] = [
    Keyword::Taunt,
    Keyword::DivineShield,
    Keyword::Charge,
    Keyword::Rush,
    Keyword::Windfury,
    Keyword::Poisonous,
    Keyword::Lifesteal,
    Keyword::Stealth,
];

/// 卡池關鍵字在 HearthstoneJSON 的名稱，以及敘述裡可以出現的英文字。
const MECHANICS: [&str; 8] = [
    "TAUNT",
    "DIVINE_SHIELD",
    "CHARGE",
    "RUSH",
    "WINDFURY",
    "POISONOUS",
    "LIFESTEAL",
    "STEALTH",
];
const WORDS: [&str; 9] = [
    "Taunt",
    "Divine",
    "Shield",
    "Charge",
    "Rush",
    "Windfury",
    "Poisonous",
    "Lifesteal",
    "Stealth",
];

fn parse_array(text: &str, what: &str) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("{what} 格式錯誤：{e}"))?;
    v.as_array()
        .cloned()
        .ok_or_else(|| format!("{what} 應該是陣列"))
}

fn mechanics(c: &Value) -> BTreeSet<&str> {
    c.get("mechanics")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// 敘述拿掉 HTML 標記、`[x]`、逗號與句點後，只剩關鍵字。
fn text_is_keywords_only(text: &str) -> bool {
    let text = text.replace("[x]", " ");
    let mut plain = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => {
                in_tag = true;
                plain.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            ',' | '.' => plain.push(' '),
            _ => plain.push(ch),
        }
    }
    plain.split_whitespace().all(|w| WORDS.contains(&w))
}

/// 從我們的英文卡表與 RosettaStone 的 cards.json 選出卡池（依卡號排序）。
pub fn keyword_pool(
    en_cards_json: &str,
    rosetta_cards_json: &str,
    implemented: &BTreeSet<String>,
) -> Result<Vec<String>, String> {
    let ours = parse_array(en_cards_json, "我們的卡表")?;
    let theirs = parse_array(rosetta_cards_json, "RosettaStone 卡表")?;
    let by_id: HashMap<&str, &Value> = theirs
        .iter()
        .filter_map(|c| c.get("id").and_then(Value::as_str).map(|id| (id, c)))
        .collect();
    let mut out = Vec::new();
    for c in &ours {
        let Some(id) = c.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(r) = by_id.get(id).filter(|_| implemented.contains(id)) else {
            continue;
        };
        let collectible = c.get("collectible").and_then(Value::as_bool) == Some(true);
        if !collectible || c.get("type").and_then(Value::as_str) != Some("MINION") {
            continue;
        }
        let m = mechanics(c);
        if !m.iter().all(|k| MECHANICS.contains(k)) || m != mechanics(r) {
            continue;
        }
        if !text_is_keywords_only(c.get("text").and_then(Value::as_str).unwrap_or("")) {
            continue;
        }
        let same = ["cost", "attack", "health", "durability"]
            .iter()
            .all(|f| c.get(*f).and_then(Value::as_i64) == r.get(*f).and_then(Value::as_i64));
        if same {
            out.push(id.to_string());
        }
    }
    out.sort();
    Ok(out)
}

/// 讀回卡池清單（一行一張，`#` 開頭為註解）。
pub fn parse_pool(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// 輸出卡池清單。
pub fn to_text(ids: &[String]) -> String {
    let mut out = String::from(
        "# 第 1 階段差分測試卡池：RosettaStone 已實作、數值與關鍵字一致、敘述只有關鍵字的手下。\n\
         # 由 `difftest keywords --rosetta <RosettaStone 原始碼目錄>` 產生，請勿手動修改。\n",
    );
    for id in ids {
        out.push_str(id);
        out.push('\n');
    }
    out
}
