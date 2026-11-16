//! 第 3a 階段 C、D、E：召喚、摧毀、數值、被動加成與觸發、秘密（`docs/第3a階段-case表.md`）。

mod common;

use common::*;
use emberstone_core::{Action, Game, RngEvent, Target};

/// 在 player 的戰場放 n 隻淡水鱷。
fn fill_board(g: &mut Game, player: usize, n: usize) {
    for _ in 0..n {
        ready(g, player, FILL);
    }
}

fn play_choose(g: &mut Game, id: &'static str, which: u8, target: Option<Target>) {
    let i = give(g, 0, id);
    g.player_mut(0).mana = 10;
    act(g, choose_one(i, None, target, which));
}

// C1：糾纏之根 2 召喚兩個樹苗放最右邊；空位不夠能放幾隻放幾隻；戰場滿了不能選召喚
#[test]
fn c1_living_roots_summons_fill_board() {
    let mut g = start(&[], &[]);
    fill_board(&mut g, 0, 5);
    play_choose(&mut g, "CORE_AT_037", 2, None);
    assert_eq!(&board_ids(&g, 0)[5..], &["AT_037t", "AT_037t"]);
    let mut g = start(&[], &[]);
    fill_board(&mut g, 0, 6);
    play_choose(&mut g, "CORE_AT_037", 2, None);
    assert_eq!(g.player(0).board.len(), 7);
    let i = give(&mut g, 0, "CORE_AT_037");
    g.player_mut(0).mana = 10;
    let options: Vec<u8> = g
        .legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Play {
                hand, choose_one, ..
            } if hand == i => Some(choose_one),
            _ => None,
        })
        .collect();
    assert!(options.iter().all(|&w| w == 1));
    play_choose(&mut g, "CORE_AT_037", 1, Some(hero(1))); // 1：造成 2 點
    assert_eq!(hero_health(&g, 1), 28);
}

// C1：揮尾好朋友 2 召喚六隻海獺，空位不夠只放到滿；1 召喚 6/6 嘲諷虎鯨
#[test]
fn c1_flipper_friends() {
    let mut g = start(&[], &[]);
    fill_board(&mut g, 0, 3);
    play_choose(&mut g, "CORE_TSC_650", 2, None);
    assert_eq!(&board_ids(&g, 0)[3..], &["TSC_650t4"; 4]);
    let mut g = start(&[], &[]);
    play_choose(&mut g, "CORE_TSC_650", 1, None);
    assert_eq!(board_ids(&g, 0), vec!["TSC_650t"]);
    assert!(board_entity(&g, 0, 0).taunt);
}

// C1：惡魔襲擊 3 點傷害（加法術傷害）＋兩個 1/3 嘲諷虛無行者放最右邊
#[test]
fn c1_demonic_assault() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS3_007");
    cast(&mut g, "CORE_SW_088", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 26);
    assert_eq!(board_ids(&g, 0), vec!["CS3_007", "CS2_065", "CS2_065"]);
}

// C2：獵物致傷的土狼、海獺有衝刺：當回合能攻擊手下、不能攻擊英雄
#[test]
fn c2_rush_tokens() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, FILL);
    cast(&mut g, "CORE_BAR_801", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 29);
    assert_eq!(board_ids(&g, 0), vec!["BAR_035t"]);
    let attacks: Vec<_> = g
        .legal_actions()
        .into_iter()
        .filter(|a| matches!(a, Action::Attack { .. }))
        .collect();
    assert_eq!(attacks, vec![attack(minion(0, 0), minion(1, 0))]);
}

// C3：動物夥伴從米莎、雷歐克、霍弗隨機召喚一隻（抽法同 RosettaStone：Int(0, 2)）
#[test]
fn c3_animal_companion() {
    for (value, id) in [(0, "NEW1_032"), (1, "NEW1_033"), (2, "NEW1_034")] {
        let mut g = start_scripted(
            HUNTER,
            WARRIOR,
            vec![RngEvent::Int {
                lo: 0,
                hi: 2,
                value,
            }],
        );
        cast(&mut g, "CORE_NEW1_031", None, None);
        assert_eq!(board_ids(&g, 0), vec![id]);
    }
    let mut g = start(&[], &[]);
    fill_board(&mut g, 0, 7);
    let i = give(&mut g, 0, "CORE_NEW1_031");
    g.player_mut(0).mana = 10;
    assert!(play_targets(&g, i).is_empty());
}

