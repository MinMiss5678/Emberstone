//! 第 2 階段 H：沉默（case H1–H3）；I：秘密（case I1–I4）。

mod common;

use common::*;
use emberstone_core::{Action, Game, Outcome};

/// 打出鐵喙貓頭鷹沉默目標（放在最右邊）。
fn owl(g: &mut Game, target: emberstone_core::Target) {
    let pos = g.player(g.current_player()).board.len();
    cast(g, "CS2_203", Some(pos), Some(target));
}

// H1：沉默移除附魔（含本回合加成）、關鍵字、死亡之聲、觸發、它提供的被動加成；解除凍結
#[test]
fn h1_silence_removes_everything() {
    let mut g = start(&[], &[]);
    ready(&mut g, 1, "CORE_EX1_096"); // 掠寶囤積者（死亡之聲）
    ready(&mut g, 1, "CORE_CS2_122"); // 團隊隊長（被動加成）
    ready(&mut g, 1, "CS2_179"); // 森金禦盾大師（嘲諷）
    cast(&mut g, "CORE_CS2_009", None, Some(minion(1, 2))); // 野性印記
    cast(&mut g, "CORE_CS2_188", Some(0), Some(minion(1, 2))); // 嚴厲的士官 +2（本回合）
    let e = g.player(1).board[2];
    g.entity_mut(e).frozen = true;
    owl(&mut g, minion(1, 2));
    let s = board_entity(&g, 1, 2);
    assert_eq!((s.attack, s.health), (3 + 1, 5)); // 原始 3/5，團隊隊長 +1 仍在
    assert!(!s.taunt && !s.frozen);
    // 沉默團隊隊長：其他手下失去加成
    owl(&mut g, minion(1, 1));
    assert_eq!(board_stats(&g, 1), vec![(2, 1), (2, 3), (3, 5)]);
    // 沉默掠寶囤積者：死亡時不抽牌
    owl(&mut g, minion(1, 0));
    let hand = g.player(1).hand.len();
    cast(&mut g, "CORE_CS2_029", None, Some(minion(1, 0)));
    assert_eq!(g.player(1).hand.len(), hand);
}

// H1：沉默觸發類手下後不再觸發
#[test]
fn h1_silenced_trigger() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CORE_EX1_509"); // 魚人招潮者
    owl(&mut g, minion(0, 0));
    cast(&mut g, "CORE_EX1_506", Some(0), None);
    assert_eq!(board_entity(&g, 0, 2).attack, 1);
}

// H2：沉默後回到原始數值；目前生命只在超過新上限時才降
#[test]
fn h2_silence_health() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_182"); // 冰風雪人 4/5
    ready(&mut g, 0, "CS2_182");
    for i in 0..2 {
        cast(&mut g, "CORE_CS2_004", None, Some(minion(0, i))); // +2 生命 → 4/7
    }
    cast(&mut g, "CORE_CS1_130", None, Some(minion(0, 0))); // 3 點 → 4/4
    cast(&mut g, "CORE_CS2_029", None, Some(minion(0, 1))); // 6 點 → 4/1
    owl(&mut g, minion(0, 0));
    owl(&mut g, minion(0, 1));
    assert_eq!(board_stats(&g, 0)[..2], [(4, 4), (4, 1)]);
    assert_eq!(board_entity(&g, 0, 0).max_health, 5);
}

// H3：別人給的被動加成不受影響
#[test]
fn h3_others_aura_still_applies() {
    let mut g = start(&[], &[]);
    ready(&mut g, 0, "CS2_120");
    ready(&mut g, 0, "CORE_CS2_222");
    owl(&mut g, minion(0, 0));
    assert_eq!(board_stats(&g, 0)[0], (3, 4));
}

fn secrets(g: &Game, player: usize) -> Vec<&'static str> {
    g.player(player)
        .secrets
        .iter()
        .map(|&s| g.entity(s).card.id)
        .collect()
}

