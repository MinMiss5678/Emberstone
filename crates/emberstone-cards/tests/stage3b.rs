//! 第 3b 階段：流放、費用調整、休眠、發現、複製卡片、召喚加倍、洗入牌堆（`docs/第3b階段-case表.md`）。

mod common;

use common::*;
use emberstone_core::{Action, Game, RngEvent, Target};

/// 手牌第 from 張移到第 to 張。
fn move_card(g: &mut Game, player: usize, from: usize, to: usize) {
    let hand = &mut g.player_mut(player).hand;
    let e = hand.remove(from);
    hand.insert(to, e);
}

/// 手牌中最後一張 id 的位置。
fn hand_pos(g: &Game, player: usize, id: &str) -> usize {
    hand_ids(g, player)
        .iter()
        .rposition(|&c| c == id)
        .unwrap_or_else(|| panic!("手牌沒有 {id}"))
}

/// 手牌第 hand 張的目前費用。
fn cost_at(g: &Game, player: usize, hand: usize) -> i32 {
    g.hand_cost(g.player(player).hand[hand])
}

/// 合法行動裡有沒有打出手牌第 hand 張。
fn can_play(g: &Game, hand: usize) -> bool {
    g.legal_actions()
        .iter()
        .any(|a| matches!(a, Action::Play { hand: h, .. } if *h == hand))
}

/// attacker 可以攻擊的目標。
fn attack_options(g: &Game, attacker: Target) -> Vec<Target> {
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Attack {
                attacker: a,
                target,
            } if a == attacker => Some(target),
            _ => None,
        })
        .collect()
}

fn fill_board(g: &mut Game, player: usize, n: usize) {
    for _ in 0..n {
        ready(g, player, FILL);
    }
}

// ---- A. 流放與費用調整 ----

// A1、A2：靈視從最左、最右打出（或只剩這張）時流放成立，多抽一張；在中間只抽一張
#[test]
fn a2_spectral_sight_outcast() {
    for (to, drawn) in [(None, 2), (Some(0), 2), (Some(1), 1)] {
        let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
        let i = give(&mut g, 0, "CORE_BT_491");
        let i = match to {
            Some(t) => {
                move_card(&mut g, 0, i, t);
                t
            }
            None => i,
        };
        g.player_mut(0).mana = 10;
        let before = g.player(0).hand.len();
        act(&mut g, play(i, None, None));
        assert_eq!(g.player(0).hand.len(), before - 1 + drawn, "位置 {to:?}");
    }
    // 手牌只剩這張
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    g.player_mut(0).hand.clear();
    let i = give(&mut g, 0, "CORE_BT_491");
    g.player_mut(0).mana = 10;
    act(&mut g, play(i, None, None));
    assert_eq!(g.player(0).hand.len(), 2);
}

// A3：魔眼光束在手牌最左或最右時費用 1，位置改變就跟著變；3 點傷害、生命竊取
#[test]
fn a3_eye_beam_outcast_cost() {
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    ready(&mut g, 1, "CS2_182"); // 冰風雪人 4/5
    let i = give(&mut g, 0, "CORE_BT_801");
    assert_eq!(cost_at(&g, 0, i), 1);
    move_card(&mut g, 0, i, 1);
    assert_eq!(cost_at(&g, 0, 1), 3);
    move_card(&mut g, 0, 1, 0);
    assert_eq!(cost_at(&g, 0, 0), 1);
    set_hero_health(&mut g, 0, 20);
    g.player_mut(0).mana = 1;
    act(&mut g, play(0, None, Some(minion(1, 0))));
    assert_eq!(board_stats(&g, 1), vec![(4, 2)]);
    assert_eq!(hero_health(&g, 0), 23);
    assert_eq!(g.player(0).mana, 0);
}

// A4、A5：準備讓本回合下一個法術 −2；手牌裡的法術都顯示 −2，手下不變；打出法術就用掉
#[test]
fn a4_preparation_next_spell() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_145", None, None);
    let f = give(&mut g, 0, "CORE_CS2_029"); // 火球術 4
    let m = give(&mut g, 0, FILL);
    assert_eq!(cost_at(&g, 0, f), 2);
    assert_eq!(cost_at(&g, 0, m), 2);
    g.player_mut(0).mana = 2;
    assert!(can_play(&g, f));
    act(&mut g, play(f, None, Some(hero(1))));
    assert_eq!(g.player(0).mana, 0);
    assert_eq!(hero_health(&g, 1), 24);
    let f = give(&mut g, 0, "CORE_CS2_029");
    assert_eq!(cost_at(&g, 0, f), 4);
}

