//! 卡片批次分類：比對我們的卡和 RosettaStone 已實作的卡。

use std::collections::{BTreeSet, HashMap};

use serde_json::Value;

/// 批次：A＝可直接對照，B＝有實作但欄位不同，C＝沒有參考。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Batch {
    A,
    B,
    C,
}

/// 分類結果的一列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub batch: Batch,
    pub rosetta_id: Option<String>,
    pub diffs: Vec<String>,
}

/// 從 RosettaStone 的卡片清單文件（CardList - *.md）取出已實作的卡號。
/// 表格列的格式：`系列 | 卡號 | 英文名 | O`（O 表示已實作）。
pub fn implemented_ids(card_lists: &[&str]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for text in card_lists {
        for line in text.lines() {
            let parts: Vec<&str> = line.split('|').map(str::trim).collect();
            if parts.len() >= 4
                && parts[3] == "O"
                && !parts[1].is_empty()
                && parts[1]
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                ids.insert(parts[1].to_string());
            }
        }
    }
    ids
}

fn parse_array(text: &str, what: &str) -> Result<Vec<Value>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("{what} 格式錯誤：{e}"))?;
    v.as_array()
        .cloned()
        .ok_or_else(|| format!("{what} 應該是陣列"))
}

/// 比對欄位，依固定順序列出不同的欄位名稱。
fn differences(ours: &Value, theirs: &Value) -> Vec<String> {
    let mut diffs = Vec::new();
    for field in ["cost", "attack", "health", "durability"] {
        if ours.get(field).and_then(Value::as_i64) != theirs.get(field).and_then(Value::as_i64) {
            diffs.push(field.to_string());
        }
    }
    let mechanics = |c: &Value| -> Vec<String> {
        let mut m: Vec<String> = c
            .get("mechanics")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        m.sort();
        m
    };
    if mechanics(ours) != mechanics(theirs) {
        diffs.push("mechanics".to_string());
    }
    let text = |c: &Value| normalize_text(c.get("text").and_then(Value::as_str).unwrap_or(""));
    if text(ours) != text(theirs) {
        diffs.push("text".to_string());
    }
    diffs
}

/// 敘述正規化：HTML 標記換成空白、拿掉 `[x]` 與數字前的 `$`／`#`、空白合併、轉小寫。
fn normalize_text(text: &str) -> String {
    let text = text.replace("[x]", "");
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_tag = false;
    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            '$' | '#' if chars.peek().is_some_and(|n| n.is_ascii_digit()) => {}
            _ => out.push(ch),
        }
    }
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// 分類：輸入我們的英文卡表與 RosettaStone 的 cards.json。只分類核心系列的可收藏卡，依卡號排序。
pub fn classify(
    en_cards_json: &str,
    rosetta_cards_json: &str,
    implemented: &BTreeSet<String>,
) -> Result<Vec<Row>, String> {
    let ours = parse_array(en_cards_json, "我們的卡表")?;
    let theirs = parse_array(rosetta_cards_json, "RosettaStone 卡表")?;
    let mut by_id: HashMap<&str, &Value> = HashMap::new();
    let mut by_dbf: HashMap<i64, &Value> = HashMap::new();
    for c in &theirs {
        if let Some(id) = c.get("id").and_then(Value::as_str) {
            by_id.insert(id, c);
        }
        if let Some(dbf) = c.get("dbfId").and_then(Value::as_i64) {
            by_dbf.insert(dbf, c);
        }
    }
    let is_implemented = |c: &Value| {
        c.get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| implemented.contains(id))
    };

    let mut rows = Vec::new();
    for c in &ours {
        let collectible = c.get("collectible").and_then(Value::as_bool) == Some(true);
        if c.get("set").and_then(Value::as_str) != Some("CORE") || !collectible {
            continue;
        }
        let id = c
            .get("id")
            .and_then(Value::as_str)
            .ok_or("卡片缺少 id")?
            .to_string();
        let reference: Option<&Value> = by_id
            .get(id.as_str())
            .copied()
            .filter(|r| is_implemented(r))
            .or_else(|| {
                c.get("countAsCopyOfDbfId")
                    .and_then(Value::as_i64)
                    .and_then(|d| by_dbf.get(&d).copied())
                    .filter(|r| is_implemented(r))
            });
        let row = match reference {
            None => Row {
                id,
                batch: Batch::C,
                rosetta_id: None,
                diffs: Vec::new(),
            },
            Some(r) => {
                let diffs = differences(c, r);
                Row {
                    id,
                    batch: if diffs.is_empty() { Batch::A } else { Batch::B },
                    rosetta_id: r.get("id").and_then(Value::as_str).map(str::to_string),
                    diffs,
                }
            }
        };
        rows.push(row);
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(rows)
}

const HEADER: &str = "id\tbatch\trosetta_id\tdiffs";

/// 輸出成 TSV（第一列為欄位名稱）。
pub fn to_tsv(rows: &[Row]) -> String {
    let mut out = format!("{HEADER}\n");
    for r in rows {
        let batch = match r.batch {
            Batch::A => "A",
            Batch::B => "B",
            Batch::C => "C",
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            r.id,
            batch,
            r.rosetta_id.as_deref().unwrap_or(""),
            r.diffs.join(",")
        ));
    }
    out
}

/// 讀回 TSV。
pub fn parse_tsv(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER) {
        return Err("批次表的欄位名稱不符".to_string());
    }
    lines
        .filter(|l| !l.is_empty())
        .map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() != 4 {
                return Err(format!("批次表格式錯誤：{line}"));
            }
            let batch = match cols[1] {
                "A" => Batch::A,
                "B" => Batch::B,
                "C" => Batch::C,
                other => return Err(format!("不明的批次 {other}")),
            };
            Ok(Row {
                id: cols[0].to_string(),
                batch,
                rosetta_id: (!cols[2].is_empty()).then(|| cols[2].to_string()),
                diffs: if cols[3].is_empty() {
                    Vec::new()
                } else {
                    cols[3].split(',').map(str::to_string).collect()
                },
            })
        })
        .collect()
}