// I1：打出秘密放進秘密區；同一張不能同時有兩張；最多 5 個
#[test]
fn i1_secret_zone_rules() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_289", None, None); // 寒冰護體
    assert_eq!(secrets(&g, 0), vec!["CORE_EX1_289"]);
    g.player_mut(0).mana = 10;
    let i = give(&mut g, 0, "CORE_EX1_289");
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Play { hand, .. } if *hand == i))
    );
    for id in [
        "CORE_EX1_287",
        "CORE_LOOT_101",
        "CORE_EX1_610",
        "CORE_EX1_611",
    ] {
        cast(&mut g, id, None, None);
    }
    assert_eq!(g.player(0).secrets.len(), 5);
    let j = give(&mut g, 0, "CORE_BAR_812"); // 綠洲盟友
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Play { hand, .. } if *hand == j))
    );
}

// I2：只在對手回合觸發；觸發後揭露並移除（爆炸符文：對手打出手下後造成 6 點、溢傷打英雄）
#[test]
fn i2_explosive_runes_only_on_opponent_turn() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_LOOT_101", None, None);
    cast(&mut g, "CS2_120", Some(0), None); // 自己出手下：不觸發
    assert_eq!(board_ids(&g, 0), vec!["CS2_120"]);
    end(&mut g);
    cast(&mut g, "CS2_120", Some(0), None); // 對手出 2/3：6 點，溢傷 3 打英雄
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 1), 27);
    assert!(g.player(0).secrets.is_empty());
}

// I4：寒冰護體：英雄被攻擊時獲得 8 點護甲
#[test]
fn i4_ice_barrier() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_289", None, None);
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_182");
    act(&mut g, attack(minion(1, i), hero(0)));
    assert_eq!((hero_entity(&g, 0).armor, hero_health(&g, 0)), (4, 30));
    assert!(g.player(0).secrets.is_empty());
}

// I4：法術反制：對手的法術無效，法力照扣
#[test]
fn i4_counterspell() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_EX1_287", None, None);
    end(&mut g);
    cast(&mut g, "CORE_CS2_029", None, Some(hero(0)));
    assert_eq!(hero_health(&g, 0), 30);
    assert_eq!(g.player(1).mana, 6);
    assert!(g.player(0).secrets.is_empty());
}

// I4：冰凍陷阱：攻擊的手下回到手牌，費用 +2
#[test]
fn i4_freezing_trap() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_611", None, None);
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_182");
    act(&mut g, attack(minion(1, i), hero(0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 0), 30);
    let last = *g.player(1).hand.last().unwrap();
    assert_eq!(g.entity(last).card.id, "CS2_182");
    g.player_mut(1).mana = 5;
    let pos = g.player(1).hand.len() - 1;
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Play { hand, .. } if *hand == pos))
    );
    g.player_mut(1).mana = 6;
    act(&mut g, play(pos, Some(0), None));
    assert_eq!(g.player(1).mana, 0);
}

// I4：冰凍陷阱送回手牌的衍生物（奈幽蟲族，費用 4）也是費用 +2，下一回合起 6 點法力就能打出
#[test]
fn i4_freezing_trap_token() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_611", None, None);
    end(&mut g);
    let i = ready(&mut g, 1, "FP1_007t");
    act(&mut g, attack(minion(1, i), hero(0)));
    let pos = g.player(1).hand.len() - 1;
    assert_eq!(g.entity(g.player(1).hand[pos]).card.id, "FP1_007t");
    end(&mut g);
    end(&mut g);
    let pos = hand_ids(&g, 1)
        .iter()
        .position(|&id| id == "FP1_007t")
        .unwrap();
    g.player_mut(1).mana = 6;
    assert!(
        g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Play { hand, .. } if *hand == pos))
    );
}

// I4：被冰凍陷阱送回、再打出、又被送回的手下，費用還是原費用 +2（上一次的 +2 打出後就失效）
#[test]
fn i4_freezing_trap_twice() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_611", None, None);
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_182"); // 冰風雪人（4 費）
    act(&mut g, attack(minion(1, i), hero(0)));
    let pos = g.player(1).hand.len() - 1;
    g.player_mut(1).mana = 6;
    act(&mut g, play(pos, Some(0), None));
    end(&mut g);
    cast(&mut g, "CORE_EX1_611", None, None);
    end(&mut g);
    act(&mut g, attack(minion(1, 0), hero(0)));
    let last = *g.player(1).hand.last().unwrap();
    assert_eq!(g.entity(last).card.id, "CS2_182");
    assert_eq!(g.entity(last).cost(), 6);
}

