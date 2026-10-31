//! 亂數事件與「指定亂數」：照記錄的序列回傳結果，要求不符就回報不同步。

use serde::{Deserialize, Serialize};

/// 一次抽亂數：類型、參數與結果。從 n 個中挑一個記成 Int { lo: 0, hi: n - 1 }。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RngEvent {
    Int {
        lo: i64,
        hi: i64,
        value: i64,
    },
    Float {
        lo: f64,
        hi: f64,
        value: f64,
    },
    /// 洗牌後第 i 個位置放原本的第 perm[i] 個。
    Shuffle {
        n: usize,
        perm: Vec<usize>,
    },
}

/// 亂數不同步：第 index 個事件和要求的不符（或序列已用完）。
#[derive(Debug, Clone, PartialEq)]
pub struct Desync {
    pub index: usize,
    pub expected: Option<RngEvent>,
    pub requested: String,
}

/// 照記錄序列回傳結果的亂數來源。
pub struct ScriptedRng {
    events: Vec<RngEvent>,
    next: usize,
}

impl ScriptedRng {
    pub fn new(events: Vec<RngEvent>) -> Self {
        Self { events, next: 0 }
    }

    /// 取下一個事件；不符合 `matches` 就回報不同步（不前進）。
    fn take<T>(
        &mut self,
        requested: String,
        matches: impl FnOnce(&RngEvent) -> Option<T>,
    ) -> Result<T, Desync> {
        let index = self.next;
        let expected = self.events.get(index).cloned();
        match expected.as_ref().and_then(matches) {
            Some(v) => {
                self.next += 1;
                Ok(v)
            }
            None => Err(Desync {
                index,
                expected,
                requested,
            }),
        }
    }

    pub fn int(&mut self, lo: i64, hi: i64) -> Result<i64, Desync> {
        self.take(format!("int({lo}, {hi})"), |e| match *e {
            RngEvent::Int {
                lo: l,
                hi: h,
                value,
            } if l == lo && h == hi => Some(value),
            _ => None,
        })
    }

    pub fn pick(&mut self, n: usize) -> Result<usize, Desync> {
        self.int(0, n as i64 - 1).map(|v| v as usize)
    }

    pub fn float(&mut self, lo: f64, hi: f64) -> Result<f64, Desync> {
        self.take(format!("float({lo}, {hi})"), |e| match *e {
            RngEvent::Float {
                lo: l,
                hi: h,
                value,
            } if l == lo && h == hi => Some(value),
            _ => None,
        })
    }

    pub fn shuffle(&mut self, n: usize) -> Result<Vec<usize>, Desync> {
        self.take(format!("shuffle({n})"), |e| match e {
            RngEvent::Shuffle { n: m, perm } if *m == n => Some(perm.clone()),
            _ => None,
        })
    }
}
