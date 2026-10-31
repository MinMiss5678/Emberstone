//! 規則測試共用的開局與行動輔助函式。

#![allow(dead_code)]

use emberstone_core::{Action, Game, GameConfig, PlayerConfig, RngSource, Target};

/// 淡水鱷（2 費 2/3），補滿牌組用。
pub const FILL: &str = "CS2_120";
pub const MAGE: &str = "HERO_08";
pub const WARRIOR: &str = "HERO_01";
pub const COIN: &str = "GAME_005";

/// 前幾張固定，其餘用淡水鱷補到 len 張（照抽牌順序）。
pub fn deck(head: &[&'static str], len: usize) -> Vec<&'static str> {
    let mut d = head.to_vec();
    while d.len() < len {
        d.push(FILL);
    }
    d
}

pub fn config(p1: Vec<&'static str>, p2: Vec<&'static str>) -> GameConfig {
    GameConfig {
        players: [
            PlayerConfig {
                hero: MAGE,
                deck: p1,
            },
            PlayerConfig {
                hero: WARRIOR,
                deck: p2,
            },
        ],
        shuffle: false,
        rng: RngSource::Seed(1),
    }
}

/// 法師對戰士，各 30 張、不洗牌。
pub fn start(p1: &[&'static str], p2: &[&'static str]) -> Game {
    Game::new(config(deck(p1, 30), deck(p2, 30))).expect("開局失敗")
}

pub fn hero(player: usize) -> Target {
    Target::Hero { player }
}

pub fn minion(player: usize, index: usize) -> Target {
    Target::Minion { player, index }
}

pub fn play(hand: usize, pos: Option<usize>) -> Action {
    Action::Play {
        hand,
        pos,
        target: None,
        choose_one: 0,
    }
}

pub fn attack(attacker: Target, target: Target) -> Action {
    Action::Attack { attacker, target }
}

pub fn act(g: &mut Game, a: Action) {
    g.act(&a)
        .unwrap_or_else(|e| panic!("行動 {a:?} 失敗：{e:?}"));
}

pub fn end(g: &mut Game) {
    act(g, Action::End);
}

/// 手牌中第一張指定卡的位置。
pub fn hand_index(g: &Game, player: usize, id: &str) -> usize {
    g.player(player)
        .hand
        .iter()
        .position(|&e| g.entity(e).card.id == id)
        .unwrap_or_else(|| panic!("手牌裡沒有 {id}"))
}

/// 手牌的卡號。
pub fn hand_ids(g: &Game, player: usize) -> Vec<&'static str> {
    g.player(player)
        .hand
        .iter()
        .map(|&e| g.entity(e).card.id)
        .collect()
}

/// 戰場的卡號。
pub fn board_ids(g: &Game, player: usize) -> Vec<&'static str> {
    g.player(player)
        .board
        .iter()
        .map(|&e| g.entity(e).card.id)
        .collect()
}

/// 在戰場最右邊放一隻可以立刻攻擊的手下，回傳它在戰場的位置。
pub fn ready(g: &mut Game, player: usize, id: &'static str) -> usize {
    let pos = g.player(player).board.len();
    let e = g.summon(player, id, pos).expect("放不上戰場");
    g.entity_mut(e).summoning_sick = false;
    pos
}

/// 第 index 隻手下（實體）。
pub fn board_entity(g: &Game, player: usize, index: usize) -> &emberstone_core::Entity {
    g.entity(g.player(player).board[index])
}

/// 目前玩家合法的攻擊目標（某個攻擊者）。
pub fn attack_targets(g: &Game, attacker: Target) -> Vec<Target> {
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Attack {
                attacker: who,
                target,
            } if who == attacker => Some(target),
            _ => None,
        })
        .collect()
}

pub fn hero_health(g: &Game, player: usize) -> i32 {
    g.entity(g.player(player).hero).health
}
