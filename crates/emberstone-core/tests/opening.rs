//! 第 1 階段 A：開局與可重現性（case A1–A4）。

mod common;

use common::*;
use emberstone_core::{Game, GameError, Rng, RngEvent, RngSource};

// A1：先手抽 3 張、後手抽 4 張加幸運幣；先手第 1 回合水晶 1/1，再抽 1 張
#[test]
fn a1_opening_draws_and_coin() {
    let g = start(
        &["CS2_168", "CS2_182", "CS2_172", "CS2_118", "CS2_179"],
        &["CS2_182", "CS2_168", "CS2_172", "CS2_118", "CS2_179"],
    );
    assert_eq!(g.turn(), 1);
    assert_eq!(g.current_player(), 0);
    assert_eq!(
        hand_ids(&g, 0),
        vec!["CS2_168", "CS2_182", "CS2_172", "CS2_118"]
    );
    assert_eq!(
        hand_ids(&g, 1),
        vec!["CS2_182", "CS2_168", "CS2_172", "CS2_118", COIN]
    );
    let p1 = g.player(0);
    assert_eq!((p1.mana, p1.max_mana), (1, 1));
    let p2 = g.player(1);
    assert_eq!((p2.mana, p2.max_mana), (0, 0));
    assert_eq!((p1.deck.len(), p2.deck.len()), (26, 26));
    assert_eq!(hero_health(&g, 0), 30);
    assert_eq!(g.entity(p1.hero).card.id, MAGE);
    assert_eq!(g.entity(p2.hero).card.id, WARRIOR);
    assert!(g.outcome().is_none());
}

fn shuffled(seed: u64) -> Game {
    let mut cfg = config(
        deck(&["CS2_168", "CS2_182", "CS2_172", "CS2_118", "CS2_179"], 30),
        deck(&["CS2_131", "EX1_008", "EX1_170", "EX1_017", "CS2_124"], 30),
    );
    cfg.shuffle = true;
    cfg.rng = RngSource::Seed(seed);
    Game::new(cfg).expect("開局失敗")
}

// A2：同牌組、同種子、同一串行動 → 每一步狀態完全相同
#[test]
fn a2_same_seed_same_actions_same_states() {
    let mut a = shuffled(42);
    let mut b = shuffled(42);
    assert_eq!(a, b);
    for step in 0..300 {
        let opts = a.legal_actions();
        if opts.is_empty() {
            break;
        }
        assert_eq!(opts, b.legal_actions());
        let pick = opts[(step * 7 + 3) % opts.len()].clone();
        act(&mut a, pick.clone());
        act(&mut b, pick);
        assert_eq!(a, b, "第 {step} 步不同");
    }
}

// A3：複製後在複本上行動，原本的對局不受影響
#[test]
fn a3_clone_is_independent() {
    let mut g = start(&["CS2_168"], &[]);
    let before = g.clone();
    let mut copy = g.clone();
    act(&mut copy, play(0, Some(0)));
    end(&mut copy);
    assert_eq!(g, before);
    assert_ne!(copy, before);
    // 原本的對局照樣可以進行
    act(&mut g, play(0, Some(0)));
    assert_eq!(board_ids(&g, 0), vec!["CS2_168"]);
}

// A4：照記錄序列回傳；類型或範圍不符就回報不同步
#[test]
fn a4_scripted_rng_returns_recorded_results() {
    let mut rng = Rng::scripted(vec![
        RngEvent::Int {
            lo: 0,
            hi: 3,
            value: 2,
        },
        RngEvent::Shuffle {
            n: 3,
            perm: vec![2, 0, 1],
        },
    ]);
    assert_eq!(rng.int(0, 3), Ok(2));
    assert_eq!(rng.shuffle(3), Ok(vec![2, 0, 1]));
}

#[test]
fn a4_scripted_rng_reports_desync() {
    let mut rng = Rng::scripted(vec![RngEvent::Int {
        lo: 0,
        hi: 3,
        value: 2,
    }]);
    let e = rng.int(0, 4).expect_err("範圍不符應該不同步");
    assert_eq!(e.index, 0);
    let e = rng.shuffle(4).expect_err("類型不符應該不同步");
    assert_eq!(e.index, 0);
    assert_eq!(rng.int(0, 3), Ok(2));
    let e = rng.int(0, 3).expect_err("序列用完應該不同步");
    assert_eq!((e.index, e.expected), (1, None));
}

// A4：開局洗牌照記錄的排列（洗牌後第 i 張是原本的第 perm[i] 張；先洗先手再洗後手）
#[test]
fn a4_game_shuffle_follows_script() {
    let rotate: Vec<usize> = (0..30).map(|i| (i + 1) % 30).collect();
    let identity: Vec<usize> = (0..30).collect();
    let mut cfg = config(deck(&["CS2_182"], 30), deck(&["CS2_168"], 30));
    cfg.shuffle = true;
    cfg.rng = RngSource::Scripted(vec![
        RngEvent::Shuffle {
            n: 30,
            perm: rotate,
        },
        RngEvent::Shuffle {
            n: 30,
            perm: identity,
        },
    ]);
    let g = Game::new(cfg).expect("開局失敗");
    assert!(!hand_ids(&g, 0).contains(&"CS2_182"));
    let last = *g.player(0).deck.last().unwrap();
    assert_eq!(g.entity(last).card.id, "CS2_182");
    assert_eq!(hand_ids(&g, 1)[0], "CS2_168");
}

#[test]
fn a4_game_stops_on_desync() {
    let mut cfg = config(deck(&[], 30), deck(&[], 30));
    cfg.shuffle = true;
    cfg.rng = RngSource::Scripted(vec![RngEvent::Int {
        lo: 0,
        hi: 1,
        value: 0,
    }]);
    match Game::new(cfg) {
        Err(GameError::Desync(d)) => assert_eq!(d.index, 0),
        other => panic!("應該回報不同步：{:?}", other.map(|_| ())),
    }
}
