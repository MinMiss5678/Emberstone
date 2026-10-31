//! 第 1 階段 E：英雄、武器、護甲、勝負（case E1–E5）。

mod common;

use common::*;
use emberstone_core::{ActError, Game, Outcome};

/// 先手裝上一把武器（直接加進手牌再打出）。
fn equip(g: &mut Game, id: &'static str) {
    g.add_to_hand(0, id).expect("加不進手牌");
    g.player_mut(0).mana = 10;
    let i = hand_index(g, 0, id);
    act(g, play(i, None));
}

// E1：英雄攻擊力＝武器攻擊力；每攻擊一次耐久 −1，歸零時武器摧毀
#[test]
fn e1_hero_attacks_with_weapon() {
    let mut g = start(&[], &[]);
    equip(&mut g, "CS2_106"); // 熾炎戰斧 3/2
    assert_eq!(g.hero_attack(0), 3);
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 27);
    let w = g.player(0).weapon.expect("武器還在");
    assert_eq!(g.entity(w).durability, 1);
    assert!(attack_targets(&g, hero(0)).is_empty());
    end(&mut g);
    end(&mut g);
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 24);
    assert!(g.player(0).weapon.is_none());
    assert_eq!(g.hero_attack(0), 0);
}

// E2：英雄攻擊手下時也受到反擊傷害
#[test]
fn e2_hero_takes_counter_damage() {
    let mut g = start(&[], &[]);
    equip(&mut g, "CS2_106");
    ready(&mut g, 1, "CS2_120"); // 2/3
    act(&mut g, attack(hero(0), minion(1, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 0), 28);
}

// E3：已有武器時裝新武器，舊武器被摧毀
#[test]
fn e3_new_weapon_replaces_old() {
    let mut g = start(&[], &[]);
    equip(&mut g, "CS2_106");
    let old = g.player(0).weapon.unwrap();
    equip(&mut g, "CS2_112"); // 奧金收割斧 5/2
    let new = g.player(0).weapon.unwrap();
    assert_ne!(old, new);
    assert_eq!(g.entity(new).card.id, "CS2_112");
    assert_eq!(g.hero_attack(0), 5);
}

// E4：先扣護甲，不夠才扣生命
#[test]
fn e4_armor_absorbs_first() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182"); // 4/5
    let h = g.player(1).hero;
    g.entity_mut(h).armor = 5;
    act(&mut g, attack(minion(0, yeti), hero(1)));
    assert_eq!((g.entity(h).armor, g.entity(h).health), (1, 30));
    end(&mut g);
    end(&mut g);
    act(&mut g, attack(minion(0, yeti), hero(1)));
    assert_eq!((g.entity(h).armor, g.entity(h).health), (0, 27));
}

// E5：英雄生命歸零 → 遊戲結束、對方獲勝
#[test]
fn e5_hero_death_ends_game() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182");
    let h = g.player(1).hero;
    g.entity_mut(h).health = 3;
    act(&mut g, attack(minion(0, yeti), hero(1)));
    assert_eq!(g.outcome(), Some(Outcome::Won(0)));
    assert!(g.legal_actions().is_empty());
    assert_eq!(
        g.act(&emberstone_core::Action::End),
        Err(ActError::GameOver)
    );
}

// E5：雙方同時歸零為平手
#[test]
fn e5_both_heroes_dead_is_draw() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182");
    let mine = g.player(0).hero;
    g.entity_mut(mine).health = 0;
    let theirs = g.player(1).hero;
    g.entity_mut(theirs).health = 4;
    act(&mut g, attack(minion(0, yeti), hero(1)));
    assert_eq!(g.outcome(), Some(Outcome::Draw));
}
