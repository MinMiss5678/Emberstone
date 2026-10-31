//! 對局亂數：同種子可重現；也可換成「照記錄序列回傳」（差分測試、實戰紀錄重播）。

/// 一次抽亂數：類型、參數與結果。從 n 個中挑一個記成 `Int { lo: 0, hi: n - 1 }`。
#[derive(Debug, Clone, PartialEq)]
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

/// 亂數不同步：第 index 個事件和要求的不符（或序列已用完，expected 為 None）。
#[derive(Debug, Clone, PartialEq)]
pub struct Desync {
    pub index: usize,
    pub expected: Option<RngEvent>,
    pub requested: String,
}

/// 對局的亂數來源。
#[derive(Debug, Clone, PartialEq)]
pub enum Rng {
    /// SplitMix64。
    Seeded(u64),
    /// 照記錄序列回傳。
    Scripted { events: Vec<RngEvent>, next: usize },
}

impl Rng {
    pub fn seeded(seed: u64) -> Self {
        Rng::Seeded(seed)
    }

    pub fn scripted(events: Vec<RngEvent>) -> Self {
        Rng::Scripted { events, next: 0 }
    }

    /// 閉區間 [lo, hi] 的整數。
    pub fn int(&mut self, lo: i64, hi: i64) -> Result<i64, Desync> {
        match self {
            Rng::Seeded(state) => {
                let span = (hi - lo) as u64 + 1;
                Ok(lo + (split_mix(state) % span) as i64)
            }
            Rng::Scripted { .. } => self.take(format!("int({lo}, {hi})"), |e| match *e {
                RngEvent::Int {
                    lo: l,
                    hi: h,
                    value,
                } if l == lo && h == hi => Some(value),
                _ => None,
            }),
        }
    }

    /// 洗 n 個：回傳排列，洗牌後第 i 個位置放原本的第 perm[i] 個。
    pub fn shuffle(&mut self, n: usize) -> Result<Vec<usize>, Desync> {
        match self {
            Rng::Seeded(state) => {
                let mut perm: Vec<usize> = (0..n).collect();
                for i in (1..n).rev() {
                    let j = (split_mix(state) % (i as u64 + 1)) as usize;
                    perm.swap(i, j);
                }
                Ok(perm)
            }
            Rng::Scripted { .. } => self.take(format!("shuffle({n})"), |e| match e {
                RngEvent::Shuffle { n: m, perm } if *m == n => Some(perm.clone()),
                _ => None,
            }),
        }
    }

    /// 記錄序列的下一個事件；不符合 `matches` 就回報不同步（不前進）。
    fn take<T>(
        &mut self,
        requested: String,
        matches: impl FnOnce(&RngEvent) -> Option<T>,
    ) -> Result<T, Desync> {
        let Rng::Scripted { events, next } = self else {
            unreachable!("只有記錄序列會呼叫 take");
        };
        let index = *next;
        let expected = events.get(index);
        match expected.and_then(matches) {
            Some(v) => {
                *next += 1;
                Ok(v)
            }
            None => Err(Desync {
                index,
                expected: expected.cloned(),
                requested,
            }),
        }
    }
}

fn split_mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
