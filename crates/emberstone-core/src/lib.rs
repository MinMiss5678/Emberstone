//! 對局狀態與規則：回合流程、法力、戰鬥、事件、觸發、被動加成、附魔、
//! 亂數、合法行動列舉。對局狀態可整份複製，供搜尋 AI 分支使用。
//!
//! 只認得卡片的數值與關鍵字；卡片效果由 `emberstone-cards` 透過 `GameConfig::effects` 提供。

mod effects;
mod game;
mod rng;

pub use effects::{
    Aura, CardDef, Choice, Ctx, Effect, EffectLookup, Sel, TargetReq, Trigger, Usable, When,
};
pub use game::{
    ActError, Action, DefRef, Entity, EntityId, Game, GameConfig, GameError, Outcome, Player,
    PlayerConfig, RngSource, Target,
};
pub use rng::{Desync, Rng, RngEvent};
