//! 狀態快照（case S1）與比對。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weapon {
    pub card: String,
    pub attack: i32,
    pub durability: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hero {
    pub card: String,
    pub health: i32,
    pub armor: i32,
    pub attack: i32,
    pub weapon: Option<Weapon>,
    pub power_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mana {
    pub available: i32,
    pub total: i32,
    pub overload_locked: i32,
    pub overload_owed: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Minion {
    pub card: String,
    pub attack: i32,
    pub health: i32,
    pub max_health: i32,
    pub taunt: bool,
    pub divine_shield: bool,
    pub frozen: bool,
    pub stealth: bool,
    pub windfury: bool,
    pub attacks_this_turn: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerState {
    pub hero: Hero,
    pub mana: Mana,
    pub hand: Vec<String>,
    pub board: Vec<Minion>,
    pub deck_count: i32,
    pub secrets: Vec<String>,
    pub fatigue: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub turn: i32,
    pub current_player: u8,
    pub game_over: bool,
    /// 遊戲結束時的勝方（1 或 2）；平手或未結束為 None。
    pub winner: Option<u8>,
    pub players: [PlayerState; 2],
}

/// 第一個不同的步數與不同的欄位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    pub step: usize,
    pub fields: Vec<String>,
}

/// 逐步比對兩串快照，回傳第一個不同之處。
pub fn first_difference(a: &[Snapshot], b: &[Snapshot]) -> Option<Difference> {
    for (step, (x, y)) in a.iter().zip(b).enumerate() {
        let fields = snapshot_fields(x, y);
        if !fields.is_empty() {
            return Some(Difference { step, fields });
        }
    }
    (a.len() != b.len()).then(|| Difference {
        step: a.len().min(b.len()),
        fields: vec!["（快照數量不同）".to_string()],
    })
}

/// 依結構順序列出不同的欄位路徑；清單類欄位（手牌、戰場、秘密）整個算一個欄位。
fn snapshot_fields(a: &Snapshot, b: &Snapshot) -> Vec<String> {
    let mut out = Vec::new();
    let mut cmp = |path: String, same: bool| {
        if !same {
            out.push(path);
        }
    };
    cmp("turn".into(), a.turn == b.turn);
    cmp(
        "current_player".into(),
        a.current_player == b.current_player,
    );
    cmp("game_over".into(), a.game_over == b.game_over);
    cmp("winner".into(), a.winner == b.winner);
    for (i, (p, q)) in a.players.iter().zip(&b.players).enumerate() {
        let at = |f: &str| format!("players[{i}].{f}");
        cmp(at("hero.card"), p.hero.card == q.hero.card);
        cmp(at("hero.health"), p.hero.health == q.hero.health);
        cmp(at("hero.armor"), p.hero.armor == q.hero.armor);
        cmp(at("hero.attack"), p.hero.attack == q.hero.attack);
        cmp(at("hero.weapon"), p.hero.weapon == q.hero.weapon);
        cmp(
            at("hero.power_used"),
            p.hero.power_used == q.hero.power_used,
        );
        cmp(at("mana.available"), p.mana.available == q.mana.available);
        cmp(at("mana.total"), p.mana.total == q.mana.total);
        cmp(
            at("mana.overload_locked"),
            p.mana.overload_locked == q.mana.overload_locked,
        );
        cmp(
            at("mana.overload_owed"),
            p.mana.overload_owed == q.mana.overload_owed,
        );
        cmp(at("hand"), p.hand == q.hand);
        cmp(at("board"), p.board == q.board);
        cmp(at("deck_count"), p.deck_count == q.deck_count);
        cmp(at("secrets"), p.secrets == q.secrets);
        cmp(at("fatigue"), p.fatigue == q.fatigue);
    }
    out
}