// I3：多個秘密同時符合條件時依打出順序：爆炸陷阱先打死攻擊者，冰凍陷阱就不觸發、留在場上
#[test]
fn i3_secrets_in_play_order() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_610", None, None); // 爆炸陷阱：英雄被攻擊時對全部敵人造成 2 點
    cast(&mut g, "CORE_EX1_611", None, None); // 冰凍陷阱
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_172"); // 血沼迅猛龍 3/2
    act(&mut g, attack(minion(1, i), hero(0)));
    assert!(g.player(1).board.is_empty());
    assert_eq!(hero_health(&g, 0), 30);
    assert_eq!(hero_health(&g, 1), 28);
    assert_eq!(secrets(&g, 0), vec!["CORE_EX1_611"]);
}

// I3a：冰凍陷阱先打出：攻擊者送回手牌後，爆炸陷阱照樣觸發；攻擊取消
#[test]
fn i3a_freezing_trap_then_explosive_trap() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_611", None, None); // 冰凍陷阱
    cast(&mut g, "CORE_EX1_610", None, None); // 爆炸陷阱
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_172"); // 血沼迅猛龍 3/2
    ready(&mut g, 1, "CS2_120"); // 淡水鱷 2/3
    act(&mut g, attack(minion(1, i), hero(0)));
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CS2_172"));
    assert_eq!(board_stats(&g, 1), vec![(2, 1)]);
    assert_eq!(hero_health(&g, 1), 28);
    assert_eq!(hero_health(&g, 0), 30);
    assert!(secrets(&g, 0).is_empty());
}

// I3d：冰凍陷阱送回暴風城勇士後，爆炸陷阱結算時勇士的加成還在；秘密都結算完才移除
#[test]
fn i3d_aura_updates_after_all_secrets() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_611", None, None); // 冰凍陷阱
    cast(&mut g, "CORE_EX1_610", None, None); // 爆炸陷阱
    end(&mut g);
    ready(&mut g, 1, "CS2_172"); // 血沼迅猛龍 3/2，勇士旁 4/3
    let c = ready(&mut g, 1, "CORE_CS2_222"); // 暴風城勇士 6/6
    act(&mut g, attack(minion(1, c), hero(0)));
    assert_eq!(hand_ids(&g, 1).last(), Some(&"CORE_CS2_222"));
    assert_eq!(board_stats(&g, 1), vec![(3, 1)]);
    assert_eq!(hero_health(&g, 1), 28);
    assert!(secrets(&g, 0).is_empty());
}

// I3b：秘密把攻擊方的英雄打死：秘密結算完進入死亡階段，遊戲結束，攻擊不發生
#[test]
fn i3b_secret_kills_attacking_hero() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_610", None, None); // 爆炸陷阱
    end(&mut g);
    let i = ready(&mut g, 1, "CS2_182"); // 冰風雪人 4/5
    set_hero_health(&mut g, 1, 2);
    act(&mut g, attack(minion(1, i), hero(0)));
    assert_eq!(g.outcome(), Some(Outcome::Won(0)));
    assert_eq!(hero_health(&g, 0), 30);
}

// I4a：爆炸符文打到聖盾手下：聖盾破掉並吸收等於手下生命的傷害，其餘打英雄
#[test]
fn i4a_explosive_runes_divine_shield() {
    let mut g = start(&[], &[]);
    cast(&mut g, "CORE_LOOT_101", None, None); // 爆炸符文
    end(&mut g);
    cast(&mut g, "EX1_020", Some(0), None); // 血色十字軍 3/1 聖盾術
    assert_eq!(hero_health(&g, 1), 25);
    assert_eq!(board_stats(&g, 1), vec![(3, 1)]);
    assert!(!board_entity(&g, 1, 0).divine_shield);
}

// I3c：秘密結算完才進行戰鬥，攻擊力用當下的值（白沫狂戰士被爆炸陷阱打傷 +1）
#[test]
fn i3c_combat_uses_attack_after_secrets() {
    let mut g = start_with(HUNTER, &[], WARRIOR, &[]);
    cast(&mut g, "CORE_EX1_610", None, None); // 爆炸陷阱
    end(&mut g);
    let i = ready(&mut g, 1, "CORE_EX1_604"); // 白沫狂戰士 2/4
    act(&mut g, attack(minion(1, i), hero(0)));
    assert_eq!(board_stats(&g, 1), vec![(3, 2)]);
    assert_eq!(hero_health(&g, 0), 27);
}