// A4：準備在回合結束失效
#[test]
fn a4_preparation_expires_at_end_of_turn() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_145", None, None);
    end(&mut g);
    end(&mut g);
    let f = give(&mut g, 0, "CORE_CS2_029");
    assert_eq!(cost_at(&g, 0, f), 4);
}

// A4：幸運幣、另一張準備也是法術，打出就用掉；第二張準備的效果照常
#[test]
fn a4_preparation_used_by_any_spell() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_145", None, None);
    cast(&mut g, COIN, None, None);
    let f = give(&mut g, 0, "CORE_CS2_029");
    assert_eq!(cost_at(&g, 0, f), 4);
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_145", None, None);
    cast(&mut g, "CORE_EX1_145", None, None);
    let f = give(&mut g, 0, "CORE_CS2_029");
    assert_eq!(cost_at(&g, 0, f), 2);
}

// A4：被法術反制的法術也用掉準備（提案）
#[test]
fn a4_preparation_used_by_countered_spell() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_145", None, None);
    // 準備之後才讓對手有法術反制（直接放進秘密區）
    let s = g.add_to_hand(1, "CORE_EX1_287").expect("加不進手牌");
    g.player_mut(1).hand.pop();
    g.player_mut(1).secrets.push(s);
    cast(&mut g, "CORE_CS2_029", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 30);
    assert!(g.player(1).secrets.is_empty());
    let f = give(&mut g, 0, "CORE_CS2_029");
    assert_eq!(cost_at(&g, 0, f), 4);
}

// A6：「設為 1」再加準備 −2 → 0（提案）
#[test]
fn a6_set_cost_then_preparation() {
    let mut g = start_with(DEMON_HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_145", None, None);
    let i = give(&mut g, 0, "CORE_BT_801");
    assert_eq!(cost_at(&g, 0, i), 0);
}

// ---- B. 休眠 ----

// B1、B3：被囚禁的邪惡犬魔以休眠上場：不能攻擊、不能被指定或攻擊、不受全體效果與被動加成影響
#[test]
fn b1_b3_felhound_dormant() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_BT_156", Some(0), None);
    let e = g.player(0).board[0];
    assert!(g.entity(e).dormant > 0);
    ready(&mut g, 0, "CORE_CS2_222"); // 暴風城勇士
    assert_eq!(board_stats(&g, 0)[0], (3, 5));
    g.entity_mut(e).summoning_sick = false;
    assert!(attack_options(&g, minion(0, 0)).is_empty());
    end(&mut g);
    let i = give(&mut g, 1, "CORE_CS2_029");
    g.player_mut(1).mana = 10;
    assert!(!play_targets(&g, i).contains(&Some(minion(0, 0))));
    ready(&mut g, 1, "CS2_182");
    assert!(!attack_options(&g, minion(1, 0)).contains(&minion(0, 0)));
    cast(&mut g, "CORE_CS2_032", None, None); // 烈焰風暴
    assert_eq!(g.entity(e).health, 5);
    assert_eq!(board_stats(&g, 0), vec![(3, 5), (7, 2)]);
}

// B2、B4：打出後第二個自己的回合開始時甦醒；甦醒那回合視同剛上場（衝刺只能打手下），下一回合正常攻擊
#[test]
fn b2_b4_felhound_awakens() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_BT_156", Some(0), None);
    let e = g.player(0).board[0];
    end(&mut g);
    ready(&mut g, 1, "CS2_182");
    end(&mut g);
    assert!(g.entity(e).dormant > 0, "第一個自己的回合還在休眠");
    end(&mut g);
    end(&mut g);
    assert_eq!(g.entity(e).dormant, 0);
    assert_eq!(attack_options(&g, minion(0, 0)), vec![minion(1, 0)]);
    end(&mut g);
    end(&mut g);
    assert!(attack_options(&g, minion(0, 0)).contains(&hero(1)));
}

// case: 3b-B4a 剛上場時對手沒有手下，之後同一回合出現的手下可以攻擊（實測 R14）
#[test]
fn b4a_rush_can_attack_minion_appearing_later() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CS3_038", Some(0), None); // 紅鰓刃顎魚人（衝刺）
    ready(&mut g, 1, FILL);
    assert_eq!(attack_options(&g, minion(0, 0)), vec![minion(1, 0)]);
}

