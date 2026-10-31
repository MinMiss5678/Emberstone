//! 第 1 階段 D：攻擊與關鍵字（case D1–D9）。

mod common;

use common::*;
use emberstone_core::ActError;

fn sorted(mut v: Vec<emberstone_core::Target>) -> Vec<emberstone_core::Target> {
    v.sort_by_key(|t| format!("{t:?}"));
    v
}

// D1：雙方同時造成傷害；生命歸零的在攻擊結束後移出戰場
#[test]
fn d1_minions_trade() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182"); // 4/5
    let raptor = ready(&mut g, 0, "CS2_172"); // 3/2
    ready(&mut g, 1, "CS2_120"); // 2/3
    ready(&mut g, 1, "CS2_120");
    act(&mut g, attack(minion(0, yeti), minion(1, 0)));
    assert_eq!(board_entity(&g, 0, yeti).health, 3);
    assert_eq!(board_ids(&g, 1), vec!["CS2_120"]);
    act(&mut g, attack(minion(0, raptor), minion(1, 0)));
    assert_eq!(board_ids(&g, 0), vec!["CS2_182"]);
    assert!(g.player(1).board.is_empty());
}

// D2：對手有嘲諷手下時只能攻擊嘲諷手下
#[test]
fn d2_taunt_must_be_attacked() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182");
    ready(&mut g, 1, "CS2_120");
    ready(&mut g, 1, "CS2_179"); // 嘲諷
    assert_eq!(attack_targets(&g, minion(0, yeti)), vec![minion(1, 1)]);
    assert_eq!(
        g.act(&attack(minion(0, yeti), hero(1))),
        Err(ActError::Illegal)
    );
    assert_eq!(
        g.act(&attack(minion(0, yeti), minion(1, 0))),
        Err(ActError::Illegal)
    );
}

// D3：潛行不能被選為目標；攻擊後失去潛行；潛行中的嘲諷不強制
#[test]
fn d3_stealth() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182");
    let panther = ready(&mut g, 0, "EX1_017"); // 潛行 4/2
    ready(&mut g, 1, "EX1_017");
    ready(&mut g, 1, "CS2_120");
    let taunt = ready(&mut g, 1, "CS2_179");
    let e = g.player(1).board[taunt];
    g.entity_mut(e).stealth = true;
    assert_eq!(
        sorted(attack_targets(&g, minion(0, yeti))),
        sorted(vec![hero(1), minion(1, 1)])
    );
    assert!(board_entity(&g, 0, panther).stealth);
    act(&mut g, attack(minion(0, panther), hero(1)));
    assert!(!board_entity(&g, 0, panther).stealth);
}

// D4：聖盾術吸收一次傷害，之後正常受傷
#[test]
fn d4_divine_shield() {
    let mut g = start(&[], &[]);
    let yeti = ready(&mut g, 0, "CS2_182");
    ready(&mut g, 1, "EX1_008"); // 1/1 聖盾術
    assert!(board_entity(&g, 1, 0).divine_shield);
    act(&mut g, attack(minion(0, yeti), minion(1, 0)));
    let squire = board_entity(&g, 1, 0);
    assert_eq!((squire.health, squire.divine_shield), (1, false));
    assert_eq!(board_entity(&g, 0, yeti).health, 4);
    end(&mut g);
    end(&mut g);
    act(&mut g, attack(minion(0, yeti), minion(1, 0)));
    assert!(g.player(1).board.is_empty());
}

// D5：致命劇毒消滅受傷的手下；被聖盾術擋下無效；對英雄沒有特殊效果
#[test]
fn d5_poisonous() {
    let mut g = start(&[], &[]);
    let cobra = ready(&mut g, 0, "EX1_170"); // 2/3 致命劇毒
    ready(&mut g, 1, "EX1_008");
    ready(&mut g, 1, "CS2_186"); // 7/7
    act(&mut g, attack(minion(0, cobra), minion(1, 0)));
    assert_eq!(board_ids(&g, 1), vec!["EX1_008", "CS2_186"]);
    assert_eq!(board_entity(&g, 1, 0).health, 1);
    end(&mut g);
    end(&mut g);
    act(&mut g, attack(minion(0, cobra), hero(1)));
    assert_eq!(hero_health(&g, 1), 28);
    end(&mut g);
    end(&mut g);
    act(&mut g, attack(minion(0, cobra), minion(1, 1)));
    assert_eq!(board_ids(&g, 1), vec!["EX1_008"]);
    assert!(g.player(0).board.is_empty());
}

// D6：風怒每回合可以攻擊 2 次
#[test]
fn d6_windfury_attacks_twice() {
    let mut g = start(&[], &[]);
    let harpy = ready(&mut g, 0, "EX1_033"); // 4/5 風怒
    act(&mut g, attack(minion(0, harpy), hero(1)));
    act(&mut g, attack(minion(0, harpy), hero(1)));
    assert_eq!(hero_health(&g, 1), 22);
    assert!(attack_targets(&g, minion(0, harpy)).is_empty());
}

// D7：生命竊取回復自己的英雄等量生命，最多到 30
#[test]
fn d7_lifesteal() {
    let mut g = start(&[], &[]);
    let leech = ready(&mut g, 0, "GIL_558"); // 2/1 生命竊取
    let h = g.player(0).hero;
    g.entity_mut(h).health = 25;
    act(&mut g, attack(minion(0, leech), hero(1)));
    assert_eq!((hero_health(&g, 0), hero_health(&g, 1)), (27, 28));
    end(&mut g);
    end(&mut g);
    g.entity_mut(h).health = 29;
    act(&mut g, attack(minion(0, leech), hero(1)));
    assert_eq!(hero_health(&g, 0), 30);
}

// D8：攻擊力 0 不能攻擊
#[test]
fn d8_zero_attack_cannot_attack() {
    let mut g = start(&[], &[]);
    let totem = ready(&mut g, 0, "CS2_051"); // 0/2 嘲諷
    assert!(attack_targets(&g, minion(0, totem)).is_empty());
    assert_eq!(
        g.act(&attack(minion(0, totem), hero(1))),
        Err(ActError::Illegal)
    );
}

// D9：在對方回合被凍結 → 下一個自己的回合不能攻擊，那個回合結束時解凍
#[test]
fn d9_freeze_thaws_after_skipped_turn() {
    let mut g = start(&[], &[]);
    end(&mut g);
    let yeti = ready(&mut g, 0, "CS2_182");
    let elemental = ready(&mut g, 1, "CS2_033"); // 3/6 凍結
    act(&mut g, attack(minion(1, elemental), minion(0, yeti)));
    assert!(board_entity(&g, 0, yeti).frozen);
    end(&mut g);
    assert!(board_entity(&g, 0, yeti).frozen);
    assert!(attack_targets(&g, minion(0, yeti)).is_empty());
    end(&mut g);
    assert!(!board_entity(&g, 0, yeti).frozen);
    end(&mut g);
    assert!(!attack_targets(&g, minion(0, yeti)).is_empty());
}
