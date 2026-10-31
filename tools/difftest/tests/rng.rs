//! Emberstone 端「指定亂數」（case R4）。

use emberstone_difftest::rng::{RngEvent, ScriptedRng};

fn int(lo: i64, hi: i64, value: i64) -> RngEvent {
    RngEvent::Int { lo, hi, value }
}

// R4：照序列依序回傳記錄的結果
#[test]
fn r4_returns_recorded_values_in_order() {
    let mut rng = ScriptedRng::new(vec![
        int(0, 2, 1),
        RngEvent::Shuffle {
            n: 3,
            perm: vec![2, 0, 1],
        },
        RngEvent::Float {
            lo: 0.0,
            hi: 1.0,
            value: 0.25,
        },
    ]);
    assert_eq!(rng.int(0, 2), Ok(1));
    assert_eq!(rng.shuffle(3), Ok(vec![2, 0, 1]));
    assert_eq!(rng.float(0.0, 1.0), Ok(0.25));
}

// R4：從 n 個中挑一個，等同範圍 0..=n-1 的整數
#[test]
fn r4_pick_is_int_zero_to_n_minus_one() {
    let mut rng = ScriptedRng::new(vec![int(0, 4, 3)]);
    assert_eq!(rng.pick(5), Ok(3));
}

// R4：參數不同 → 回報不同步，指出第幾個事件
#[test]
fn r4_parameter_mismatch_reports_event_index() {
    let mut rng = ScriptedRng::new(vec![int(0, 2, 1), int(0, 3, 2)]);
    assert_eq!(rng.int(0, 2), Ok(1));
    let err = rng.int(0, 5).unwrap_err();
    assert_eq!(err.index, 1);
}

// R4：類型不同 → 回報不同步
#[test]
fn r4_kind_mismatch_reports_event_index() {
    let mut rng = ScriptedRng::new(vec![RngEvent::Shuffle {
        n: 3,
        perm: vec![0, 1, 2],
    }]);
    assert_eq!(rng.int(0, 2).unwrap_err().index, 0);
}

// R4：序列用完還要亂數 → 回報不同步
#[test]
fn r4_exhausted_sequence_reports_event_index() {
    let mut rng = ScriptedRng::new(vec![int(0, 1, 0)]);
    assert_eq!(rng.int(0, 1), Ok(0));
    assert_eq!(rng.int(0, 1).unwrap_err().index, 1);
}
