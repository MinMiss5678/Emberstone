//! 第 2 階段 A：英雄技能（case A1–A11a）。

mod common;

use common::*;
use emberstone_core::{ActError, Outcome, RngEvent};

// A1：每回合限用 1 次；費用 2；法力不夠或已用過就不能用
#[test]
fn a1_once_per_turn_and_costs_mana() {
    let mut g = start(&[], &[]);
    // 第 1 回合只有 1 顆水晶
    assert!(power_targets(&g).is_empty());
    assert_eq!(g.act(&power(Some(hero(1)))), Err(ActError::Illegal));
    until_turn(&mut g, 3);
    assert_eq!(g.player(0).mana, 2);
    act(&mut g, power(Some(hero(1))));
    assert_eq!(g.player(0).mana, 0);
    assert!(g.player(0).power_used);
    g.player_mut(0).mana = 10;
    assert!(power_targets(&g).is_empty());
    assert_eq!(g.act(&power(Some(hero(1)))), Err(ActError::Illegal));
    end(&mut g);
    end(&mut g);
    assert!(!g.player(0).power_used);
    assert!(!power_targets(&g).is_empty());
}

// A1：惡魔之爪費用 1，第 1 回合就能用
#[test]
fn a1_demon_claws_costs_one() {
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    act(&mut g, power(None));
    assert_eq!(g.player(0).mana, 0);
}

// A2：火焰衝擊對任一角色 1 點；對手的潛行不能選；不受法術傷害加成
#[test]
fn a2_fireblast() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "EX1_017"); // 自己的潛行手下
    ready(&mut g, 0, "CS3_007"); // 法術傷害 +1
    ready(&mut g, 1, "EX1_017"); // 對手的潛行手下
    ready(&mut g, 1, "CS2_120");
    g.player_mut(0).mana = 2;
    assert_eq!(
        power_targets(&g),
        sorted(vec![
            Some(hero(0)),
            Some(hero(1)),
            Some(minion(0, 0)),
            Some(minion(0, 1)),
            Some(minion(1, 1)),
        ])
    );
    act(&mut g, power(Some(minion(1, 1))));
    assert_eq!(board_entity(&g, 1, 1).health, 2);
}

// A3：次級治療術恢復 2 點，不超過上限
#[test]
fn a3_lesser_heal() {
    let mut g = start_with(PRIEST, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 10;
    set_hero_health(&mut g, 0, 25);
    act(&mut g, power(Some(hero(0))));
    assert_eq!(hero_health(&g, 0), 27);
    end(&mut g);
    end(&mut g);
    set_hero_health(&mut g, 0, 29);
    act(&mut g, power(Some(hero(0))));
    assert_eq!(hero_health(&g, 0), 30);
}

// A4：護甲提升！護甲 +2
#[test]
fn a4_armor_up() {
    let mut g = start_with(WARRIOR, &[], MAGE, &[]);
    g.player_mut(0).mana = 2;
    assert_eq!(power_targets(&g), vec![None]);
    act(&mut g, power(None));
    assert_eq!(hero_entity(&g, 0).armor, 2);
}

// A5：穩固射擊對敵方英雄造成 2 點傷害（不用選目標）
#[test]
fn a5_steady_shot() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 2;
    assert_eq!(power_targets(&g), vec![None]);
    assert_eq!(g.act(&power(Some(hero(1)))), Err(ActError::Illegal));
    act(&mut g, power(None));
    assert_eq!(hero_health(&g, 1), 28);
}

// A6：變身本回合攻擊 +1、護甲 +1；沒有武器也能攻擊
#[test]
fn a6_shapeshift() {
    let mut g = start_with(DRUID, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 2;
    act(&mut g, power(None));
    assert_eq!(g.hero_attack(0), 1);
    assert_eq!(hero_entity(&g, 0).armor, 1);
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 29);
    end(&mut g);
    end(&mut g);
    assert_eq!(g.hero_attack(0), 0);
    assert_eq!(hero_entity(&g, 0).armor, 1);
}

// A7：惡魔之爪本回合攻擊 +1；沒有武器也能攻擊
#[test]
fn a7_demon_claws() {
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    act(&mut g, power(None));
    assert_eq!(g.hero_attack(0), 1);
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 29);
    end(&mut g);
    end(&mut g);
    assert_eq!(g.hero_attack(0), 0);
}

// A8：匕首專精裝備 1/2 匕首，舊武器被摧毀
#[test]
fn a8_dagger_mastery() {
    let mut g = start_with(ROGUE, &[], WARRIOR, &[]);
    cast(&mut g, "CS2_106", None, None); // 熾炎戰斧 3/2
    act(&mut g, power(None));
    let w = g.player(0).weapon.expect("應該有武器");
    let w = g.entity(w);
    assert_eq!(w.card.id, "CS2_082");
    assert_eq!((w.attack, w.durability), (1, 2));
}