// case: 3b-B4a 甦醒時對手沒有手下，之後同一回合出現的手下可以攻擊（不能攻擊英雄：第 1 階段 C3）
#[test]
fn b4a_awakened_felhound_can_attack_minion_appearing_later() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_BT_156", Some(0), None);
    until_turn(&mut g, 5);
    ready(&mut g, 1, FILL);
    assert_eq!(attack_options(&g, minion(0, 0)), vec![minion(1, 0)]);
}

// case: 3b-B4b 甦醒那回合被凍結：回合結束時對手有手下 → 本回合結束就解凍
#[test]
fn b4b_awakened_felhound_frozen_thaws_if_enemy_at_end() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_BT_156", Some(0), None);
    until_turn(&mut g, 5);
    let e = g.player(0).board[0];
    g.entity_mut(e).frozen = true;
    ready(&mut g, 1, FILL);
    end(&mut g);
    assert!(!g.entity(e).frozen);
}

// B5：打出休眠手下不觸發對手的爆炸符文（實測 R3）
#[test]
fn b5_dormant_play_does_not_trigger_runes() {
    let mut g = start(&[], &[]);
    end(&mut g);
    cast(&mut g, "CORE_LOOT_101", None, None);
    end(&mut g);
    cast(&mut g, "CORE_BT_156", Some(0), None);
    assert_eq!(g.player(1).secrets.len(), 1);
    assert_eq!(board_stats(&g, 0), vec![(3, 5)]);
}

// B6：甦醒不算召喚：卡德加不會多複製一隻（實測 R11）
#[test]
fn b6_awakening_is_not_summon() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_DAL_575");
    cast(&mut g, "CORE_BT_156", Some(1), None);
    until_turn(&mut g, 5);
    assert_eq!(board_ids(&g, 0), vec!["CORE_DAL_575", "CORE_BT_156"]);
}

// ---- C. 發現（符文寶珠） ----

/// 隨機產生的候選：法師法術 4 張、盜賊法術 1 張、手下 1 張。
const POOL: &[&str] = &[
    "CORE_CS2_029",
    "CORE_CS2_023",
    "CORE_CS2_032",
    "CORE_EX1_287",
    "CORE_CS2_072",
    "CS2_182",
];
const MAGE_SPELLS: &[&str] = &[
    "CORE_CS2_029",
    "CORE_CS2_023",
    "CORE_CS2_032",
    "CORE_EX1_287",
];

fn start_pool(pool: &'static [&'static str]) -> Game {
    let mut cfg = config(MAGE, deck(&[], 30), WARRIOR, deck(&[], 30));
    cfg.random_pool = Some(pool);
    Game::new(cfg).expect("開局失敗")
}

/// 選擇畫面的選項卡號。
fn choice_ids(g: &Game) -> Vec<&'static str> {
    g.choice()
        .expect("沒有選擇畫面")
        .iter()
        .map(|&e| g.entity(e).card.id)
        .collect()
}

// C1、C2、C3：2 點傷害（加法術傷害）後，從職業法術隨機挑 3 張不重複；選的牌放進手牌
#[test]
fn c1_c3_rune_orb_discovers_class_spell() {
    let mut g = start_pool(POOL);
    ready(&mut g, 0, "CS3_007");
    cast(&mut g, "CORE_BAR_541", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 27);
    let options = choice_ids(&g);
    assert_eq!(options.len(), 3);
    assert!(
        options.iter().all(|id| MAGE_SPELLS.contains(id)),
        "{options:?}"
    );
    let mut dedup = options.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(dedup.len(), 3);
    act(&mut g, Action::Choose { index: 1 });
    assert_eq!(hand_ids(&g, 0).last(), Some(&options[1]));
    assert!(g.choice().is_none());
}

// C2：候選不到 3 張就全部列出（不洗牌）；沒有候選就沒有選擇畫面
#[test]
fn c2_rune_orb_few_candidates() {
    const TWO: &[&str] = &["CORE_CS2_029", "CORE_CS2_072", "CORE_CS2_023"];
    let mut g = start_pool(TWO);
    cast(&mut g, "CORE_BAR_541", None, Some(hero(1)));
    assert_eq!(choice_ids(&g), vec!["CORE_CS2_029", "CORE_CS2_023"]);
    const NONE: &[&str] = &["CORE_CS2_072", "CS2_182"];
    let mut g = start_pool(NONE);
    cast(&mut g, "CORE_BAR_541", None, Some(hero(1)));
    assert!(g.choice().is_none());
    assert_eq!(hero_health(&g, 1), 28);
}

