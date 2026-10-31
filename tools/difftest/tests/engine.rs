//! 第 1 階段 G2：Emberstone 引擎接到差分測試（不需要橋接程式）。

use emberstone_difftest::actions::{Action, Target};
use emberstone_difftest::bridge::{GameSetup, PlayerSetup, RngMode};
use emberstone_difftest::emberstone;

fn setup() -> GameSetup {
    let deck = |head: &[&str]| -> Vec<String> {
        let mut d: Vec<String> = head.iter().map(|s| s.to_string()).collect();
        while d.len() < 30 {
            d.push("CS2_120".to_string());
        }
        d
    };
    GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CS2_168", "CS2_182", "EX1_008", "CS2_172"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["CS2_179", "CS2_168"]),
        },
        rng: RngMode::Replay { events: vec![] },
    }
}

// G2：開局快照的格式和橋接程式相同（玩家編號 1、2，英雄卡號照職業）
#[test]
fn g2_opening_snapshot() {
    let g = emberstone::new_game(&setup()).expect("開局失敗");
    let s = emberstone::snapshot(&g);
    assert_eq!(
        (s.turn, s.current_player, s.game_over, s.winner),
        (1, 1, false, None)
    );
    let [p1, p2] = &s.players;
    assert_eq!(p1.hero.card, "HERO_08");
    assert_eq!(p2.hero.card, "HERO_01");
    assert_eq!((p1.hero.health, p1.hero.armor, p1.hero.attack), (30, 0, 0));
    assert!(p1.hero.weapon.is_none() && !p1.hero.power_used);
    assert_eq!(p1.hand, vec!["CS2_168", "CS2_182", "EX1_008", "CS2_172"]);
    assert_eq!(
        p2.hand,
        vec!["CS2_179", "CS2_168", "CS2_120", "CS2_120", "GAME_005"]
    );
    assert_eq!((p1.mana.available, p1.mana.total), (1, 1));
    assert_eq!((p1.mana.overload_locked, p1.mana.overload_owed), (0, 0));
    assert_eq!((p1.deck_count, p2.deck_count), (26, 26));
    assert!(p1.board.is_empty() && p1.secrets.is_empty());
    assert_eq!(p1.fatigue, 0);
}

// G2：手下欄位照實填入
#[test]
fn g2_minion_snapshot() {
    let mut g = emberstone::new_game(&setup()).expect("開局失敗");
    let play = |hand| Action::Play {
        hand,
        pos: Some(0),
        target: None,
        choose_one: 0,
    };
    emberstone::act(&mut g, &play(2)).expect("出銀白侍從失敗");
    let s = emberstone::snapshot(&g);
    let m = &s.players[0].board[0];
    assert_eq!(m.card, "EX1_008");
    assert_eq!((m.attack, m.health, m.max_health), (1, 1, 1));
    assert!(m.divine_shield && !m.taunt && !m.frozen && !m.stealth && !m.windfury);
    assert_eq!(m.attacks_this_turn, 0);
    assert_eq!(s.players[0].mana.available, 0);
}

// G2：合法行動用差分測試的格式（玩家 1、2），可以原樣執行
#[test]
fn g2_options_round_trip() {
    let mut g = emberstone::new_game(&setup()).expect("開局失敗");
    let opts = emberstone::options(&g);
    assert!(opts.contains(&Action::End));
    assert!(opts.contains(&Action::Play {
        hand: 0,
        pos: Some(0),
        target: None,
        choose_one: 0
    }));
    emberstone::act(&mut g, &opts[0]).expect("執行合法行動失敗");
    emberstone::act(&mut g, &Action::End).expect("結束回合失敗");
    let coin = Action::Play {
        hand: 4,
        pos: None,
        target: None,
        choose_one: 0,
    };
    assert!(emberstone::options(&g).contains(&coin));
    emberstone::act(&mut g, &coin).expect("用幸運幣失敗");
    assert_eq!(emberstone::snapshot(&g).players[1].mana.available, 2);
    let bad = Action::Attack {
        attacker: Target::Hero { player: 2 },
        target: Target::Hero { player: 1 },
    };
    assert!(emberstone::act(&mut g, &bad).is_err());
}

// G2：不認得的職業回報錯誤
#[test]
fn g2_unknown_class_is_error() {
    let mut s = setup();
    s.p1.class = "DEATHKNIGHT".into();
    assert!(emberstone::new_game(&s).is_err());
}