// C4：暗言術：滅摧毀全部攻擊力 ≥ 5（含被動加成）的手下
#[test]
fn c4_shadow_word_ruin() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_CS2_122"); // 團隊隊長 2/3
    ready(&mut g, 0, "CS2_182"); // 冰風雪人 4/5 → 5 攻
    ready(&mut g, 1, "CS2_182"); // 4 攻，留下
    ready(&mut g, 1, "CS2_200"); // 巨型奧格 6/7
    cast(&mut g, "CORE_EX1_197", None, None);
    assert_eq!(board_ids(&g, 0), vec!["CORE_CS2_122"]);
    assert_eq!(board_ids(&g, 1), vec!["CS2_182"]);
}

// C5：惡魔斬隙者：摧毀手下，英雄受到等同其目前生命的傷害
#[test]
fn c5_riftcleaver() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182");
    let e = g.player(1).board[0];
    g.entity_mut(e).health = 3;
    cast(&mut g, "CORE_ULD_165", Some(0), Some(minion(1, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 0), 27);
}

// C6：一視同仁：全部手下生命設為 1，之後再套用被動加成
#[test]
fn c6_equality_then_auras() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_CS2_222"); // 暴風城勇士 7/7
    ready(&mut g, 0, "CS2_182"); // 5/6
    ready(&mut g, 1, "CS2_182");
    let e = g.player(1).board[0];
    g.entity_mut(e).health = 2;
    cast(&mut g, "CORE_EX1_619", None, None);
    assert_eq!(board_stats(&g, 0), vec![(7, 1), (5, 2)]);
    assert_eq!(board_stats(&g, 1), vec![(4, 1)]);
    assert_eq!(board_entity(&g, 1, 0).max_health, 1);
}

// C7：瘋狂的鍊金師：對調目前攻擊力與目前生命；被動加成併入後再套用一次（實測 R2）
#[test]
fn c7_crazed_alchemist_swap() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_CS2_222");
    ready(&mut g, 0, "CS2_182"); // 5/6
    cast(&mut g, "CORE_EX1_059", Some(2), Some(minion(0, 1)));
    assert_eq!(board_entity(&g, 0, 1).attack, 7);
    assert_eq!(board_entity(&g, 0, 1).health, 6);
    // 受傷的用目前生命
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182");
    let e = g.player(1).board[0];
    g.entity_mut(e).health = 3;
    cast(&mut g, "CORE_EX1_059", Some(0), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(3, 4)]);
    assert_eq!(board_entity(&g, 1, 0).max_health, 4);
    // 聖盾保留；攻擊力 0 的對調後死亡，聖盾也救不了
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CORE_GVG_085"); // 1/2 聖盾
    cast(&mut g, "CORE_EX1_059", Some(0), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(2, 1)]);
    assert!(board_entity(&g, 1, 0).divine_shield);
    let e = g.player(1).board[0];
    g.entity_mut(e).attack = 0;
    cast(&mut g, "CORE_EX1_059", Some(0), Some(minion(1, 0)));
    assert!(g.player(1).board.is_empty());
}

// C8：暮光飛龍：手牌每張 +1 生命（不含自己）
#[test]
fn c8_twilight_drake() {
    let mut g = start(&[], &[]);
    let n = g.player(0).hand.len() as i32;
    cast(&mut g, "CORE_EX1_043", Some(0), None);
    assert_eq!(board_stats(&g, 0), vec![(4, 1 + n)]);
    assert_eq!(board_entity(&g, 0, 0).max_health, 1 + n);
}

// C9：血帆劫掠者：獲得等同武器攻擊力的攻擊力；沒有武器時不變
#[test]
fn c9_bloodsail_raider() {
    let mut g = start_with(ROGUE, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_NEW1_018", Some(0), None);
    assert_eq!(board_stats(&g, 0), vec![(2, 3)]);
    act(&mut g, power(None));
    cast(&mut g, "CORE_NEW1_018", Some(1), None);
    assert_eq!(board_stats(&g, 0)[1], (3, 3));
}

// C10：受傷的托維爾人：戰吼對自己 3 點，會觸發受傷效果
#[test]
fn c10_injured_tolvir() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_604"); // 白沫狂戰士
    cast(&mut g, "CORE_ULD_271", Some(1), None);
    assert_eq!(board_stats(&g, 0), vec![(3, 4), (2, 3)]);
    assert!(board_entity(&g, 0, 1).taunt);
}

