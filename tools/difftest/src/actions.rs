//! 橋接程式的行動格式（用位置描述，兩個引擎通用）。

use serde::{Deserialize, Serialize};

/// 指定的角色：英雄或戰場上第 index 隻手下。player 為 1 或 2。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    Hero { player: u8 },
    Minion { player: u8, index: usize },
}

/// 一個行動。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    /// 打出手牌第 hand 張；手下要指定放在戰場第 pos 格；二選一卡 choose_one 為 1 或 2，其餘為 0。
    Play {
        hand: usize,
        pos: Option<usize>,
        target: Option<Target>,
        choose_one: u8,
    },
    Attack {
        attacker: Target,
        target: Target,
    },
    Power {
        target: Option<Target>,
    },
    /// 發現等選擇畫面，選第 index 個。
    Choose {
        index: usize,
    },
    End,
}