// C3a、C3b：選牌前不進入死亡階段；選的牌先放進手牌，之後盜匪的死亡之聲送回的手下因手牌滿了被摧毀（實測 R5）
#[test]
fn c3a_discover_before_death_phase() {
    let mut g = start_pool(POOL);
    ready(&mut g, 0, FILL);
    ready(&mut g, 1, "CORE_ULD_280"); // 薩克特盜匪 4/4
    let sapper = g.player(1).board[0];
    g.entity_mut(sapper).health = 2;
    while g.player(0).hand.len() < 9 {
        give(&mut g, 0, FILL);
    }
    let i = give(&mut g, 0, "CORE_BAR_541");
    g.player_mut(0).mana = 10;
    act(&mut g, play(i, None, Some(minion(1, 0))));
    assert_eq!(g.player(1).board.len(), 1, "選牌前盜匪還沒移出戰場");
    let options = choice_ids(&g);
    act(&mut g, Action::Choose { index: 0 });
    assert!(g.player(1).board.is_empty());
    assert!(g.player(0).board.is_empty(), "送回的淡水鱷因手牌滿了被摧毀");
    assert_eq!(g.player(0).hand.len(), 10);
    assert_eq!(hand_ids(&g, 0).last(), Some(&options[0]));
}

// ---- D. 複製卡片 ----

// D1：任一玩家打出法術（含幸運幣、秘密），另一方得到一張相同的法術
#[test]
fn d1_lorewalker_cho() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_100");
    cast(&mut g, "CORE_CS2_029", None, Some(hero(1)));
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CORE_CS2_029"));
    cast(&mut g, "CORE_EX1_287", None, None);
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CORE_EX1_287"));
    end(&mut g);
    cast(&mut g, COIN, None, None);
    assert_eq!(hand_ids(&g, 0).last(), Some(&COIN));
    // 複製是原始費用的新牌
    let c = hand_pos(&g, 1, "CORE_CS2_029");
    assert_eq!(cost_at(&g, 1, c), 4);
}

// D2：被法術反制的法術也複製（提案）
#[test]
fn d2_lorewalker_copies_countered_spell() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_100");
    end(&mut g);
    cast(&mut g, "CORE_EX1_287", None, None);
    end(&mut g);
    cast(&mut g, "CORE_CS2_029", None, Some(hero(1)));
    assert_eq!(hero_health(&g, 1), 30);
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CORE_CS2_029"));
}

// D3：打出時、法術結算前就複製：法術打死阿洲也照樣給（實測 R6）
#[test]
fn d3_lorewalker_copies_before_resolution() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_100"); // 0/4
    end(&mut g);
    cast(&mut g, "CORE_CS2_032", None, None); // 對手施放烈焰風暴
    assert!(g.player(0).board.is_empty());
    assert_eq!(hand_ids(&g, 0).last(), Some(&"CORE_CS2_032"));
}

// D4、D5：對手抽牌時，阿拉巴斯特那方得到一張費用 1 的複製；疲勞、燒掉的牌不觸發
#[test]
fn d4_d5_alabaster() {
    let mut g = start(&[], &["CS2_182"]);
    // 後手前 4 張開局抽完，第 5 張是回合開始抽的
    let mut g2 = start(&[], &[FILL, FILL, FILL, FILL, "CS2_182"]);
    ready(&mut g2, 0, "CORE_SCH_717");
    let before = g2.player(0).hand.len();
    end(&mut g2);
    assert_eq!(g2.player(0).hand.len(), before + 1);
    assert_eq!(hand_ids(&g2, 0).last(), Some(&"CS2_182"));
    let c = g2.player(0).hand.len() - 1;
    assert_eq!(cost_at(&g2, 0, c), 1);
    // 燒掉的牌不觸發
    ready(&mut g, 0, "CORE_SCH_717");
    while g.player(1).hand.len() < 10 {
        give(&mut g, 1, FILL);
    }
    let before = g.player(0).hand.len();
    end(&mut g);
    assert_eq!(g.player(1).hand.len(), 10);
    assert_eq!(g.player(0).hand.len(), before);
    // 疲勞不觸發
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_SCH_717");
    g.player_mut(1).deck.clear();
    let before = g.player(0).hand.len();
    end(&mut g);
    assert_eq!(g.player(0).hand.len(), before);
}

