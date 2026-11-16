//! 第 3a 階段 A、B：傷害、治療、抽牌、護甲與指定目標的條件（`docs/第3a階段-case表.md`）。

mod common;

use common::*;
use emberstone_core::Game;

/// 先手手牌清空後，只留 n 張淡水鱷。
fn set_hand(g: &mut Game, n: usize) {
    g.player_mut(0).hand.clear();
    for _ in 0..n {
        give(g, 0, FILL);
    }
}

// A1：法術傷害只加在法術上；戰吼不加
#[test]
fn a1_spell_damage_adds_to_spells_only() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_012"); // 血法師薩爾諾斯：法術傷害 +1
    ready(&mut g, 0, "CS3_007"); // 見習電擊手：法術傷害 +1
    cast(&mut g, "CORE_CS2_029", None, Some(hero(1))); // 火球術 6 + 2
    assert_eq!(hero_health(&g, 1), 22);
    cast(&mut g, "CORE_CS2_042", Some(2), Some(hero(1))); // 火元素戰吼 4，不加
    assert_eq!(hero_health(&g, 1), 18);
}

// A1：秘密觸發時也加法術傷害，看秘密擁有者那一方（實測 R1）
#[test]
fn a1_secret_uses_owner_spell_damage() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CS3_007"); // 獵人方法術傷害 +1
    cast(&mut g, "CORE_EX1_610", None, None); // 爆炸陷阱
    end(&mut g);
    ready(&mut g, 1, "CORE_EX1_012"); // 攻擊方的法術傷害不算
    let i = ready(&mut g, 1, "CS2_182"); // 冰風雪人 4/5
    act(&mut g, attack(minion(1, i), hero(0)));
    assert_eq!(hero_health(&g, 1), 27);
    // 薩爾諾斯 1/1 被打死；雪人受 3 點後照常攻擊
    assert_eq!(board_ids(&g, 1), vec!["CS2_182"]);
    assert_eq!(board_entity(&g, 1, 0).health, 2);
    assert_eq!(hero_health(&g, 0), 26);
}

// A1：爆炸符文的 6 點加上秘密擁有者的法術傷害，溢出照總量算（實測 R10）
#[test]
fn a1_explosive_runes_spell_damage() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007"); // 法術傷害 +1
    cast(&mut g, "CORE_LOOT_101", None, None);
    end(&mut g);
    cast(&mut g, "CS2_182", Some(0), None); // 冰風雪人 4/5 受 5、溢出 2
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 1), 28);
    // 7 生命的手下正好被 7 點打死，沒有溢出
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    cast(&mut g, "CORE_LOOT_101", None, None);
    end(&mut g);
    cast(&mut g, "CS2_186", Some(0), None); // 戰爭傀儡 7/7
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 1), 30);
}

// A2：噬靈瘟疫 4 次各 1 點，只挑還活著的敵方手下；法術傷害增加次數；生命竊取逐次回復
#[test]
fn a2_devouring_plague_split_and_lifesteal() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    ready(&mut g, 1, "CS2_182");
    ready(&mut g, 1, "CS2_182");
    set_hero_health(&mut g, 0, 20);
    cast(&mut g, "CORE_BAR_311", None, None);
    let lost: i32 = board_stats(&g, 1).iter().map(|&(_, h)| 5 - h).sum();
    assert_eq!(lost, 5);
    assert_eq!(hero_health(&g, 0), 25);
    assert_eq!(hero_health(&g, 1), 30);
}

// A2：沒有目標的那次不造成傷害，也不回復
#[test]
fn a2_devouring_plague_no_target_left() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_168"); // 魚人劫掠者 2/1
    set_hero_health(&mut g, 0, 20);
    cast(&mut g, "CORE_BAR_311", None, None);
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 0), 21);
}

// A3：生命竊取的法術回復實際造成的傷害；聖盾擋下不回復
#[test]
fn a3_drain_soul_lifesteal() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    ready(&mut g, 1, "CS2_182");
    ready(&mut g, 1, "CORE_GVG_085"); // 煩人機器人 1/2 聖盾
    set_hero_health(&mut g, 0, 20);
    cast(&mut g, "CORE_ICC_055", None, Some(minion(1, 0)));
    assert_eq!(board_entity(&g, 1, 0).health, 1);
    assert_eq!(hero_health(&g, 0), 24);
    cast(&mut g, "CORE_ICC_055", None, Some(minion(1, 1)));
    assert!(!board_entity(&g, 1, 1).divine_shield);
    assert_eq!(hero_health(&g, 0), 24);
}

