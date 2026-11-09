//! 第 2 階段 F：變形、衍生物、二選一、發現、隨機效果（case F1–F4）。

mod common;

use common::*;
use emberstone_core::{ActError, Action, Game, RngEvent, RngSource};

// F1：變形：換成新卡，數值、關鍵字、附魔全部重設、傷害清除；留在原位置
#[test]
fn f1_polymorph() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_120");
    ready(&mut g, 1, "CS2_179"); // 森金禦盾大師 3/5 嘲諷
    ready(&mut g, 1, "CS2_120");
    cast(&mut g, "CORE_CS2_009", None, Some(minion(1, 1))); // 野性印記 +2/+3
    cast(&mut g, "CORE_DS1_185", None, Some(minion(1, 1))); // 秘法射擊 2 點
    cast(&mut g, "CS2_022", None, Some(minion(1, 1))); // 變形術
    assert_eq!(board_ids(&g, 1), vec!["CS2_120", "CS2_tk1", "CS2_120"]);
    let sheep = board_entity(&g, 1, 1);
    assert_eq!((sheep.attack, sheep.health, sheep.max_health), (1, 1, 1));
    assert!(!sheep.taunt);
}

// F1a：自己回合把自己的手下變形 → 視同剛上場
#[test]
fn f1a_transformed_minion_is_new() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_182");
    ready(&mut g, 0, "CS2_182");
    ready(&mut g, 0, "CS2_182");
    // 可以攻擊的雪人變成綿羊：不能攻擊
    cast(&mut g, "CS2_022", None, Some(minion(0, 0)));
    assert!(attack_targets_of(&g, minion(0, 0)).is_empty());
    // 變成有衝鋒的：可以立刻攻擊英雄；有衝刺的：只能攻擊手下
    ready(&mut g, 1, "CS2_120");
    let e = g.player(0).board[1];
    g.transform(e, "CS2_124"); // 狼騎兵：衝鋒
    assert!(attack_targets_of(&g, minion(0, 1)).contains(&hero(1)));
    let e = g.player(0).board[2];
    g.transform(e, "CS3_038"); // 紅鰓刃顎魚人：衝刺
    assert_eq!(attack_targets_of(&g, minion(0, 2)), vec![minion(1, 0)]);
}

fn attack_targets_of(g: &Game, attacker: emberstone_core::Target) -> Vec<emberstone_core::Target> {
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Attack {
                attacker: who,
                target,
            } if who == attacker => Some(target),
            _ => None,
        })
        .collect()
}

// F2：二選一（野性之力）：choose_one 1 是召喚 3/2 獵豹，2 是你的手下 +1/+1（順序同 RosettaStone）
#[test]
fn f2_choose_one_without_target() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_EX1_160");
    let opts: Vec<Action> = g
        .legal_actions()
        .into_iter()
        .filter(|a| matches!(a, Action::Play { hand, .. } if *hand == i))
        .collect();
    assert_eq!(
        opts,
        vec![choose_one(i, None, None, 1), choose_one(i, None, None, 2)]
    );
    assert_eq!(g.act(&play(i, None, None)), Err(ActError::Illegal));
    let mut a = g.clone();
    act(&mut a, choose_one(i, None, None, 1));
    assert_eq!(board_ids(&a, 0), vec!["CS2_120", "EX1_160t"]);
    act(&mut g, choose_one(i, None, None, 2));
    assert_eq!(board_stats(&g, 0), vec![(3, 4)]);
}

// F1b：變形後保留原本的上場順序：先上場的手下被變形後和後上場的一起死，仍先移出戰場
// 盤面 [A, 鬼怪蠕行者, 冰風雪人]：A 先移除 → 蠕行者記下的位置是 0，蜘蛛放在最左邊
#[test]
fn f1b_transform_keeps_play_order() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_168"); // 魚人劫掠者（A，先上場）
    ready(&mut g, 0, "FP1_002"); // 鬼怪蠕行者
    ready(&mut g, 0, "CS2_182"); // 冰風雪人 4/5
    cast(&mut g, "CS2_022", None, Some(minion(0, 0))); // 變形術：A 變成綿羊
    cast(&mut g, "CORE_CS2_062", None, None); // 地獄烈焰：綿羊和蠕行者同時死亡
    assert_eq!(board_ids(&g, 0), vec!["FP1_002t", "FP1_002t", "CS2_182"]);
}

// F2a：戰場已滿時，二選一的召喚選項不能選，另一個選項照常
#[test]
fn f2a_summon_option_needs_board_slot() {
    let mut g = start(&[], &[]);
    for _ in 0..7 {
        ready(&mut g, 0, "CS2_120");
    }
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_EX1_160");
    let opts: Vec<Action> = g
        .legal_actions()
        .into_iter()
        .filter(|a| matches!(a, Action::Play { hand, .. } if *hand == i))
        .collect();
    assert_eq!(opts, vec![choose_one(i, None, None, 2)]);
    assert_eq!(g.act(&choose_one(i, None, None, 1)), Err(ActError::Illegal));
}

// F2：二選一（憤怒）：兩個選項各自列出合法目標
#[test]
fn f2_choose_one_with_targets() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182");
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_EX1_154");
    let hand = g.player(0).hand.len();
    let mut a = g.clone();
    act(&mut a, choose_one(i, None, Some(minion(1, 0)), 1)); // 3 點
    assert_eq!(board_entity(&a, 1, 0).health, 2);
    act(&mut g, choose_one(i, None, Some(minion(1, 0)), 2)); // 1 點並抽一張
    assert_eq!(board_entity(&g, 1, 0).health, 4);
    assert_eq!(g.player(0).hand.len(), hand);
}

