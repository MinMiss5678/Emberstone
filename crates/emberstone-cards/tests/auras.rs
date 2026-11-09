//! 第 2 階段 E：觸發、被動加成、附魔（case E1–E7）。

mod common;

use common::*;

// E1：受傷觸發：每有一隻手下受傷 +1 攻擊；範圍傷害打到 N 隻就 +N；聖盾術擋下不算
#[test]
fn e1_frothing_berserker_counts_each_damaged_minion() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_604"); // 白沫狂戰士 2/4
    ready(&mut g, 0, "CS2_182"); // 冰風雪人 4/5
    ready(&mut g, 1, "CS2_182");
    ready(&mut g, 1, "EX1_008"); // 銀白侍從 1/1 聖盾術
    cast(&mut g, "CORE_CS2_062", None, None); // 地獄烈焰：3 隻手下受傷、侍從被聖盾擋下
    assert_eq!(board_entity(&g, 0, 0).attack, 2 + 3);
}

// E2：被動加成：來源在場上才有效（只加其他友方手下），離場馬上失效
#[test]
fn e2_passive_buffs_follow_source() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120"); // 淡水鱷 2/3
    ready(&mut g, 1, "CS2_120");
    cast(&mut g, "CORE_CS2_222", Some(1), None); // 暴風城勇士 7/7
    cast(&mut g, "CORE_CS2_122", Some(2), None); // 團隊隊長 2/3
    assert_eq!(board_stats(&g, 0), vec![(4, 4), (8, 7), (3, 4)]);
    assert_eq!(board_stats(&g, 1), vec![(2, 3)]);
}

// E2：來源離場（被打死）後加成馬上失效
#[test]
fn e2_passive_buff_removed_when_source_dies() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CORE_CS2_122"); // 團隊隊長
    assert_eq!(board_stats(&g, 0)[0], (3, 3));
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 1)));
    assert_eq!(board_stats(&g, 0), vec![(2, 3)]);
}

// E2a：被動加成的來源在死亡階段死亡，加成保留到死亡之聲結算完才移除；這段期間召喚的衍生物拿不到
#[test]
fn e2a_dead_source_buff_lasts_through_deathrattles() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_CS2_222"); // 暴風城勇士
    ready(&mut g, 0, "CS2_120"); // 淡水鱷 2/3 → 3/4
    ready(&mut g, 0, "EX1_097"); // 憎惡：死亡之聲對所有角色造成 2 點
    ready(&mut g, 0, "FP1_002"); // 鬼怪蠕行者：死亡之聲召喚 2 隻 1/1 蜘蛛
    for i in [0, 2, 3] {
        let m = g.player(0).board[i];
        g.entity_mut(m).health = 0;
    }
    end(&mut g); // 回合結束的死亡階段：勇士、憎惡、蠕行者一起死
    // 淡水鱷 3/4 吃 2 點變 3/2，勇士的加成之後才移除 → 2/2（上限 3）；蜘蛛沒拿到勇士的加成
    assert_eq!(board_stats(&g, 0), vec![(2, 2), (1, 1), (1, 1)]);
    assert_eq!(board_entity(&g, 0, 0).max_health, 3);
}

// E3：鄰近被動加成隨位置變化即時更新
// E7＋E4：生命被動加成移除後，受傷的手下剛好回滿（目前生命＝新上限）→ 受傷加成立刻失效
#[test]
fn e7_health_aura_removed_ends_enrage() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_CS2_222"); // 暴風城勇士
    ready(&mut g, 0, "CORE_OG_218"); // 血蹄勇戰者 2/6 → 3/7
    let brave = g.player(0).board[1];
    g.deal_damage(brave, 1, 1); // 6/7 受傷：+3 攻擊
    assert_eq!(board_stats(&g, 0)[1], (6, 6));
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0))); // 火球術 6 點
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0))); // 再一發打死勇士
    // 上限 7 → 6、目前 6：回滿了，受傷加成失效
    assert_eq!(board_stats(&g, 0), vec![(2, 6)]);
}

// E3：鄰近被動加成隨位置變化即時更新
#[test]
fn e3_adjacent_buff_follows_position() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CORE_EX1_162"); // 恐狼首領
    ready(&mut g, 0, "CS2_120");
    assert_eq!(board_stats(&g, 0), vec![(3, 3), (2, 2), (3, 3)]);
    cast(&mut g, "CS2_182", Some(1), None); // 雪人插在左邊鱷魚和恐狼之間
    assert_eq!(board_stats(&g, 0), vec![(2, 3), (5, 5), (2, 2), (3, 3)]);
}

// E4：移除生命的被動加成：上限降回去；目前生命只在超過新上限時才降，不會因此死亡
#[test]
fn e4_removing_health_buff_does_not_kill() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120"); // 淡水鱷 2/3 → 3/4
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CORE_CS2_222");
    cast(&mut g, "CORE_CS1_130", None, Some(minion(0, 0))); // 聖光懲擊 3 點：3/4 → 3/1
    assert_eq!(board_stats(&g, 0)[0], (3, 1));
    let champion = g.player(0).board[2];
    g.entity_mut(champion).health = 1;
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 2))); // 打死暴風城勇士
    assert_eq!(board_stats(&g, 0), vec![(2, 1), (2, 3)]);
    assert_eq!(board_entity(&g, 0, 0).max_health, 3);
}

// E5：永久附魔保留；複製對局時一起複製
#[test]
fn e5_permanent_enchantments() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    let hand = g.player(0).hand.len();
    cast(&mut g, "CORE_CS2_004", None, Some(minion(0, 0))); // 真言術：盾 +2 生命，抽一張
    assert_eq!(board_stats(&g, 0)[0], (2, 5));
    assert_eq!(g.player(0).hand.len(), hand + 1);
    cast(&mut g, "CORE_CS2_009", None, Some(minion(0, 0))); // 野性印記 +2/+3 嘲諷
    assert_eq!(board_stats(&g, 0)[0], (4, 8));
    assert!(board_entity(&g, 0, 0).taunt);
    let copy = g.clone();
    end(&mut g);
    end(&mut g);
    assert_eq!(board_stats(&g, 0)[0], (4, 8));
    assert_eq!(board_stats(&copy, 0)[0], (4, 8));
}

// E6：本回合附魔在回合結束時失效
#[test]
fn e6_this_turn_buff_expires() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    cast(&mut g, "CORE_CS2_188", Some(1), Some(minion(0, 0))); // 嚴厲的士官
    assert_eq!(board_stats(&g, 0)[0], (4, 3));
    end(&mut g);
    assert_eq!(board_stats(&g, 0)[0], (2, 3));
}

// E7：受傷時才有的加成：受傷就生效、回滿就失效
#[test]
fn e7_while_damaged_bonus() {
    let mut g = start_with(PRIEST, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CORE_OG_218"); // 血蹄勇戰者 2/6
    assert_eq!(board_stats(&g, 0)[0], (2, 6));
    cast(&mut g, "CORE_DS1_185", None, Some(minion(0, 0))); // 秘法射擊 2 點
    assert_eq!(board_stats(&g, 0)[0], (5, 4));
    act(&mut g, power(Some(minion(0, 0)))); // 次級治療術 +2
    assert_eq!(board_stats(&g, 0)[0], (2, 6));
}
