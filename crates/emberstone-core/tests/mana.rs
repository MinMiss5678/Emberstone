//! 第 1 階段 B：回合與法力（case B1–B6）。

mod common;

use common::*;
use emberstone_core::Game;

fn mana(g: &Game, player: usize) -> (i32, i32) {
    let p = g.player(player);
    (p.mana, p.max_mana)
}

// B1：回合開始水晶上限 +1（最多 10）、補滿、抽 1 張
#[test]
fn b1_turn_start_gains_crystal_and_draws() {
    let mut g = start(&[], &[]);
    end(&mut g);
    assert_eq!((g.turn(), g.current_player()), (2, 1));
    assert_eq!(mana(&g, 1), (1, 1));
    assert_eq!(g.player(1).hand.len(), 6);
    assert_eq!(g.player(1).deck.len(), 25);
    end(&mut g);
    assert_eq!((g.turn(), g.current_player()), (3, 0));
    assert_eq!(mana(&g, 0), (2, 2));
    assert_eq!(g.player(0).hand.len(), 5);
    assert_eq!(g.player(0).deck.len(), 25);
    // 先手第 10 個回合是第 19 回合；第 21 回合仍是 10
    while g.turn() < 19 {
        end(&mut g);
    }
    assert_eq!(mana(&g, 0), (10, 10));
    end(&mut g);
    end(&mut g);
    assert_eq!(g.turn(), 21);
    assert_eq!(mana(&g, 0), (10, 10));
}

// B2：幸運幣讓本回合可用法力 +1；下回合照常補滿到上限
#[test]
fn b2_coin_adds_one_mana_this_turn() {
    let mut g = start(&[], &[]);
    end(&mut g);
    let coin = hand_index(&g, 1, COIN);
    act(&mut g, play(coin, None));
    assert_eq!(mana(&g, 1), (2, 1));
    assert!(!hand_ids(&g, 1).contains(&COIN));
    end(&mut g);
    end(&mut g);
    assert_eq!(mana(&g, 1), (2, 2));
}

// B2a：10/10 時使用幸運幣 → 11/10
#[test]
fn b2a_coin_can_exceed_ten() {
    let mut g = start(&[], &[]);
    while g.turn() < 20 {
        end(&mut g);
    }
    assert_eq!(g.current_player(), 1);
    assert_eq!(mana(&g, 1), (10, 10));
    let coin = hand_index(&g, 1, COIN);
    act(&mut g, play(coin, None));
    assert_eq!(mana(&g, 1), (11, 10));
}

// B3：超載 1 → 下個回合鎖 1 顆，再下個回合解除
#[test]
fn b3_overload_locks_next_turn_only() {
    let mut g = start(&["AT_052"], &[]);
    end(&mut g);
    end(&mut g);
    assert_eq!(mana(&g, 0), (2, 2));
    let golem = hand_index(&g, 0, "AT_052");
    act(&mut g, play(golem, Some(0)));
    assert_eq!(mana(&g, 0), (0, 2));
    assert_eq!(g.player(0).overload_owed, 1);
    end(&mut g);
    end(&mut g);
    let p = g.player(0);
    assert_eq!(
        (p.mana, p.max_mana, p.overload_locked, p.overload_owed),
        (2, 3, 1, 0)
    );
    end(&mut g);
    end(&mut g);
    let p = g.player(0);
    assert_eq!((p.mana, p.max_mana, p.overload_locked), (4, 4, 0));
}

// B4：手牌 10 張時抽牌 → 抽到的牌摧毀，牌庫 −1
#[test]
fn b4_overdraw_burns_card() {
    let mut g = start(&[], &[]);
    for _ in 0..6 {
        g.add_to_hand(0, "CS2_182").expect("加不進手牌");
    }
    assert_eq!(g.player(0).hand.len(), 10);
    let hand_before = g.player(0).hand.clone();
    end(&mut g);
    end(&mut g);
    assert_eq!(g.player(0).hand, hand_before);
    assert_eq!(g.player(0).deck.len(), 25);
}

// B5：牌庫空了抽牌 → 疲勞 1、2、……，雙方各自累計
#[test]
fn b5_fatigue_increases_per_player() {
    let mut g = Game::new(config(deck(&[], 4), deck(&[], 5))).expect("開局失敗");
    assert!(g.player(0).deck.is_empty());
    end(&mut g);
    assert!(g.player(1).deck.is_empty());
    end(&mut g);
    assert_eq!((hero_health(&g, 0), g.player(0).fatigue), (29, 1));
    end(&mut g);
    assert_eq!((hero_health(&g, 1), g.player(1).fatigue), (29, 1));
    end(&mut g);
    assert_eq!((hero_health(&g, 0), g.player(0).fatigue), (27, 2));
    assert_eq!((hero_health(&g, 1), g.player(1).fatigue), (29, 1));
}

// B6：回合結束換對方；攻擊次數在擁有者的回合開始時重置
#[test]
fn b6_end_turn_switches_and_resets_attacks() {
    let mut g = start(&[], &[]);
    let i = ready(&mut g, 0, "CS2_182");
    act(&mut g, attack(minion(0, i), hero(1)));
    assert_eq!(board_entity(&g, 0, i).attacks_this_turn, 1);
    assert!(attack_targets(&g, minion(0, i)).is_empty());
    end(&mut g);
    assert_eq!(g.current_player(), 1);
    end(&mut g);
    assert_eq!(board_entity(&g, 0, i).attacks_this_turn, 0);
    assert!(attack_targets(&g, minion(0, i)).contains(&hero(1)));
}
