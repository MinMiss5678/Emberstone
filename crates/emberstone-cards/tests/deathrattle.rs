//! 第 2 階段 D：死亡與死亡之聲（case D1–D6）。D3 的盤面取自使用者 2026-11-01 的實戰紀錄。

mod common;

use common::*;
use emberstone_core::Game;

/// 先手依序放上手下（上場順序就是陣列順序），pos 是放的位置。
fn place(g: &mut Game, player: usize, minions: &[(&'static str, usize)]) {
    for &(id, pos) in minions {
        g.summon(player, id, pos).expect("放不上戰場");
    }
}

/// 先手打出地獄烈焰（對全部角色造成 3 點傷害）。
fn hellfire(g: &mut Game) {
    cast(g, "CORE_CS2_062", None, None);
}

// D1：多隻同時死亡：先全部移出戰場，再照上場順序（不分敵我）結算死亡之聲
#[test]
fn d1_deathrattles_in_play_order_across_sides() {
    // 先手的憎惡體先上場：牠的死亡之聲（全部 2 點）先結算，後手麥田魔像的衍生物之後才召喚，活下來
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("EX1_097", 0)]);
    place(&mut g, 1, &[("EX1_556", 0)]);
    for p in 0..2 {
        let e = g.player(p).board[0];
        g.entity_mut(e).health = 1;
    }
    hellfire(&mut g);
    assert!(g.player(0).board.is_empty());
    assert_eq!(board_ids(&g, 1), vec!["skele21"]);

    // 後手的麥田魔像先上場：衍生物先召喚，再被憎惡體的死亡之聲打死
    let mut g = start(&[], &[]);
    place(&mut g, 1, &[("EX1_556", 0)]);
    place(&mut g, 0, &[("EX1_097", 0)]);
    for p in 0..2 {
        let e = g.player(p).board[0];
        g.entity_mut(e).health = 1;
    }
    hellfire(&mut g);
    assert!(g.player(1).board.is_empty());
}

// D6：兩隻憎惡體同時死亡；第一個死亡之聲打死的雪人留在戰場，第二個照樣打到它，白沫狂戰士也照算
#[test]
fn d6_mortally_wounded_minion_still_hit_by_later_deathrattle() {
    let mut g = start(&[], &[]);
    place(
        &mut g,
        0,
        &[
            ("EX1_097", 0),
            ("EX1_097", 1),
            ("CORE_EX1_604", 2),
            ("CS2_182", 3),
        ],
    );
    let ids = g.player(0).board.clone();
    g.entity_mut(ids[0]).health = 1;
    g.entity_mut(ids[1]).health = 1;
    let frothing = g.entity_mut(ids[2]);
    frothing.max_health = 20;
    frothing.health = 20;
    g.entity_mut(ids[3]).health = 5; // 地獄烈焰後 2，第一個死亡之聲打到 0
    hellfire(&mut g);
    // 地獄烈焰 4 隻受傷 +4；兩個死亡之聲各打到狂戰士和雪人 +2、+2
    assert_eq!(board_ids(&g, 0), vec!["CORE_EX1_604"]);
    assert_eq!(board_stats(&g, 0), vec![(10, 13)]);
}

// D2：死亡之聲引起新的死亡 → 重複死亡階段
#[test]
fn d2_deathrattle_chain() {
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("EX1_097", 0), ("CORE_EX1_096", 1)]); // 憎惡體、掠寶囤積者
    let hand = g.player(0).hand.len();
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0))); // 火球術打死憎惡體
    assert!(g.player(0).board.is_empty());
    assert_eq!(g.player(0).hand.len(), hand + 1); // 囤積者被 2 點打死，抽一張
    assert_eq!((hero_health(&g, 0), hero_health(&g, 1)), (28, 28));
}

// D3：13:30:05 盤面：麥田魔像（後下、左）、鬼怪蠕行者（先下、右），一起死
#[test]
fn d3_both_summoners_die_pushes_later_marker() {
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("FP1_002", 0), ("EX1_556", 0)]);
    assert_eq!(board_ids(&g, 0), vec!["EX1_556", "FP1_002"]);
    hellfire(&mut g);
    assert_eq!(board_ids(&g, 0), vec!["FP1_002t", "FP1_002t", "skele21"]);
}

// D3：13:21:29 盤面：著魔的村民（先）、鬼怪蠕行者（後）、右邊存活一隻
#[test]
fn d3_position_from_removal_order() {
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("OG_241", 0), ("FP1_002", 1), ("CS2_182", 2)]);
    hellfire(&mut g);
    assert_eq!(
        board_ids(&g, 0),
        vec!["OG_241a", "FP1_002t", "FP1_002t", "CS2_182"]
    );
}

// D3：12:57:49 盤面：恩若司之子、冰風雪人（存活）、鬼怪蠕行者、著魔的村民（上場順序：之子、雪人、村民、蠕行者）
#[test]
fn d3_survivor_left_and_push() {
    let mut g = start(&[], &[]);
    place(
        &mut g,
        0,
        &[("OG_256", 0), ("CS2_182", 1), ("OG_241", 2), ("FP1_002", 2)],
    );
    assert_eq!(
        board_ids(&g, 0),
        vec!["OG_256", "CS2_182", "FP1_002", "OG_241"]
    );
    hellfire(&mut g);
    assert_eq!(
        board_ids(&g, 0),
        vec!["CS2_182", "OG_241a", "FP1_002t", "FP1_002t"]
    );
}

// D3：12:17:58 盤面縮小版：麥田魔像（第 2 個上場）、奈幽之卵（第 1 個）、村民（第 3）、奈幽之卵（第 4），全部陣亡
#[test]
fn d3_full_wipe_lines_up_in_resolution_order() {
    let mut g = start(&[], &[]);
    place(
        &mut g,
        0,
        &[
            ("FP1_007", 0),
            ("EX1_556", 0),
            ("OG_241", 2),
            ("FP1_007", 3),
        ],
    );
    assert_eq!(
        board_ids(&g, 0),
        vec!["EX1_556", "FP1_007", "OG_241", "FP1_007"]
    );
    hellfire(&mut g);
    assert_eq!(
        board_ids(&g, 0),
        vec!["FP1_007t", "skele21", "OG_241a", "FP1_007t"]
    );
}

// D3：單隻死亡：衍生物放在原本的位置
#[test]
fn d3_single_death_same_position() {
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("CS2_182", 0), ("FP1_002", 1), ("CS2_182", 2)]);
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 1)));
    assert_eq!(
        board_ids(&g, 0),
        vec!["CS2_182", "FP1_002t", "FP1_002t", "CS2_182"]
    );
}

// D4：死亡之聲抽牌、加護甲
#[test]
fn d4_draw_and_armor() {
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("CORE_EX1_096", 0), ("CORE_SW_068", 1)]);
    let hand = g.player(0).hand.len();
    for p in 0..2 {
        let e = g.player(0).board[p];
        g.entity_mut(e).health = 1;
    }
    hellfire(&mut g);
    assert_eq!(g.player(0).hand.len(), hand + 1);
    assert_eq!(hero_entity(&g, 0).armor, 8);
}

// D5：同一批死亡之聲依序結算、各自看當下的戰場
#[test]
fn d5_later_deathrattle_buffs_earlier_token() {
    let mut g = start(&[], &[]);
    place(&mut g, 0, &[("OG_256", 0), ("EX1_556", 1), ("OG_256", 2)]);
    hellfire(&mut g);
    assert_eq!(board_ids(&g, 0), vec!["skele21"]);
    assert_eq!(board_stats(&g, 0), vec![(3, 2)]);
}