// D6：對手抽到橡實：橡實施放、補抽；阿拉巴斯特先給補抽那張的複製，再給橡實的複製（不施放）（實測 R7）
#[test]
fn d6_alabaster_copies_acorn() {
    let mut g = start(&[], &[FILL, FILL, FILL, FILL, "SW_439t", "CS2_182"]);
    ready(&mut g, 0, "CORE_SCH_717");
    end(&mut g);
    assert_eq!(board_ids(&g, 1), vec!["SW_439t2"]);
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CS2_182"));
    let ids = hand_ids(&g, 0);
    assert_eq!(&ids[ids.len() - 2..], &["CS2_182", "SW_439t"]);
    assert_eq!(board_ids(&g, 0), vec!["CORE_SCH_717"]);
}

// D7、D8：奎格瓦把上一個自己回合打出的法術（含幸運幣、秘密、被反制的）放回手牌；更早的不算
#[test]
fn d7_d8_kragwa() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_CS2_023", None, None); // 第 1 回合：秘法智力（不算）
    end(&mut g);
    cast(&mut g, "CORE_EX1_287", None, None); // 對手的法術反制
    end(&mut g);
    cast(&mut g, "CORE_CS2_029", None, Some(hero(1))); // 被反制
    cast(&mut g, COIN, None, None);
    cast(&mut g, "CORE_LOOT_101", None, None); // 秘密
    end(&mut g);
    end(&mut g);
    g.player_mut(0).hand.truncate(5);
    let before = g.player(0).hand.len();
    cast(&mut g, "CORE_TRL_345", Some(0), None);
    let mut got: Vec<&str> = hand_ids(&g, 0)[before..].to_vec();
    got.sort();
    assert_eq!(got, vec!["CORE_CS2_029", "CORE_LOOT_101", COIN]);
}

// ---- E. 召喚加倍（卡德加） ----

// E1、E2、E3：戰吼召喚的手下多一隻，放在原手下右邊，也觸發「召喚後」；打出的手下本身不加倍（實測 R12）
#[test]
fn e1_e3_khadgar_tidehunter() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_509"); // 魚人招潮者 1/2
    ready(&mut g, 0, "CORE_DAL_575");
    ready(&mut g, 0, FILL);
    cast(&mut g, "CORE_EX1_506", Some(0), None);
    assert_eq!(
        board_ids(&g, 0),
        vec![
            "CORE_EX1_506",
            "CORE_EX1_506a",
            "CORE_EX1_506a",
            "CORE_EX1_509",
            "CORE_DAL_575",
            FILL
        ]
    );
    assert_eq!(board_entity(&g, 0, 3).attack, 4);
}

// E1、E2：英雄技能召喚也加倍；隨機召喚是複製同一隻、不重抽；戰場滿了不召喚
#[test]
fn e1_e2_khadgar_power_and_random_summon() {
    let mut g = start_with(PALADIN, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CORE_DAL_575");
    g.player_mut(0).mana = 10;
    act(&mut g, power(None));
    assert_eq!(
        board_ids(&g, 0),
        vec!["CORE_DAL_575", "CS2_101t", "CS2_101t"]
    );
    let mut g = start_scripted(
        HUNTER,
        WARRIOR,
        vec![RngEvent::Int {
            lo: 0,
            hi: 2,
            value: 1,
        }],
    );
    ready(&mut g, 0, "CORE_DAL_575");
    cast(&mut g, "CORE_NEW1_031", None, None);
    assert_eq!(
        board_ids(&g, 0),
        vec!["CORE_DAL_575", "NEW1_033", "NEW1_033"]
    );
    let mut g = start_with(PALADIN, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CORE_DAL_575");
    fill_board(&mut g, 0, 5);
    g.player_mut(0).mana = 10;
    act(&mut g, power(None));
    assert_eq!(g.player(0).board.len(), 7);
}

// E4：兩隻卡德加不疊加，只召喚兩次
#[test]
fn e4_two_khadgars_do_not_stack() {
    let mut g = start_with(PALADIN, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CORE_DAL_575");
    ready(&mut g, 0, "CORE_DAL_575");
    g.player_mut(0).mana = 10;
    act(&mut g, power(None));
    assert_eq!(g.player(0).board.len(), 4);
}

// ---- F. 洗入牌堆與抽中時施放（活潑的松鼠） ----

// F1：死亡之聲把 4 張橡實洗入牌堆
#[test]
fn f1_squirrel_shuffles_acorns() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_SW_439");
    let before = g.player(0).deck.len();
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 0)));
    let deck = &g.player(0).deck;
    assert_eq!(deck.len(), before + 4);
    let acorns = deck
        .iter()
        .filter(|&&c| g.entity(c).card.id == "SW_439t")
        .count();
    assert_eq!(acorns, 4);
}

