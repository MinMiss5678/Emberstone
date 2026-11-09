//! 卡片效果測試共用的開局與行動輔助函式（效果由 emberstone-cards 提供）。

#![allow(dead_code)]

use emberstone_core::{
    Action, Entity, Game, GameConfig, PlayerConfig, RngEvent, RngSource, Target,
};

/// 淡水鱷（2 費 2/3），補滿牌組用。
pub const FILL: &str = "CS2_120";
pub const WARRIOR: &str = "HERO_01";
pub const SHAMAN: &str = "HERO_02";
pub const ROGUE: &str = "HERO_03";
pub const PALADIN: &str = "HERO_04";
pub const HUNTER: &str = "HERO_05";
pub const DRUID: &str = "HERO_06";
pub const WARLOCK: &str = "HERO_07";
pub const MAGE: &str = "HERO_08";
pub const PRIEST: &str = "HERO_09";
pub const DEMON_HUNTER: &str = "HERO_10";
pub const COIN: &str = "GAME_005";

/// 前幾張固定，其餘用淡水鱷補到 len 張（照抽牌順序）。
pub fn deck(head: &[&'static str], len: usize) -> Vec<&'static str> {
    let mut d = head.to_vec();
    while d.len() < len {
        d.push(FILL);
    }
    d
}

pub fn config(
    p1_hero: &'static str,
    p1_deck: Vec<&'static str>,
    p2_hero: &'static str,
    p2_deck: Vec<&'static str>,
) -> GameConfig {
    GameConfig {
        players: [
            PlayerConfig {
                hero: p1_hero,
                deck: p1_deck,
            },
            PlayerConfig {
                hero: p2_hero,
                deck: p2_deck,
            },
        ],
        shuffle: false,
        rng: RngSource::Seed(1),
        effects: Some(emberstone_cards::lookup),
    }
}

/// 指定雙方英雄，牌組各 30 張、不洗牌。
pub fn start_with(
    p1_hero: &'static str,
    p1: &[&'static str],
    p2_hero: &'static str,
    p2: &[&'static str],
) -> Game {
    Game::new(config(p1_hero, deck(p1, 30), p2_hero, deck(p2, 30))).expect("開局失敗")
}

/// 法師對戰士。
pub fn start(p1: &[&'static str], p2: &[&'static str]) -> Game {
    start_with(MAGE, p1, WARRIOR, p2)
}

/// 亂數照記錄序列回傳的對局。
pub fn start_scripted(p1_hero: &'static str, p2_hero: &'static str, events: Vec<RngEvent>) -> Game {
    let mut cfg = config(p1_hero, deck(&[], 30), p2_hero, deck(&[], 30));
    cfg.rng = RngSource::Scripted(events);
    Game::new(cfg).expect("開局失敗")
}

pub fn hero(player: usize) -> Target {
    Target::Hero { player }
}

pub fn minion(player: usize, index: usize) -> Target {
    Target::Minion { player, index }
}

pub fn play(hand: usize, pos: Option<usize>, target: Option<Target>) -> Action {
    Action::Play {
        hand,
        pos,
        target,
        choose_one: 0,
    }
}

pub fn choose_one(hand: usize, pos: Option<usize>, target: Option<Target>, which: u8) -> Action {
    Action::Play {
        hand,
        pos,
        target,
        choose_one: which,
    }
}

pub fn power(target: Option<Target>) -> Action {
    Action::Power { target }
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

/// 結束回合直到第 turn 回合。
pub fn until_turn(g: &mut Game, turn: u32) {
    while g.turn() < turn {
        end(g);
    }
}

/// 加一張卡到手牌並回傳它在手牌的位置。
pub fn give(g: &mut Game, player: usize, id: &'static str) -> usize {
    g.add_to_hand(player, id).expect("加不進手牌");
    g.player(player).hand.len() - 1
}

/// 手牌加一張卡、補足法力後打出（手下放在 pos）。
pub fn cast(g: &mut Game, id: &'static str, pos: Option<usize>, target: Option<Target>) {
    let p = g.current_player();
    let i = give(g, p, id);
    g.player_mut(p).mana = 10;
    act(g, play(i, pos, target));
}

pub fn hand_ids(g: &Game, player: usize) -> Vec<&'static str> {
    g.player(player)
        .hand
        .iter()
        .map(|&e| g.entity(e).card.id)
        .collect()
}

pub fn board_ids(g: &Game, player: usize) -> Vec<&'static str> {
    g.player(player)
        .board
        .iter()
        .map(|&e| g.entity(e).card.id)
        .collect()
}

/// 戰場上每隻手下的「攻擊力/生命」。
pub fn board_stats(g: &Game, player: usize) -> Vec<(i32, i32)> {
    g.player(player)
        .board
        .iter()
        .map(|&e| (g.entity(e).attack, g.entity(e).health))
        .collect()
}

/// 在戰場最右邊放一隻可以立刻攻擊的手下（不觸發戰吼），回傳位置。
pub fn ready(g: &mut Game, player: usize, id: &'static str) -> usize {
    let pos = g.player(player).board.len();
    let e = g.summon(player, id, pos).expect("放不上戰場");
    g.entity_mut(e).summoning_sick = false;
    pos
}

pub fn board_entity(g: &Game, player: usize, index: usize) -> &Entity {
    g.entity(g.player(player).board[index])
}

pub fn hero_entity(g: &Game, player: usize) -> &Entity {
    g.entity(g.player(player).hero)
}

pub fn hero_health(g: &Game, player: usize) -> i32 {
    hero_entity(g, player).health
}

pub fn set_hero_health(g: &mut Game, player: usize, health: i32) {
    let h = g.player(player).hero;
    g.entity_mut(h).health = health;
}

/// 合法行動中，某張手牌（或英雄技能）可以選的目標。
pub fn play_targets(g: &Game, hand: usize) -> Vec<Option<Target>> {
    let mut v: Vec<Option<Target>> = g
        .legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Play {
                hand: h, target, ..
            } if h == hand => Some(target),
            _ => None,
        })
        .collect();
    v.sort_by_key(|t| format!("{t:?}"));
    v.dedup();
    v
}

pub fn power_targets(g: &Game) -> Vec<Option<Target>> {
    let mut v: Vec<Option<Target>> = g
        .legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Power { target } => Some(target),
            _ => None,
        })
        .collect();
    v.sort_by_key(|t| format!("{t:?}"));
    v
}

pub fn sorted(mut v: Vec<Option<Target>>) -> Vec<Option<Target>> {
    v.sort_by_key(|t| format!("{t:?}"));
    v
}
