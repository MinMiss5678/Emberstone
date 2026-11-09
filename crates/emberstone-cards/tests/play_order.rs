//! 第 2 階段 C：出牌時序（case C1–C5）。

mod common;

use common::*;
use emberstone_core::{ActError, Outcome, RngEvent};

// C1：扣法力 → 放上戰場 → 戰吼 → 召喚後觸發 → 死亡階段
#[test]
fn c1_play_minion_sequence() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_509"); // 魚人招潮者 1/2
    ready(&mut g, 1, "CS2_168"); // 魚人劫掠者 2/1
    let i = give(&mut g, 0, "CORE_CS2_189"); // 精靈弓箭手：戰吼 1 點傷害
    g.player_mut(0).mana = 1;
    act(&mut g, play(i, Some(1), Some(minion(1, 0))));
    assert_eq!(g.player(0).mana, 0);
    assert_eq!(board_ids(&g, 0), vec!["CORE_EX1_509", "CORE_CS2_189"]);
    // 戰吼打死的手下在死亡階段移出戰場
    assert!(g.player(1).board.is_empty());
}

// C2：戰吼有合法目標時一定要選；沒有就照常出、戰吼不發動
#[test]
fn c2_battlecry_target_required_when_available() {
    let mut g = start(&[], &[]);
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_EX1_362"); // 銀色黎明保衛者：賦予一個友方手下聖盾術
    // 沒有友方手下：不用選目標
    assert_eq!(play_targets(&g, i), vec![None]);
    act(&mut g, play(i, Some(0), None));
    assert!(!board_entity(&g, 0, 0).divine_shield);
    // 有友方手下：一定要選
    let j = give(&mut g, 0, "CORE_EX1_362");
    assert_eq!(play_targets(&g, j), vec![Some(minion(0, 0))]);
    assert_eq!(g.act(&play(j, Some(1), None)), Err(ActError::Illegal));
    act(&mut g, play(j, Some(1), Some(minion(0, 0))));
    assert!(board_entity(&g, 0, 0).divine_shield);
    assert!(!board_entity(&g, 0, 1).divine_shield);
}

// C3：戰吼召喚的衍生物放在出牌手下的右邊；戰場已滿就不召喚（核心系列獵潮魚人的衍生物是 CORE_EX1_506a）
#[test]
fn c3_battlecry_token_to_the_right() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CS2_182");
    cast(&mut g, "CORE_EX1_506", Some(1), None); // 獵潮魚人
    assert_eq!(
        board_ids(&g, 0),
        vec!["CS2_120", "CORE_EX1_506", "CORE_EX1_506a", "CS2_182"]
    );
    for _ in 0..2 {
        ready(&mut g, 0, "CS2_120");
    }
    assert_eq!(g.player(0).board.len(), 6);
    cast(&mut g, "CORE_EX1_506", Some(6), None);
    assert_eq!(g.player(0).board.len(), 7);
    assert_eq!(board_ids(&g, 0)[6], "CORE_EX1_506");
}

// C4：召喚觸發：獵潮魚人和牠召喚的斥候都是魚人，招潮者 +2 攻擊
#[test]
fn c4_summon_trigger_counts_battlecry_token() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_509");
    cast(&mut g, "CORE_EX1_506", Some(1), None);
    assert_eq!(board_entity(&g, 0, 0).attack, 3);
    // 非魚人不觸發
    cast(&mut g, "CS2_120", Some(3), None);
    assert_eq!(board_entity(&g, 0, 0).attack, 3);
}

// C5：鄰近目標只影響當下左右相鄰的手下
#[test]
fn c5_adjacent_battlecry() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CS2_168");
    ready(&mut g, 0, "CS2_182");
    cast(&mut g, "CORE_EX1_058", Some(2), None); // 日怒保衛者放在劫掠者和雪人中間
    let taunts: Vec<bool> = g
        .player(0)
        .board
        .iter()
        .map(|&e| g.entity(e).taunt)
        .collect();
    assert_eq!(taunts, vec![false, true, false, true]);
}

// C6：戰吼結算後先進入死亡階段：被打死的勇士在爆炸符文觸發前就移出戰場，加成跟著消失
#[test]
fn c6_death_phase_before_after_play_secret() {
    let mut g = start_scripted(
        MAGE,
        MAGE,
        vec![
            // 候選：暴風城勇士、自己的英雄、對手的英雄 → 勇士（1 血）被打死
            RngEvent::Int {
                lo: 0,
                hi: 2,
                value: 0,
            },
            RngEvent::Int {
                lo: 0,
                hi: 1,
                value: 1,
            },
            RngEvent::Int {
                lo: 0,
                hi: 1,
                value: 1,
            },
        ],
    );
    end(&mut g);
    cast(&mut g, "CORE_LOOT_101", None, None); // 對手的爆炸符文
    end(&mut g);
    let i = ready(&mut g, 0, "CORE_CS2_222"); // 暴風城勇士
    let champion = g.player(0).board[i];
    g.entity_mut(champion).health = 1;
    cast(&mut g, "CORE_EX1_082", Some(1), None); // 瘋狂炸彈客
    // 勇士已移出戰場，炸彈客回到 3/2：爆炸符文 6 點，溢出 4 點打英雄
    assert_eq!(hero_health(&g, 0), 26);
    assert_eq!(hero_health(&g, 1), 28);
    assert!(g.player(0).board.is_empty());
}

// C7：戰吼後的死亡階段判定英雄死亡，出牌流程照樣繼續：死亡英雄的爆炸符文照樣觸發，流程結束才分勝負
#[test]
fn c7_dead_hero_secret_still_triggers() {
    let mut g = start_scripted(
        MAGE,
        MAGE,
        vec![
            // 候選：自己的英雄、對手的英雄 → 對手英雄（1 血）
            RngEvent::Int {
                lo: 0,
                hi: 1,
                value: 1,
            },
            // 對手英雄已經 0 血，只剩自己的英雄
            RngEvent::Int {
                lo: 0,
                hi: 0,
                value: 0,
            },
            RngEvent::Int {
                lo: 0,
                hi: 0,
                value: 0,
            },
        ],
    );
    end(&mut g);
    cast(&mut g, "CORE_LOOT_101", None, None); // 對手的爆炸符文
    end(&mut g);
    set_hero_health(&mut g, 1, 1);
    set_hero_health(&mut g, 0, 3);
    cast(&mut g, "CORE_EX1_082", Some(0), None); // 瘋狂炸彈客：對手英雄死亡，自己的英雄剩 1
    // 爆炸符文照樣觸發：炸彈客 3/2 吃 6 點，溢出 4 點打死自己的英雄 → 平手
    assert!(g.player(1).secrets.is_empty());
    assert_eq!(g.outcome(), Some(Outcome::Draw));
}
