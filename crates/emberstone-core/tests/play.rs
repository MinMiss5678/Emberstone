//! 第 1 階段 C：出牌（case C1–C4）。

mod common;

use common::*;
use emberstone_core::{ActError, Action};

// C1：費用不超過可用水晶才能出；扣水晶、放到指定位置，右邊的手下往右移
#[test]
fn c1_play_minion_at_position() {
    let mut g = start(&["CS2_182"], &[]);
    ready(&mut g, 0, "CS2_168");
    ready(&mut g, 0, "CS2_172");
    // 第 1 回合只有 1 顆水晶，冰風雪人（4 費）不能出
    let yeti = hand_index(&g, 0, "CS2_182");
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Play { hand, .. } if *hand == yeti))
    );
    assert_eq!(g.act(&play(yeti, Some(1))), Err(ActError::Illegal));
    g.player_mut(0).mana = 4;
    act(&mut g, play(yeti, Some(1)));
    assert_eq!(board_ids(&g, 0), vec!["CS2_168", "CS2_182", "CS2_172"]);
    assert_eq!(g.player(0).mana, 0);
    assert_eq!(g.player(0).hand.len(), 3);
}

// C2：戰場已有 7 隻就不能出手下
#[test]
fn c2_board_full() {
    let mut g = start(&["CS2_168"], &[]);
    for _ in 0..7 {
        ready(&mut g, 0, "CS2_120");
    }
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Play { .. }))
    );
    assert_eq!(g.act(&play(0, Some(7))), Err(ActError::Illegal));
}

// C3：剛出場當回合不能攻擊；衝鋒可以立刻攻擊；衝刺當回合只能攻擊手下
#[test]
fn c3_summoning_sickness_charge_rush() {
    let mut g = start(&["CS2_182", "CS2_124", "CS3_038"], &[]);
    ready(&mut g, 1, "CS2_120");
    g.player_mut(0).mana = 10;
    for (id, pos) in [("CS2_182", 0), ("CS2_124", 1), ("CS3_038", 2)] {
        let i = hand_index(&g, 0, id);
        act(&mut g, play(i, Some(pos)));
    }
    assert_eq!(board_ids(&g, 0), vec!["CS2_182", "CS2_124", "CS3_038"]);
    assert!(attack_targets(&g, minion(0, 0)).is_empty());
    let mut charge = attack_targets(&g, minion(0, 1));
    charge.sort_by_key(|t| format!("{t:?}"));
    assert_eq!(charge, vec![hero(1), minion(1, 0)]);
    assert_eq!(attack_targets(&g, minion(0, 2)), vec![minion(1, 0)]);
    assert_eq!(
        g.act(&attack(minion(0, 2), hero(1))),
        Err(ActError::Illegal)
    );
}

// C4：出武器（奧達奇戰刃 3 費 2/2）→ 扣 3 點法力，英雄裝上武器
#[test]
fn c4_play_weapon() {
    let mut g = start(&["CORE_BT_921"], &[]);
    g.player_mut(0).mana = 3;
    let weapon = hand_index(&g, 0, "CORE_BT_921");
    act(&mut g, play(weapon, None));
    assert_eq!(g.player(0).mana, 0);
    let w = g.player(0).weapon.expect("應該裝上武器");
    let w = g.entity(w);
    assert_eq!(w.card.id, "CORE_BT_921");
    assert_eq!((w.attack, w.durability), (2, 2));
    assert_eq!(g.hero_attack(0), 2);
}