// A4：重擊：傷害加法術傷害，護甲不加
#[test]
fn a4_bash_damage_then_armor() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    cast(&mut g, "CORE_AT_064", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 26);
    assert_eq!(hero_entity(&g, 0).armor, 3);
}

// A4：毒襲：1 點傷害後抽一張
#[test]
fn a4_shiv_damage_and_draw() {
    let mut g = start(&[], &[]);
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_EX1_278", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 29);
    assert_eq!(g.player(0).hand.len(), 1);
}

// A5：快速射擊：傷害後手牌（不含自己）沒有牌才抽
#[test]
fn a5_quick_shot_draws_only_with_empty_hand() {
    let mut g = start(&[], &[]);
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_BRM_013", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 27);
    assert_eq!(g.player(0).hand.len(), 1);
    set_hand(&mut g, 1);
    cast(&mut g, "CORE_BRM_013", None, Some(hero(1)));
    assert_eq!(g.player(0).hand.len(), 1);
}

// A6：猛擊：目標存活（含聖盾擋下）才抽
#[test]
fn a6_slam_draws_if_survives() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182"); // 4/5 存活
    ready(&mut g, 1, "CS2_168"); // 2/1 死亡
    ready(&mut g, 1, "CORE_GVG_085"); // 聖盾
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_EX1_391", None, Some(minion(1, 0)));
    assert_eq!(g.player(0).hand.len(), 1);
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_EX1_391", None, Some(minion(1, 1)));
    assert_eq!(g.player(0).hand.len(), 0);
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_EX1_391", None, Some(minion(1, 1))); // 煩人機器人移到第 1 格
    assert_eq!(g.player(0).hand.len(), 1);
}

// A7：視界術：抽到的牌費用 −3（最低 0）；牌庫空了（疲勞）就沒有效果
#[test]
fn a7_far_sight_reduces_drawn_card() {
    let mut g = start(&[FILL, FILL, FILL, FILL, "CORE_CS2_029", FILL], &[]);
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_CS2_053", None, None);
    let drawn = g.player(0).hand[0];
    assert_eq!(g.entity(drawn).card.id, "CORE_CS2_029");
    assert_eq!(g.entity(drawn).cost(), 1);
    cast(&mut g, "CORE_CS2_053", None, None);
    let drawn = g.player(0).hand[1];
    assert_eq!(g.entity(drawn).cost(), 0); // 淡水鱷 2 費
    g.player_mut(0).deck.clear();
    cast(&mut g, "CORE_CS2_053", None, None);
    assert_eq!(g.player(0).hand.len(), 2);
    assert_eq!(hero_health(&g, 0), 29);
}

// A8：治療不超過上限；可指定任何角色
#[test]
fn a8_heals() {
    let mut g = start(&[], &[]);
    set_hero_health(&mut g, 0, 20);
    cast(&mut g, "CORE_AT_055", None, Some(hero(0))); // 快速治療 5
    assert_eq!(hero_health(&g, 0), 25);
    cast(&mut g, "CORE_TRL_307", None, Some(hero(0))); // 聖光閃現 4，抽一張
    assert_eq!(hero_health(&g, 0), 29);
    cast(&mut g, "CORE_EX1_011", Some(0), Some(hero(0))); // 巫毒巫醫 2，最多到 30
    assert_eq!(hero_health(&g, 0), 30);
    set_hero_health(&mut g, 1, 20);
    cast(&mut g, "CORE_EX1_011", Some(0), Some(hero(1))); // 也能治療敵方
    assert_eq!(hero_health(&g, 1), 22);
}

// A8：平衡梟獸二選一（同 RosettaStone 順序：1 恢復英雄 8 點、2 造成 4 點）；戰吼不加法術傷害
#[test]
fn a8_boomkin_choose_one() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    set_hero_health(&mut g, 0, 10);
    let i = give(&mut g, 0, "CORE_ONY_018");
    g.player_mut(0).mana = 10;
    act(&mut g, choose_one(i, Some(0), None, 1));
    assert_eq!(hero_health(&g, 0), 18);
    let i = give(&mut g, 0, "CORE_ONY_018");
    g.player_mut(0).mana = 10;
    act(&mut g, choose_one(i, Some(0), Some(hero(1)), 2));
    assert_eq!(hero_health(&g, 1), 26);
}

// A8：吸血蚊：敵方英雄 3 點、自己的英雄恢復 3 點，都不加法術傷害
#[test]
fn a8_lifedrinker() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    set_hero_health(&mut g, 0, 20);
    cast(&mut g, "CORE_GIL_622", Some(1), None);
    assert_eq!(hero_health(&g, 1), 27);
    assert_eq!(hero_health(&g, 0), 23);
}

