//! 幸運幣與基本法術、戰吼。

use emberstone_core::{CardDef, Choice, Effect, Sel, TargetReq};

pub(crate) fn lookup(id: &str) -> Option<&'static CardDef> {
    Some(match id {
        "GAME_005" => &THE_COIN,
        "SCH_307t" => &SOUL_FRAGMENT,
        "CORE_CS2_029" => &FIREBALL,
        "CORE_CS1_130" => &HOLY_SMITE,
        "CORE_CS2_076" => &ASSASSINATE,
        "CORE_DS1_185" => &ARCANE_SHOT,
        "CORE_CS2_189" => &ELVEN_ARCHER,
        "CORE_CS2_062" => &HELLFIRE,
        "CORE_CS2_004" => &POWER_WORD_SHIELD,
        "CORE_CS2_009" => &MARK_OF_THE_WILD,
        "CS2_022" => &POLYMORPH,
        "CORE_EX1_160" => &POWER_OF_THE_WILD,
        "CORE_EX1_154" => &WRATH,
        "CORE_DS1_184" => &TRACKING,
        _ => return None,
    })
}

const fn damage_target(target: TargetReq, amount: i32) -> CardDef {
    CardDef {
        target,
        play: Some(Effect::Damage {
            to: Sel::Target,
            amount,
        }),
        ..CardDef::EMPTY
    }
}

/// 幸運幣：在本回合獲得額外 1 顆法力水晶（可以超過 10）。
static THE_COIN: CardDef = CardDef {
    play: Some(Effect::TempMana(1)),
    ..CardDef::EMPTY
};

/// 靈魂碎片：抽中時施放，使你的英雄恢復 2 點生命值。
static SOUL_FRAGMENT: CardDef = CardDef {
    when_drawn: Some(Effect::Heal {
        to: Sel::OwnHero,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 火球術：造成 6 點傷害。
static FIREBALL: CardDef = damage_target(TargetReq::Character, 6);

/// 聖光懲擊：對一個手下造成 3 點傷害。
static HOLY_SMITE: CardDef = damage_target(TargetReq::Minion, 3);

/// 暗殺：消滅一個敵方手下。
static ASSASSINATE: CardDef = CardDef {
    target: TargetReq::EnemyMinion,
    play: Some(Effect::Destroy(Sel::Target)),
    ..CardDef::EMPTY
};

/// 秘法射擊：造成 2 點傷害。
static ARCANE_SHOT: CardDef = damage_target(TargetReq::Character, 2);

/// 精靈弓箭手：戰吼，造成 1 點傷害。
static ELVEN_ARCHER: CardDef = damage_target(TargetReq::Character, 1);

/// 地獄烈焰：對全部角色造成 3 點傷害。
static HELLFIRE: CardDef = CardDef {
    play: Some(Effect::Damage {
        to: Sel::AllCharacters,
        amount: 3,
    }),
    ..CardDef::EMPTY
};

/// 真言術：盾：賦予一個手下 +2 生命值，抽一張牌。
static POWER_WORD_SHIELD: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::Seq(&[
        Effect::Buff {
            to: Sel::Target,
            attack: 0,
            health: 2,
        },
        Effect::Draw(1),
    ])),
    ..CardDef::EMPTY
};

/// 野性印記：賦予一個手下嘲諷及 +2/+3。
static MARK_OF_THE_WILD: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::Seq(&[
        Effect::Taunt(Sel::Target),
        Effect::Buff {
            to: Sel::Target,
            attack: 2,
            health: 3,
        },
    ])),
    ..CardDef::EMPTY
};

/// 變形術：將一個手下變形為 1/1 的羊。
static POLYMORPH: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::Transform {
        to: Sel::Target,
        card: "CS2_tk1",
    }),
    ..CardDef::EMPTY
};

/// 野性之力：二選一（同 RosettaStone 的順序）：1 召喚一隻 3/2 獵豹；2 賦予你的手下 +1/+1。
static POWER_OF_THE_WILD: CardDef = CardDef {
    choose: Some([
        Choice {
            target: TargetReq::None,
            effect: Effect::Summon {
                card: "EX1_160t",
                count: 1,
            },
        },
        Choice {
            target: TargetReq::None,
            effect: Effect::Buff {
                to: Sel::FriendlyMinions,
                attack: 1,
                health: 1,
            },
        },
    ]),
    ..CardDef::EMPTY
};

/// 憤怒：二選一：1 對一個手下造成 3 點傷害；2 造成 1 點傷害並抽一張牌。
static WRATH: CardDef = CardDef {
    choose: Some([
        Choice {
            target: TargetReq::Minion,
            effect: Effect::Damage {
                to: Sel::Target,
                amount: 3,
            },
        },
        Choice {
            target: TargetReq::Minion,
            effect: Effect::Seq(&[
                Effect::Damage {
                    to: Sel::Target,
                    amount: 1,
                },
                Effect::Draw(1),
            ]),
        },
    ]),
    ..CardDef::EMPTY
};

/// 追蹤：從你的牌堆發現一張牌。
static TRACKING: CardDef = CardDef {
    play: Some(Effect::DiscoverFromDeck),
    ..CardDef::EMPTY
};