// A9：援軍召喚 1/1 白銀之手新兵；戰場已滿不能用
#[test]
fn a9_reinforce() {
    let mut g = start_with(PALADIN, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CS2_120");
    g.player_mut(0).mana = 10;
    act(&mut g, power(None));
    assert_eq!(board_ids(&g, 0), vec!["CS2_120", "CS2_101t"]);
    assert_eq!(board_stats(&g, 0)[1], (1, 1));
    end(&mut g);
    end(&mut g);
    for _ in 0..5 {
        ready(&mut g, 0, "CS2_120");
    }
    assert!(power_targets(&g).is_empty());
    assert_eq!(g.act(&power(None)), Err(ActError::Illegal));
}

const TOTEMS: [&str; 4] = ["CS2_050", "CS2_051", "CS2_052", "NEW1_009"];

// A10：從場上還沒有的基本圖騰中隨機召喚；候選順序為灼熱、石爪、風懲、治療
#[test]
fn a10_totemic_call_picks_missing_totem() {
    let mut g = start_scripted(
        SHAMAN,
        WARRIOR,
        vec![RngEvent::Int {
            lo: 0,
            hi: 1,
            value: 1,
        }],
    );
    ready(&mut g, 0, "CS2_051");
    ready(&mut g, 0, "NEW1_009");
    g.player_mut(0).mana = 2;
    act(&mut g, power(None));
    // 還沒有的是灼熱（第 0 個）與風懲（第 1 個），亂數選 1
    assert_eq!(board_ids(&g, 0), vec!["CS2_051", "NEW1_009", "CS2_052"]);
}

// A10：4 種都在時從四種中隨機召喚 1 個；戰場已滿不能用
#[test]
fn a10_totemic_call_all_four_and_full_board() {
    let mut g = start_scripted(
        SHAMAN,
        WARRIOR,
        vec![RngEvent::Int {
            lo: 0,
            hi: 3,
            value: 2,
        }],
    );
    for t in TOTEMS {
        ready(&mut g, 0, t);
    }
    g.player_mut(0).mana = 10;
    act(&mut g, power(None));
    assert_eq!(board_ids(&g, 0)[4], "CS2_052");
    end(&mut g);
    end(&mut g);
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CS2_120");
    assert_eq!(g.player(0).board.len(), 7);
    assert!(power_targets(&g).is_empty());
}

// A11：生命分流先受 2 點傷害再抽牌（抽到靈魂碎片時，滿血也看得出先扣血再回血）
// A10：召喚出的治療圖騰在你的回合結束時，替全部友方手下恢復 1 點生命值
#[test]
fn a10_healing_totem_heals_at_end_of_turn() {
    let mut g = start_with(SHAMAN, &[], WARRIOR, &[]);
    ready(&mut g, 0, "NEW1_009");
    ready(&mut g, 0, "CS2_182");
    let yeti = g.player(0).board[1];
    g.entity_mut(yeti).health = 2;
    end(&mut g);
    assert_eq!(board_entity(&g, 0, 1).health, 3);
    end(&mut g);
    assert_eq!(board_entity(&g, 0, 1).health, 3);
}

#[test]
fn a11_life_tap_damage_then_draw() {
    let mut g = start_with(WARLOCK, &[], WARRIOR, &[]);
    end(&mut g);
    end(&mut g);
    let hand = g.player(0).hand.len();
    let deck = g.player(0).deck.len();
    act(&mut g, power(None));
    assert_eq!(hero_health(&g, 0), 28);
    assert_eq!(g.player(0).hand.len(), hand + 1);
    assert_eq!(g.player(0).deck.len(), deck - 1);
}

#[test]
fn a11_life_tap_soul_fragment_order() {
    // 先手開局抽 4 張，第 5 張是靈魂碎片、第 6 張是冰風雪人
    let mut g = start_with(
        WARLOCK,
        &[
            "CS2_120", "CS2_120", "CS2_120", "CS2_120", "SCH_307t", "CS2_182",
        ],
        WARRIOR,
        &[],
    );
    g.player_mut(0).mana = 2;
    act(&mut g, power(None));
    // 30 → 28（生命分流）→ 30（靈魂碎片抽中時施放），並補抽冰風雪人
    assert_eq!(hero_health(&g, 0), 30);
    let hand = hand_ids(&g, 0);
    assert!(!hand.contains(&"SCH_307t"));
    assert_eq!(hand.last(), Some(&"CS2_182"));
}

// A11a：生命分流把英雄打到 0，抽到靈魂碎片回血 → 不會死亡
#[test]
fn a11a_life_tap_lethal_saved_by_soul_fragment() {
    let mut g = start_with(
        WARLOCK,
        &["CS2_120", "CS2_120", "CS2_120", "CS2_120", "SCH_307t"],
        WARRIOR,
        &[],
    );
    g.player_mut(0).mana = 2;
    set_hero_health(&mut g, 0, 2);
    act(&mut g, power(None));
    assert_eq!(hero_health(&g, 0), 2);
    assert_eq!(g.outcome(), None);
}

#[test]
fn a11a_life_tap_lethal_without_heal_dies() {
    let mut g = start_with(WARLOCK, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 2;
    set_hero_health(&mut g, 0, 2);
    act(&mut g, power(None));
    assert_eq!(g.outcome(), Some(Outcome::Won(1)));
}