// F3：發現（追蹤）：選擇畫面時合法行動只剩「選第 i 個」；選完放進手牌
#[test]
fn f3_discover_from_deck() {
    // 開局抽 4 張淡水鱷；牌庫剩 冰風雪人、魚人劫掠者、淡水鱷×24
    let mut cfg = config(
        HUNTER,
        deck(
            &[
                "CS2_120", "CS2_120", "CS2_120", "CS2_120", "CS2_182", "CS2_168",
            ],
            30,
        ),
        WARRIOR,
        deck(&[], 30),
    );
    // 不重複的卡依 dbfId 排序：雪人(90)、劫掠者(191)、淡水鱷(1369)；洗牌後第 i 個放原本的第 perm[i] 個
    cfg.rng = RngSource::Scripted(vec![RngEvent::Shuffle {
        n: 3,
        perm: vec![2, 0, 1],
    }]);
    let mut g = Game::new(cfg).expect("開局失敗");
    let deck = g.player(0).deck.len();
    cast(&mut g, "CORE_DS1_184", None, None);
    let opts = g.legal_actions();
    assert_eq!(
        opts,
        vec![
            Action::Choose { index: 0 },
            Action::Choose { index: 1 },
            Action::Choose { index: 2 },
        ]
    );
    assert_eq!(g.act(&Action::End), Err(ActError::Illegal));
    act(&mut g, Action::Choose { index: 1 }); // 冰風雪人
    assert_eq!(hand_ids(&g, 0).last(), Some(&"CS2_182"));
    assert_eq!(g.player(0).deck.len(), deck - 1);
    let top = g.player(0).deck[0];
    assert_eq!(g.entity(top).card.id, "CS2_168");
    assert!(g.legal_actions().contains(&Action::End));
}

// F4：隨機效果（瘋狂炸彈客）：3 次各從「其他角色」挑一個（自己的其他手下、自己的英雄、對手的手下、對手的英雄）
#[test]
fn f4_mad_bomber_follows_script() {
    let mut g = start_scripted(
        MAGE,
        WARRIOR,
        vec![
            RngEvent::Int {
                lo: 0,
                hi: 2,
                value: 1,
            },
            RngEvent::Int {
                lo: 0,
                hi: 2,
                value: 1,
            },
            RngEvent::Int {
                lo: 0,
                hi: 2,
                value: 2,
            },
        ],
    );
    ready(&mut g, 1, "CS2_120");
    cast(&mut g, "CORE_EX1_082", Some(0), None);
    assert_eq!(board_entity(&g, 1, 0).health, 1);
    assert_eq!(hero_health(&g, 1), 29);
    assert_eq!(hero_health(&g, 0), 30);
}

// F4a：第二、三發不會打到已經被打死的角色（候選數跟著變少）
#[test]
fn f4a_mad_bomber_skips_dead() {
    let mut g = start_scripted(
        MAGE,
        WARRIOR,
        vec![
            // 候選：自己的英雄、對手的魚人劫掠者（2/1）、對手的英雄 → 打死劫掠者
            RngEvent::Int {
                lo: 0,
                hi: 2,
                value: 1,
            },
            // 剩自己的英雄、對手的英雄
            RngEvent::Int {
                lo: 0,
                hi: 1,
                value: 1,
            },
            RngEvent::Int {
                lo: 0,
                hi: 1,
                value: 0,
            },
        ],
    );
    ready(&mut g, 1, "CS2_168");
    cast(&mut g, "CORE_EX1_082", Some(0), None);
    assert!(g.player(1).board.is_empty());
    assert_eq!((hero_health(&g, 0), hero_health(&g, 1)), (29, 29));
}

// F4b：沒有可選的角色時，剩下的飛彈不造成傷害
#[test]
fn f4b_mad_bomber_no_targets_left() {
    let mut g = start_scripted(
        MAGE,
        WARRIOR,
        vec![
            // 候選：自己的英雄、對手的英雄 → 對手英雄 0 血
            RngEvent::Int {
                lo: 0,
                hi: 1,
                value: 1,
            },
            // 剩自己的英雄 → 0 血；第三發沒有候選，不抽亂數也不造成傷害
            RngEvent::Int {
                lo: 0,
                hi: 0,
                value: 0,
            },
        ],
    );
    set_hero_health(&mut g, 0, 1);
    set_hero_health(&mut g, 1, 1);
    cast(&mut g, "CORE_EX1_082", Some(0), None);
    assert_eq!((hero_health(&g, 0), hero_health(&g, 1)), (0, 0));
}

// F5：裝備武器的英雄在對手回合攻擊力是 0，被攻擊時不反擊
#[test]
fn f5_weapon_inactive_on_opponent_turn() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CS2_106", None, None); // 熾炎戰斧 3/2
    assert_eq!(g.hero_attack(0), 3);
    end(&mut g);
    assert_eq!(g.hero_attack(0), 0);
    let i = ready(&mut g, 1, "CS2_120"); // 淡水鱷 2/3
    act(&mut g, attack(minion(1, i), hero(0)));
    assert_eq!(board_entity(&g, 1, i).health, 3);
    assert_eq!(hero_health(&g, 0), 28);
}