// C11：冷光先知：其他友方魚人永久 +2 生命
#[test]
fn c11_coldlight_seer() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_168"); // 魚人劫掠者 2/1
    ready(&mut g, 0, FILL);
    ready(&mut g, 1, "CS2_168");
    cast(&mut g, "CORE_EX1_103", Some(2), None);
    assert_eq!(board_stats(&g, 0), vec![(2, 3), (2, 3), (2, 3)]);
    assert_eq!(board_stats(&g, 1), vec![(2, 1)]);
}

// C12：薩克特盜匪：死亡之聲把一隻隨機敵方手下送回手牌；手牌滿了摧毀；沒有手下沒有效果
#[test]
fn c12_sahket_sapper() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_ULD_280");
    ready(&mut g, 1, "CS2_182");
    let hand = g.player(1).hand.len();
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(g.player(1).hand.len(), hand + 1);
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CS2_182"));
    // 手牌滿了：摧毀
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_ULD_280");
    ready(&mut g, 1, "CS2_182");
    while g.player(1).hand.len() < 10 {
        give(&mut g, 1, FILL);
    }
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(g.player(1).hand.len(), 10);
    // 沒有敵方手下
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_ULD_280");
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0)));
    assert!(g.player(0).board.is_empty());
}

// C13：第一道火焰：2 點傷害後放一張第二道火焰到手牌（打出後手牌一定有空位）
#[test]
fn c13_first_flame() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182");
    cast(&mut g, "CORE_SW_108", None, Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(4, 3)]);
    assert_eq!(hand_ids(&g, 0).last(), Some(&"SW_108t"));
    while g.player(0).hand.len() < 9 {
        give(&mut g, 0, FILL);
    }
    cast(&mut g, "CORE_SW_108", None, Some(minion(1, 0)));
    assert_eq!(g.player(0).hand.len(), 10);
    assert_eq!(hand_ids(&g, 0).last(), Some(&"SW_108t"));
}

// D1：魚人隊長（其他友方魚人 +2 攻）、南海船長（其他友方海盜 +1/+1）
#[test]
fn d1_tribal_auras() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_168"); // 魚人
    ready(&mut g, 0, "CORE_NEW1_018"); // 血帆劫掠者（海盜）
    ready(&mut g, 1, "CS2_168");
    ready(&mut g, 0, "CORE_EX1_507");
    ready(&mut g, 0, "CORE_NEW1_027");
    assert_eq!(board_stats(&g, 0), vec![(4, 1), (3, 4), (3, 3), (3, 3)]);
    assert_eq!(board_stats(&g, 1), vec![(2, 1)]);
}

// D2：焦油蠕行者：在對手的回合 +2 攻擊力
#[test]
fn d2_tar_creeper() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_UNG_928");
    assert_eq!(board_entity(&g, 0, 0).attack, 1);
    end(&mut g);
    assert_eq!(board_entity(&g, 0, 0).attack, 3);
    end(&mut g);
    assert_eq!(board_entity(&g, 0, 0).attack, 1);
}

// D3：葛羅瑪許‧地獄吼：衝鋒；受傷時 +6 攻擊力
#[test]
fn d3_grommash() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_414", Some(0), None);
    assert!(g.legal_actions().contains(&attack(minion(0, 0), hero(1))));
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0)));
    assert_eq!(board_stats(&g, 0), vec![(10, 3)]);
}

// D4：哀嚎水氣：打出元素手下（戰吼結算後）+1；召喚的、自己、對手打出的都不算（實測 R3）
#[test]
fn d4_wailing_vapor() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_WC_042", Some(0), None);
    assert_eq!(board_entity(&g, 0, 0).attack, 1);
    cast(&mut g, "CORE_CS2_042", Some(1), Some(hero(1))); // 火元素（元素）
    assert_eq!(board_entity(&g, 0, 0).attack, 2);
    cast(&mut g, "CS2_182", Some(2), None); // 不是元素
    assert_eq!(board_entity(&g, 0, 0).attack, 2);
    g.summon(0, "CORE_UNG_928", 3); // 召喚的焦油蠕行者（元素）
    assert_eq!(board_entity(&g, 0, 0).attack, 2);
    end(&mut g);
    cast(&mut g, "CORE_UNG_928", Some(0), None); // 對手打出
    assert_eq!(board_entity(&g, 0, 0).attack, 2);
}

/// 惡魔獵人英雄先用惡魔之爪（+1 攻擊力），場上放一隻伊利達瑞審判官。
fn inquisitor_game() -> Game {
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    g.player_mut(0).mana = 10;
    act(&mut g, power(None));
    g
}