// A9：混沌之擊、狂野怒氣：本回合英雄攻擊力，回合結束失效；狂野怒氣 2 是 8 點護甲
#[test]
fn a9_hero_attack_this_turn() {
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    set_hand(&mut g, 0);
    cast(&mut g, "CORE_BT_035", None, None);
    assert_eq!(g.hero_attack(0), 2);
    assert_eq!(g.player(0).hand.len(), 1);
    let i = give(&mut g, 0, "CORE_OG_047");
    g.player_mut(0).mana = 10;
    act(&mut g, choose_one(i, None, None, 1));
    assert_eq!(g.hero_attack(0), 6);
    let i = give(&mut g, 0, "CORE_OG_047");
    g.player_mut(0).mana = 10;
    act(&mut g, choose_one(i, None, None, 2));
    assert_eq!(hero_entity(&g, 0).armor, 8);
    end(&mut g);
    assert_eq!(hero_entity(&g, 0).attack, 0);
}

// A10：啟動：本回合 +1 法力，可以超過 10
#[test]
fn a10_innervate() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_169", None, None);
    assert_eq!(g.player(0).mana, 11);
}

// B1：背刺只能指定未受傷的手下（敵我皆可）
#[test]
fn b1_backstab_undamaged_minions() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_182");
    ready(&mut g, 1, "CS2_182");
    ready(&mut g, 1, "CS2_182");
    let e = g.player(1).board[1];
    g.entity_mut(e).health = 3;
    let i = give(&mut g, 0, "CORE_CS2_072");
    assert_eq!(
        play_targets(&g, i),
        sorted(vec![Some(minion(0, 0)), Some(minion(1, 0))])
    );
}

// B2：斬殺只能指定受傷的敵方手下
#[test]
fn b2_execute_damaged_enemy_minion() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_182");
    let e = g.player(0).board[0];
    g.entity_mut(e).health = 3;
    ready(&mut g, 1, "CS2_182");
    ready(&mut g, 1, "CS2_182");
    let e = g.player(1).board[1];
    g.entity_mut(e).health = 3;
    let i = give(&mut g, 0, "CORE_CS2_108");
    g.player_mut(0).mana = 10;
    assert_eq!(play_targets(&g, i), vec![Some(minion(1, 1))]);
    act(&mut g, play(i, None, Some(minion(1, 1))));
    assert_eq!(g.player(1).board.len(), 1);
}

// B3：致命毒藥：沒有武器不能施放；有武器時武器永久 +2 攻擊力
#[test]
fn b3_deadly_poison_needs_weapon() {
    let mut g = start_with(ROGUE, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_CS2_074");
    assert!(play_targets(&g, i).is_empty());
    act(&mut g, power(None)); // 匕首精通：裝備 1/2 匕首
    act(&mut g, play(i, None, None));
    assert_eq!(g.hero_attack(0), 3);
    end(&mut g);
    end(&mut g);
    assert_eq!(g.hero_attack(0), 3);
}

// B5：霧帆掠劫者：有武器時戰吼要指定目標並造成 2 點；沒有武器時不用選目標
#[test]
fn b5_fogsail_freebooter_needs_weapon() {
    let mut g = start_with(ROGUE, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CS3_022");
    assert_eq!(play_targets(&g, i), vec![None]);
    act(&mut g, power(None));
    assert!(play_targets(&g, i).contains(&Some(hero(1))));
    act(&mut g, play(i, Some(0), Some(hero(1))));
    assert_eq!(hero_health(&g, 1), 28);
}

// B6：火元素可指定任何角色；斬隙者、鍊金師任何手下；開朗小幫手只能友方手下
#[test]
fn b6_battlecry_target_kinds() {
    let mut g = start(&[], &[]);
    g.player_mut(0).mana = 10;
    ready(&mut g, 0, "CS2_182");
    ready(&mut g, 1, "CS2_182");
    let all = sorted(vec![
        Some(hero(0)),
        Some(hero(1)),
        Some(minion(0, 0)),
        Some(minion(1, 0)),
    ]);
    let minions = sorted(vec![Some(minion(0, 0)), Some(minion(1, 0))]);
    let i = give(&mut g, 0, "CORE_CS2_042");
    assert_eq!(play_targets(&g, i), all);
    let i = give(&mut g, 0, "CORE_ULD_165");
    assert_eq!(play_targets(&g, i), minions);
    let i = give(&mut g, 0, "CORE_EX1_059");
    assert_eq!(play_targets(&g, i), minions);
    let i = give(&mut g, 0, "CORE_ULD_191");
    assert_eq!(play_targets(&g, i), vec![Some(minion(0, 0))]);
}
