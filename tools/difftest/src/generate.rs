//! 隨機對局產生（case C3）：每一步從合法行動中依種子隨機挑一個。

use crate::actions::Action;

/// 提供合法行動並執行行動的對局；act 回傳遊戲是否結束。
pub trait OptionSource {
    fn options(&mut self) -> Vec<Action>;
    fn act(&mut self, action: &Action) -> bool;
}

/// 可重現的簡單亂數產生器（SplitMix64）。
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0..n 的隨機整數（n > 0）。
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// 依種子產生並執行隨機行動序列，最多 max_steps 步。
pub fn random_actions(seed: u64, game: &mut dyn OptionSource, max_steps: usize) -> Vec<Action> {
    let mut rng = SplitMix64::new(seed);
    let mut actions = Vec::new();
    while actions.len() < max_steps {
        let opts = game.options();
        if opts.is_empty() {
            break;
        }
        let action = opts[rng.below(opts.len())].clone();
        let done = game.act(&action);
        actions.push(action);
        if done {
            break;
        }
    }
    actions
}