// D5：伊利達瑞審判官：英雄攻擊後也攻擊同一目標；剛上場（衝刺）也照樣打英雄；不佔它的攻擊次數（實測 R4）
#[test]
fn d5_inquisitor_follows_hero() {
    let mut g = inquisitor_game();
    cast(&mut g, "CS3_020", Some(0), None);
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 21);
    // 跟著攻擊照樣記一次攻擊（同實戰紀錄的 NUM_ATTACKS_THIS_TURN），但不佔它自己的攻擊次數
    assert_eq!(board_entity(&g, 0, 0).attacks_this_turn, 1);
    ready(&mut g, 1, FILL);
    assert!(
        g.legal_actions()
            .contains(&attack(minion(0, 0), minion(1, 0)))
    );
}

// D5：凍結、本回合已攻擊過照樣跟著攻擊；目標已死亡就不攻擊
#[test]
fn d5_inquisitor_frozen_or_attacked() {
    let mut g = inquisitor_game();
    let i = ready(&mut g, 0, "CS3_020");
    let e = g.player(0).board[i];
    g.entity_mut(e).frozen = true;
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 21);
    let mut g = inquisitor_game();
    ready(&mut g, 0, "CS3_020");
    ready(&mut g, 1, FILL);
    ready(&mut g, 1, FILL);
    act(&mut g, attack(minion(0, 0), minion(1, 0)));
    act(&mut g, attack(hero(0), hero(1)));
    assert_eq!(hero_health(&g, 1), 21);
    // 英雄打死目標：審判官不攻擊
    let mut g = inquisitor_game();
    ready(&mut g, 0, "CS3_020");
    ready(&mut g, 1, "CS2_168"); // 2/1
    act(&mut g, attack(hero(0), minion(1, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(board_entity(&g, 0, 0).health, 8);
}

// D5：審判官跟著攻擊會觸發對手的攻擊類秘密（冰凍陷阱把它送回手牌）
#[test]
fn d5_inquisitor_triggers_secrets() {
    let mut g = start_with(HUNTER, &[], DEMON_HUNTER, &[]);
    cast(&mut g, "CORE_EX1_611", None, None); // 冰凍陷阱
    end(&mut g);
    g.player_mut(1).mana = 10;
    act(&mut g, power(None));
    ready(&mut g, 1, "CS3_020");
    act(&mut g, attack(hero(1), hero(0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CS3_020"));
    assert_eq!(hero_health(&g, 0), 29);
}

// D6：戰槌挑戰者：互相攻擊到一方死亡
#[test]
fn d6_warmaul_challenger_to_the_death() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_172"); // 血沼迅猛龍 3/2
    cast(&mut g, "CORE_BT_120", Some(0), Some(minion(1, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(board_stats(&g, 0), vec![(1, 4)]);
}

// D6：最多攻擊 30 次（實測 R5）
#[test]
fn d6_warmaul_challenger_stops_after_30() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, FILL);
    let e = g.player(1).board[0];
    g.entity_mut(e).attack = 0;
    g.entity_mut(e).max_health = 40;
    g.entity_mut(e).health = 40;
    cast(&mut g, "CORE_BT_120", Some(0), Some(minion(1, 0)));
    assert_eq!(board_entity(&g, 1, 0).health, 10);
    assert_eq!(board_entity(&g, 0, 0).health, 10);
}

// D6：每次攻擊都觸發攻擊類秘密；挑戰者被冰凍陷阱送回手牌就停止
#[test]
fn d6_warmaul_challenger_secret_ends_duel() {
    let mut g = start_with(WARRIOR, &[], HUNTER, &[]);
    end(&mut g);
    cast(&mut g, "CORE_EX1_611", None, None);
    ready(&mut g, 1, "CS2_182");
    end(&mut g);
    cast(&mut g, "CORE_BT_120", Some(0), Some(minion(1, 0)));
    assert!(g.player(0).board.is_empty());
    assert_eq!(hand_ids(&g, 0).last(), Some(&"CORE_BT_120"));
    assert_eq!(board_stats(&g, 1), vec![(4, 5)]);
}

// E1：綠洲盟友：友方手下被攻擊時，攻擊前召喚 3/6 水元素，攻擊照常；英雄被攻擊不觸發；戰場滿了不觸發
#[test]
fn e1_oasis_ally() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, FILL);
    cast(&mut g, "CORE_BAR_812", None, None);
    end(&mut g);
    let a = ready(&mut g, 1, "CS2_182");
    act(&mut g, attack(minion(1, a), hero(0)));
    assert_eq!(g.player(0).secrets.len(), 1);
    end(&mut g);
    end(&mut g);
    act(&mut g, attack(minion(1, a), minion(0, 0)));
    assert!(g.player(0).secrets.is_empty());
    assert_eq!(board_ids(&g, 0), vec!["CS2_033"]);
    // 戰場滿了：不觸發、留在場上
    let mut g = start(&[], &[]);
    fill_board(&mut g, 0, 7);
    cast(&mut g, "CORE_BAR_812", None, None);
    end(&mut g);
    let a = ready(&mut g, 1, "CS2_182");
    act(&mut g, attack(minion(1, a), minion(0, 0)));
    assert_eq!(g.player(0).secrets.len(), 1);
}

// E2：踏板陷阱：對手施放法術後摧毀對手一隻隨機手下；對手沒有手下時留在場上（實測 R6）；被反制時不觸發
#[test]
fn e2_pressure_plate() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_ULD_152", None, None);
    end(&mut g);
    cast(&mut g, "CORE_DS1_185", None, Some(hero(0)));
    assert_eq!(g.player(0).secrets.len(), 1);
    ready(&mut g, 1, "CS2_182");
    cast(&mut g, "CORE_DS1_185", None, Some(hero(0)));
    assert!(g.player(0).secrets.is_empty());
    assert!(g.player(1).board.is_empty());
    // 法術反制先觸發：被反制的法術不觸發踏板陷阱
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_287", None, None);
    cast(&mut g, "CORE_ULD_152", None, None);
    end(&mut g);
    ready(&mut g, 1, "CS2_182");
    cast(&mut g, "CORE_DS1_185", None, Some(hero(0)));
    assert_eq!(hero_health(&g, 0), 30);
    assert_eq!(g.player(0).secrets.len(), 1);
    assert_eq!(g.player(1).board.len(), 1);
}

// C7：鍊金師對調受傷（激怒）的手下：對調後回滿生命，不再受傷，激怒加成不再套用
#[test]
fn c7_alchemist_swaps_enraged_minion() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CORE_EX1_414"); // 葛羅瑪許 4/9
    cast(&mut g, "CORE_CS2_029", None, Some(minion(1, 0))); // 火球術 → 10/3
    assert_eq!(board_stats(&g, 1), vec![(10, 3)]);
    cast(&mut g, "CORE_EX1_059", Some(0), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(3, 10)]);
}

