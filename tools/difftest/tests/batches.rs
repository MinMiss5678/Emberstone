//! 卡片批次分類（case B1–B5）。夾具取自 HearthstoneJSON build 255790 與 RosettaStone commit 1690208 的真實資料。

use emberstone_difftest::batches::{Batch, Row, classify, implemented_ids, parse_tsv, to_tsv};

const EN: &str = include_str!("fixtures/en_cards.json");
const ROSETTA: &str = include_str!("fixtures/rosetta_cards.json");
const CARD_LIST: &str = include_str!("fixtures/cardlist.md");

fn rows() -> Vec<Row> {
    let implemented = implemented_ids(&[CARD_LIST]);
    classify(EN, ROSETTA, &implemented).expect("分類失敗")
}

fn row<'a>(rows: &'a [Row], id: &str) -> &'a Row {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("找不到 {id}"))
}

fn committed_rows() -> Vec<Row> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/batches.tsv");
    let text = std::fs::read_to_string(path).expect("讀不到 data/batches.tsv");
    parse_tsv(&text).expect("batches.tsv 格式錯誤")
}

// B1：同卡號已實作、各欄位相同 → 批次 A
#[test]
fn b1_same_id_identical_is_batch_a() {
    let rows = rows();
    let r = row(&rows, "CORE_AT_055");
    assert_eq!(r.batch, Batch::A);
    assert_eq!(r.rosetta_id.as_deref(), Some("CORE_AT_055"));
    assert!(r.diffs.is_empty());
}

// B2：同卡號沒實作，「算作同一張卡」指向的原版已實作且相同 → 批次 A，對照原版卡號
#[test]
fn b2_reprint_via_count_as_copy_is_batch_a() {
    let rows = rows();
    let r = row(&rows, "CORE_BAR_311");
    assert_eq!(r.batch, Batch::A);
    assert_eq!(r.rosetta_id.as_deref(), Some("BAR_311"));
    assert!(r.diffs.is_empty());
}

// B3：有實作但欄位不同 → 批次 B，列出差異欄位
#[test]
fn b3_differences_are_batch_b_with_fields() {
    let rows = rows();
    let r = row(&rows, "CORE_BT_921");
    assert_eq!(r.batch, Batch::B);
    assert_eq!(r.rosetta_id.as_deref(), Some("CORE_BT_921"));
    assert_eq!(
        r.diffs,
        vec!["health".to_string(), "durability".to_string()]
    );
}

// B4：RosettaStone 沒有（含死亡騎士）→ 批次 C
#[test]
fn b4_missing_cards_are_batch_c() {
    let rows = rows();
    for id in ["CORE_AT_052", "CORE_CATA_007"] {
        let r = row(&rows, id);
        assert_eq!(r.batch, Batch::C, "{id}");
        assert_eq!(r.rosetta_id, None, "{id}");
    }
}

// B5：只分類核心系列的可收藏卡；輸出可重現
#[test]
fn b5_scope_and_deterministic_output() {
    let rows = rows();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "CORE_AT_052",
            "CORE_AT_055",
            "CORE_BAR_311",
            "CORE_BT_921",
            "CORE_CATA_007"
        ]
    );
    let tsv = to_tsv(&rows);
    assert_eq!(tsv, to_tsv(&self::rows()));
    assert_eq!(parse_tsv(&tsv).expect("解析失敗"), rows);
}

// B1–B5 在真實資料產生的 data/batches.tsv 上同樣成立
#[test]
fn b5_committed_batches_file() {
    let rows = committed_rows();
    assert_eq!(rows.len(), 289);
    assert_eq!(row(&rows, "CORE_AT_055").batch, Batch::A);
    assert_eq!(
        row(&rows, "CORE_BAR_311").rosetta_id.as_deref(),
        Some("BAR_311")
    );
    assert_eq!(row(&rows, "CORE_BT_921").batch, Batch::B);
    assert_eq!(row(&rows, "CORE_AT_052").batch, Batch::C);
    assert_eq!(row(&rows, "CORE_CATA_007").batch, Batch::C);
}