// F2、F4：抽到橡實：召喚滿足的松鼠再補抽；戰場滿了不召喚、照樣補抽；卡德加讓松鼠多一隻
#[test]
fn f2_f4_acorn_cast_when_drawn() {
    let p1 = [FILL, FILL, FILL, FILL, "SW_439t", "CS2_182", FILL];
    let mut g = start(&p1, &[]);
    let before = g.player(0).hand.len();
    cast(&mut g, "CORE_CS2_023", None, None); // 秘法智力抽兩張
    assert_eq!(board_ids(&g, 0), vec!["SW_439t2"]);
    assert_eq!(&hand_ids(&g, 0)[before..], &["CS2_182", FILL]);
    let mut g = start(&p1, &[]);
    fill_board(&mut g, 0, 7);
    let before = g.player(0).hand.len();
    cast(&mut g, "CORE_CS2_023", None, None);
    assert_eq!(g.player(0).board.len(), 7);
    assert_eq!(&hand_ids(&g, 0)[before..], &["CS2_182", FILL]);
    let mut g = start(&p1, &[]);
    ready(&mut g, 0, "CORE_DAL_575");
    cast(&mut g, "CORE_CS2_023", None, None);
    assert_eq!(
        board_ids(&g, 0),
        vec!["CORE_DAL_575", "SW_439t2", "SW_439t2"]
    );
}

// F3：手牌滿了抽到橡實：燒掉，不施放、不補抽（實測 R9）
#[test]
fn f3_acorn_burned_when_hand_full() {
    let mut g = start(&[FILL, FILL, FILL, FILL, "SW_439t", "CS2_182"], &[]);
    while g.player(0).hand.len() < 10 {
        give(&mut g, 0, FILL);
    }
    let deck = g.player(0).deck.len();
    // 下一個自己的回合開始時抽到橡實
    end(&mut g);
    end(&mut g);
    assert!(g.player(0).board.is_empty());
    assert_eq!(g.player(0).deck.len(), deck - 1);
    assert_eq!(g.player(0).hand.len(), 10);
}

// F5：從手牌打出的橡實召喚一隻滿足的松鼠
#[test]
fn f5_acorn_played_from_hand() {
    let mut g = start(&[], &[]);
    cast(&mut g, "SW_439t", None, None);
    assert_eq!(board_ids(&g, 0), vec!["SW_439t2"]);
}

// ---- H. 差分測試發現的規格缺口 ----

// H1：視界術抽到橡實：−3 給抽到的橡實（已施放、落空），補抽到的冰風雪人維持原價 4（實測 R17）
#[test]
fn h1_far_sight_draws_acorn_discount_lost() {
    let mut g = start(&[FILL, FILL, FILL, FILL, "SW_439t", "CS2_182"], &[]);
    cast(&mut g, "CORE_CS2_053", None, None);
    assert_eq!(board_ids(&g, 0), vec!["SW_439t2"]);
    let i = hand_pos(&g, 0, "CS2_182");
    assert_eq!(cost_at(&g, 0, i), 4);
}

// H4：冰凍陷阱先打出：把攻擊你手下的攻擊者送回手牌後，綠洲盟友照樣觸發、召喚 3/6 水元素；攻擊取消（實測 R20）
#[test]
fn h4_freezing_trap_then_oasis_ally() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    ready(&mut g, 0, "CS2_182"); // 冰風雪人 4/5
    cast(&mut g, "CORE_EX1_611", None, None); // 冰凍陷阱
    cast(&mut g, "CORE_BAR_812", None, None); // 綠洲盟友
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_172"); // 血沼迅猛龍 3/2
    act(&mut g, attack(minion(1, i), minion(0, 0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CS2_172"));
    assert_eq!(board_ids(&g, 0), vec!["CS2_182", "CS2_033"]);
    assert_eq!(board_stats(&g, 0), vec![(4, 5), (3, 6)]);
    assert!(g.player(0).secrets.is_empty());
}