// H1：鍊金師對調對手回合的焦油蠕行者：+2 併入後再套用一次（實測 R7）
#[test]
fn h1_alchemist_tar_creeper_on_opponent_turn() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CORE_UNG_928"); // 對手的焦油蠕行者，現在是對手回合 → 3/5
    assert_eq!(board_stats(&g, 1), vec![(3, 5)]);
    cast(&mut g, "CORE_EX1_059", Some(0), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(7, 3)]);
}

// H2：沉默讓生命上限變高時，目前生命增加同樣的量（實測 R8）
#[test]
fn h2_silence_raises_health_with_max() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182"); // 4/5
    cast(&mut g, "CORE_EX1_619", None, None); // 一視同仁 → 4/1
    cast(&mut g, "CORE_CS2_203", Some(0), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(4, 5)]);
    // 被鍊金師對調過（4/5 → 5/4）再沉默 → 4/5
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CS2_182");
    cast(&mut g, "CORE_EX1_059", Some(0), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(5, 4)]);
    cast(&mut g, "CORE_CS2_203", Some(1), Some(minion(1, 0)));
    assert_eq!(board_stats(&g, 1), vec![(4, 5)]);
}

// H3：有超載的法術被法術反制，不套用超載（Wiki Counterspell；實測 R9）
#[test]
fn h3_countered_spell_no_overload() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_287", None, None); // 法術反制
    end(&mut g);
    cast(&mut g, "CORE_EX1_238", None, Some(hero(0))); // 閃電箭（超載 1）
    assert_eq!(hero_health(&g, 0), 30);
    assert_eq!(g.player(1).overload_owed, 0);
}

// E2：打出秘密也是施放法術，會觸發對手的踏板陷阱
#[test]
fn e2_pressure_plate_triggered_by_secret() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_ULD_152", None, None);
    end(&mut g);
    ready(&mut g, 1, "CS2_182");
    cast(&mut g, "CORE_EX1_611", None, None); // 對手打出冰凍陷阱（秘密）
    assert!(g.player(0).secrets.is_empty());
    assert!(g.player(1).board.is_empty());
    assert_eq!(g.player(1).secrets.len(), 1);
}
