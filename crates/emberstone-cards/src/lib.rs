//! 卡片效果：英雄技能、戰吼、死亡之聲、觸發、被動加成、秘密等。
//!
//! 效果盡量寫成資料（`emberstone_core` 的效果列舉組合），特殊卡才寫函式。
//! 隨機效果的抽法要和 RosettaStone 相同（類型、範圍、候選順序），差分測試才能重播；
//! 每張隨機卡的抽法寫在該卡的註解。

mod basic;
mod hero_powers;
mod minions;
mod secrets;

use emberstone_core::CardDef;

/// 依卡號查卡片效果；開局時放進 `GameConfig::effects`。
pub fn lookup(id: &str) -> Option<&'static CardDef> {
    hero_powers::lookup(id)
        .or_else(|| basic::lookup(id))
        .or_else(|| minions::lookup(id))
        .or_else(|| secrets::lookup(id))
}
