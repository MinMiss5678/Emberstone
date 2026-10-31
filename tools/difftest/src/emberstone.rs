//! 把 Emberstone 引擎接到差分測試：開局、行動與快照都用橋接程式的格式。

use emberstone_core as core;
use emberstone_core::{Game, GameConfig, PlayerConfig, RngSource};
use emberstone_data::card_by_id;

use crate::actions::{Action, Target};
use crate::bridge::{GameSetup, PlayerSetup, RngMode};
use crate::rng::RngEvent;
use crate::snapshot::{Hero, Mana, Minion, PlayerState, Snapshot, Weapon};

/// 職業的預設英雄（和 RosettaStone 相同）。
fn hero_card(class: &str) -> Option<&'static str> {
    Some(match class {
        "WARRIOR" => "HERO_01",
        "SHAMAN" => "HERO_02",
        "ROGUE" => "HERO_03",
        "PALADIN" => "HERO_04",
        "HUNTER" => "HERO_05",
        "DRUID" => "HERO_06",
        "WARLOCK" => "HERO_07",
        "MAGE" => "HERO_08",
        "PRIEST" => "HERO_09",
        "DEMONHUNTER" => "HERO_10",
        _ => return None,
    })
}

fn player_config(p: &PlayerSetup) -> Result<PlayerConfig, String> {
    let hero = hero_card(&p.class).ok_or_else(|| format!("不明的職業 {}", p.class))?;
    let deck = p
        .deck
        .iter()
        .map(|id| {
            card_by_id(id)
                .map(|c| c.id)
                .ok_or_else(|| format!("卡表沒有 {id}"))
        })
        .collect::<Result<_, _>>()?;
    Ok(PlayerConfig { hero, deck })
}

fn to_core_event(e: &RngEvent) -> core::RngEvent {
    match e {
        RngEvent::Int { lo, hi, value } => core::RngEvent::Int {
            lo: *lo,
            hi: *hi,
            value: *value,
        },
        RngEvent::Float { lo, hi, value } => core::RngEvent::Float {
            lo: *lo,
            hi: *hi,
            value: *value,
        },
        RngEvent::Shuffle { n, perm } => core::RngEvent::Shuffle {
            n: *n,
            perm: perm.clone(),
        },
    }
}

/// 照橋接程式的開局設定建立對局（不洗牌；記錄模式用種子，重播模式照事件序列）。
pub fn new_game(setup: &GameSetup) -> Result<Game, String> {
    let rng = match &setup.rng {
        RngMode::Record { seed } => RngSource::Seed(*seed),
        RngMode::Replay { events } => {
            RngSource::Scripted(events.iter().map(to_core_event).collect())
        }
    };
    Game::new(GameConfig {
        players: [player_config(&setup.p1)?, player_config(&setup.p2)?],
        shuffle: false,
        rng,
    })
    .map_err(|e| format!("開局失敗：{e:?}"))
}

fn to_core_target(t: &Target) -> Option<core::Target> {
    Some(match *t {
        Target::Hero { player } => core::Target::Hero {
            player: usize::from(player.checked_sub(1)?),
        },
        Target::Minion { player, index } => core::Target::Minion {
            player: usize::from(player.checked_sub(1)?),
            index,
        },
    })
}

fn from_core_target(t: &core::Target) -> Target {
    match *t {
        core::Target::Hero { player } => Target::Hero {
            player: player as u8 + 1,
        },
        core::Target::Minion { player, index } => Target::Minion {
            player: player as u8 + 1,
            index,
        },
    }
}

fn to_core(a: &Action) -> Option<core::Action> {
    Some(match a {
        Action::Play {
            hand,
            pos,
            target,
            choose_one,
        } => core::Action::Play {
            hand: *hand,
            pos: *pos,
            target: match target {
                Some(t) => Some(to_core_target(t)?),
                None => None,
            },
            choose_one: *choose_one,
        },
        Action::Attack { attacker, target } => core::Action::Attack {
            attacker: to_core_target(attacker)?,
            target: to_core_target(target)?,
        },
        Action::Power { target } => core::Action::Power {
            target: match target {
                Some(t) => Some(to_core_target(t)?),
                None => None,
            },
        },
        Action::Choose { index } => core::Action::Choose { index: *index },
        Action::End => core::Action::End,
    })
}

fn from_core(a: &core::Action) -> Action {
    match a {
        core::Action::Play {
            hand,
            pos,
            target,
            choose_one,
        } => Action::Play {
            hand: *hand,
            pos: *pos,
            target: target.as_ref().map(from_core_target),
            choose_one: *choose_one,
        },
        core::Action::Attack { attacker, target } => Action::Attack {
            attacker: from_core_target(attacker),
            target: from_core_target(target),
        },
        core::Action::Power { target } => Action::Power {
            target: target.as_ref().map(from_core_target),
        },
        core::Action::Choose { index } => Action::Choose { index: *index },
        core::Action::End => Action::End,
    }
}

fn player_state(g: &Game, p: usize) -> PlayerState {
    let me = g.player(p);
    let hero = g.entity(me.hero);
    let weapon = me.weapon.map(|w| {
        let w = g.entity(w);
        Weapon {
            card: w.card.id.to_string(),
            attack: w.attack,
            durability: w.durability,
        }
    });
    let board = me
        .board
        .iter()
        .map(|&m| {
            let e = g.entity(m);
            Minion {
                card: e.card.id.to_string(),
                attack: e.attack,
                health: e.health,
                max_health: e.max_health,
                taunt: e.taunt,
                divine_shield: e.divine_shield,
                frozen: e.frozen,
                stealth: e.stealth,
                windfury: e.windfury,
                attacks_this_turn: e.attacks_this_turn,
            }
        })
        .collect();
    PlayerState {
        hero: Hero {
            card: hero.card.id.to_string(),
            health: hero.health,
            armor: hero.armor,
            attack: g.hero_attack(p),
            weapon,
            power_used: false,
        },
        mana: Mana {
            available: me.mana,
            total: me.max_mana,
            overload_locked: me.overload_locked,
            overload_owed: me.overload_owed,
        },
        hand: me
            .hand
            .iter()
            .map(|&e| g.entity(e).card.id.to_string())
            .collect(),
        board,
        deck_count: me.deck.len() as i32,
        secrets: Vec::new(),
        fatigue: me.fatigue,
    }
}

/// 目前狀態的快照。
pub fn snapshot(game: &Game) -> Snapshot {
    let outcome = game.outcome();
    Snapshot {
        turn: game.turn() as i32,
        current_player: game.current_player() as u8 + 1,
        game_over: outcome.is_some(),
        winner: match outcome {
            Some(core::Outcome::Won(p)) => Some(p as u8 + 1),
            _ => None,
        },
        players: [player_state(game, 0), player_state(game, 1)],
    }
}

/// 目前玩家的合法行動。
pub fn options(game: &Game) -> Vec<Action> {
    game.legal_actions().iter().map(from_core).collect()
}

/// 執行行動。
pub fn act(game: &mut Game, action: &Action) -> Result<(), String> {
    let a = to_core(action).ok_or_else(|| format!("行動格式錯誤：{action:?}"))?;
    game.act(&a).map_err(|e| format!("{e:?}"))
}
