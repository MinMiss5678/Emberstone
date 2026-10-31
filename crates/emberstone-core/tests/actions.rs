//! 第 1 階段 F：合法行動（case F1、F2）。

mod common;

use common::*;
use emberstone_core::{ActError, Action};

fn sorted(mut v: Vec<Action>) -> Vec<Action> {
    v.sort_by_key(|a| format!("{a:?}"));
    v
}

// F1：第 1 回合只有 1 顆水晶：只能出 1 費手下（放在 0）或結束回合
#[test]
fn f1_opening_actions() {
    let g = start(&["CS2_182", "CS2_168", "CS2_172", "CS2_118"], &[]);
    assert_eq!(
        sorted(g.legal_actions()),
        sorted(vec![play(1, Some(0)), Action::End])
    );
}

// F1：手下附上所有可放的位置；所有可攻擊的組合；不需要目標的牌 pos 為空
#[test]
fn f1_plays_attacks_and_end() {
    let mut g = start(&["CS2_168", "CS2_106", "CS2_182", "CS2_182"], &[]);
    ready(&mut g, 0, "CS2_182");
    ready(&mut g, 1, "CS2_120");
    g.player_mut(0).mana = 2;
    let expected = vec![
        play(0, Some(0)),
        play(0, Some(1)),
        play(1, None),
        attack(minion(0, 0), hero(1)),
        attack(minion(0, 0), minion(1, 0)),
        Action::End,
    ];
    assert_eq!(sorted(g.legal_actions()), sorted(expected));
}

// F2：不合法的行動回報錯誤，狀態不變
#[test]
fn f2_illegal_actions_leave_state_unchanged() {
    let mut g = start(&["CS2_182", "CS2_168"], &[]);
    let sick = g.summon(0, "CS2_168", 0).expect("放不上戰場");
    assert!(g.entity(sick).summoning_sick);
    let before = g.clone();
    for a in [
        play(0, Some(0)),              // 費用不夠
        play(9, Some(0)),              // 手牌沒有第 9 張
        play(1, Some(5)),              // 位置超出範圍
        play(1, None),                 // 手下沒有指定位置
        attack(minion(0, 0), hero(1)), // 剛出場
        attack(minion(1, 0), hero(0)), // 對手沒有手下
        attack(hero(0), hero(1)),      // 英雄沒有攻擊力
        Action::Power { target: None },
        Action::Choose { index: 0 },
    ] {
        assert_eq!(g.act(&a), Err(ActError::Illegal), "{a:?}");
        assert_eq!(g, before, "{a:?} 改變了狀態");
    }
}
