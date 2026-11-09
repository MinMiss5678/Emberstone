//! 第 2 階段 B：指定目標與法術（case B1–B4），以及從第 1 階段搬來的幸運幣（第 1 階段 B2、B2a）。

mod common;

use common::*;
use emberstone_core::{ActError, Action};

/// 戰場：先手 [叢林豹(潛行), 淡水鱷]，後手 [叢林豹(潛行), 淡水鱷]
fn board() -> emberstone_core::Game {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "EX1_017");
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 1, "EX1_017");
    ready(&mut g, 1, "CS2_120");
    g.player_mut(0).mana = 10;
    g
}

// B1：敘述寫「角色」：雙方英雄和雙方手下（自己的英雄也算）；B1c：對手潛行不能選、自己潛行可以
#[test]
fn b1_character_targets() {
    let mut g = board();
    let i = give(&mut g, 0, "CORE_CS2_029"); // 火球術
    assert_eq!(
        play_targets(&g, i),
        sorted(vec![
            Some(hero(0)),
            Some(hero(1)),
            Some(minion(0, 0)),
            Some(minion(0, 1)),
            Some(minion(1, 1)),
        ])
    );
}

// B1a：敘述寫「手下」：只有雙方手下
#[test]
fn b1a_minion_targets() {
    let mut g = board();
    let i = give(&mut g, 0, "CORE_CS1_130"); // 聖光懲擊
    assert_eq!(
        play_targets(&g, i),
        sorted(vec![
            Some(minion(0, 0)),
            Some(minion(0, 1)),
            Some(minion(1, 1)),
        ])
    );
}

// B1b：敘述寫「敵方」：只有對手那一方
#[test]
fn b1b_enemy_minion_targets() {
    let mut g = board();
    let i = give(&mut g, 0, "CORE_CS2_076"); // 暗殺：消滅一個敵方手下
    assert_eq!(play_targets(&g, i), vec![Some(minion(1, 1))]);
}

// B2：一定要目標的牌，沒有合法目標就不能打出
#[test]
fn b2_no_target_cannot_cast() {
    let mut g = start(&[], &[]);
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_CS1_130");
    assert!(play_targets(&g, i).is_empty());
    assert_eq!(g.act(&play(i, None, None)), Err(ActError::Illegal));
}

// B3：法術傷害 +1；多個來源累加；英雄技能、戰吼不加
#[test]
fn b3_spell_damage() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007"); // 見習電擊手：法術傷害 +1
    ready(&mut g, 1, "CS2_186"); // 戰爭魔像 7/7
    cast(&mut g, "CORE_CS2_029", None, Some(minion(1, 0))); // 火球術 6 + 1
    assert!(g.player(1).board.is_empty());
    end(&mut g);
    end(&mut g);
    ready(&mut g, 0, "CS3_007");
    ready(&mut g, 1, "CS2_186");
    cast(&mut g, "CORE_DS1_185", None, Some(minion(1, 0))); // 秘法射擊 2 + 2
    assert_eq!(board_entity(&g, 1, 0).health, 3);
    // 精靈弓箭手的戰吼 1 點，不加
    cast(&mut g, "CORE_CS2_189", Some(2), Some(minion(1, 0)));
    assert_eq!(board_entity(&g, 1, 0).health, 2);
    // 火焰衝擊 1 點，不加
    g.player_mut(0).mana = 10;
    act(&mut g, power(Some(minion(1, 0))));
    assert_eq!(board_entity(&g, 1, 0).health, 1);
}

// B4：傷害法術結算後進入死亡階段，生命 ≤ 0 的手下移出戰場
#[test]
fn b4_spell_kills_after_resolving() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_120");
    ready(&mut g, 1, "CS2_182");
    cast(&mut g, "CORE_CS1_130", None, Some(minion(1, 0))); // 聖光懲擊 3 點
    assert_eq!(board_ids(&g, 1), vec!["CS2_182"]);
    cast(&mut g, "CORE_CS2_029", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 24);
}

// 第 1 階段 B2：幸運幣讓本回合可用法力 +1
#[test]
fn stage1_b2_coin_adds_one_mana_this_turn() {
    let mut g = start(&[], &[]);
    end(&mut g);
    let coin = hand_ids(&g, 1).iter().position(|&c| c == COIN).unwrap();
    act(&mut g, play(coin, None, None));
    assert_eq!((g.player(1).mana, g.player(1).max_mana), (2, 1));
    end(&mut g);
    end(&mut g);
    assert_eq!((g.player(1).mana, g.player(1).max_mana), (2, 2));
}

// 第 1 階段 B2a：10/10 時使用幸運幣 → 11/10
#[test]
fn stage1_b2a_coin_can_exceed_ten() {
    let mut g = start(&[], &[]);
    until_turn(&mut g, 20);
    assert_eq!((g.player(1).mana, g.player(1).max_mana), (10, 10));
    let coin = hand_ids(&g, 1).iter().position(|&c| c == COIN).unwrap();
    act(&mut g, play(coin, None, None));
    assert_eq!((g.player(1).mana, g.player(1).max_mana), (11, 10));
    assert!(!g.legal_actions().iter().any(|a| matches!(
        a,
        Action::Play {
            target: Some(_),
            ..
        }
    )));
}
