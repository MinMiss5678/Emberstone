//! 第 3a 階段的法術（批次 A 中沿用既有機制的卡）。

use emberstone_core::{CardDef, Choice, Effect, Sel, TargetReq, Usable};

pub(crate) fn lookup(id: &str) -> Option<&'static CardDef> {
    Some(match id {
        "CORE_AT_037" => &LIVING_ROOTS,
        "CORE_AT_055" => &FLASH_HEAL,
        "CORE_AT_064" => &BASH,
        "CORE_BAR_311" => &DEVOURING_PLAGUE,
        "CORE_BAR_801" => &WOUND_PREY,
        "CORE_BRM_013" => &QUICK_SHOT,
        "CORE_BT_035" => &CHAOS_STRIKE,
        "CORE_CS2_023" => &ARCANE_INTELLECT,
        "CORE_CS2_028" => &BLIZZARD,
        "CORE_CS2_032" => &FLAMESTRIKE,
        "CORE_CS2_053" => &FAR_SIGHT,
        "CORE_CS2_072" => &BACKSTAB,
        "CORE_CS2_074" => &DEADLY_POISON,
        "CORE_CS2_108" => &EXECUTE,
        "CORE_EX1_169" => &INNERVATE,
        "CORE_EX1_197" => &SHADOW_WORD_RUIN,
        "CORE_EX1_238" => &LIGHTNING_BOLT,
        "CORE_EX1_278" => &SHIV,
        "CORE_EX1_391" => &SLAM,
        "CORE_EX1_619" => &EQUALITY,
        "CORE_ICC_055" => &DRAIN_SOUL,
        "CORE_NEW1_031" => &ANIMAL_COMPANION,
        "CORE_OG_047" => &FERAL_RAGE,
        "CORE_SW_088" => &DEMONIC_ASSAULT,
        "CORE_SW_108" => &FIRST_FLAME,
        "SW_108t" => &SECOND_FLAME,
        "CORE_TRL_307" => &FLASH_OF_LIGHT,
        "CORE_TSC_650" => &FLIPPER_FRIENDS,
        _ => return None,
    })
}

const fn targeted(target: TargetReq, effect: Effect) -> CardDef {
    CardDef {
        target,
        play: Some(effect),
        ..CardDef::EMPTY
    }
}

const fn untargeted(effect: Effect) -> CardDef {
    CardDef {
        play: Some(effect),
        ..CardDef::EMPTY
    }
}

const fn damage(amount: i32) -> Effect {
    Effect::Damage {
        to: Sel::Target,
        amount,
    }
}

/// 糾纏之根：二選一（同 RosettaStone 的順序）：1 造成 2 點傷害；2 召喚兩個 1/1 樹苗。
static LIVING_ROOTS: CardDef = CardDef {
    choose: Some([
        Choice {
            target: TargetReq::Character,
            effect: damage(2),
        },
        Choice {
            target: TargetReq::None,
            effect: Effect::Summon {
                card: "AT_037t",
                count: 2,
            },
        },
    ]),
    ..CardDef::EMPTY
};

/// 快速治療：恢復 5 點生命值。
static FLASH_HEAL: CardDef = targeted(
    TargetReq::Character,
    Effect::Heal {
        to: Sel::Target,
        amount: 5,
    },
);

/// 重擊：造成 3 點傷害，獲得 3 點護甲值。
static BASH: CardDef = targeted(
    TargetReq::Character,
    Effect::Seq(&[damage(3), Effect::Armor(3)]),
);

/// 噬靈瘟疫：生命竊取（卡片關鍵字），造成 4 點傷害，隨機分給全部敵方手下；不受法術傷害影響。
/// 抽法（同 RosettaStone）：4 次各從還活著的敵方手下挑一個，`Int(0, n - 1)`。
static DEVOURING_PLAGUE: CardDef = untargeted(Effect::RandomSplitEnemyMinions(4));

/// 獵物致傷：造成 1 點傷害，召喚一個有衝刺的 1/1 土狼。
static WOUND_PREY: CardDef = targeted(
    TargetReq::Character,
    Effect::Seq(&[
        damage(1),
        Effect::Summon {
            card: "BAR_035t",
            count: 1,
        },
    ]),
);

/// 快速射擊：造成 3 點傷害；若你手中沒有牌，抽一張牌。
static QUICK_SHOT: CardDef = targeted(
    TargetReq::Character,
    Effect::Seq(&[damage(3), Effect::IfHandEmpty(&Effect::Draw(1))]),
);

/// 混沌之擊：本回合賦予你的英雄 +2 攻擊力，抽一張牌。
static CHAOS_STRIKE: CardDef = untargeted(Effect::Seq(&[Effect::HeroAttack(2), Effect::Draw(1)]));

/// 秘法智力：抽 2 張牌。
static ARCANE_INTELLECT: CardDef = untargeted(Effect::Draw(2));

/// 暴風雪：對全部敵方手下造成 2 點傷害並使其凍結。
static BLIZZARD: CardDef = untargeted(Effect::Seq(&[
    Effect::Damage {
        to: Sel::EnemyMinions,
        amount: 2,
    },
    Effect::Freeze(Sel::EnemyMinions),
]));

/// 烈焰風暴：對全部敵方手下造成 5 點傷害。
static FLAMESTRIKE: CardDef = untargeted(Effect::Damage {
    to: Sel::EnemyMinions,
    amount: 5,
});

/// 視界術：抽一張牌，該卡牌消耗減少 (3)。
static FAR_SIGHT: CardDef = untargeted(Effect::DrawReduceCost(3));

/// 背刺：對一個未受傷的手下造成 2 點傷害。
static BACKSTAB: CardDef = targeted(TargetReq::UndamagedMinion, damage(2));

/// 致命毒藥：賦予你的武器 +2 攻擊力（要有武器才能施放）。
static DEADLY_POISON: CardDef = CardDef {
    usable: Usable::WeaponEquipped,
    play: Some(Effect::WeaponAttack(2)),
    ..CardDef::EMPTY
};

/// 斬殺：摧毀一個受傷的敵方手下。
static EXECUTE: CardDef = targeted(TargetReq::DamagedEnemyMinion, Effect::Destroy(Sel::Target));

/// 啟動：在本回合獲得額外 1 顆法力水晶。
static INNERVATE: CardDef = untargeted(Effect::TempMana(1));

/// 暗言術：滅：摧毀所有攻擊力 5 點以上的手下。
static SHADOW_WORD_RUIN: CardDef = untargeted(Effect::Destroy(Sel::MinionsWithAttackAtLeast(5)));

/// 閃電箭：造成 3 點傷害（超載在卡表）。
static LIGHTNING_BOLT: CardDef = targeted(TargetReq::Character, damage(3));

/// 毒襲：造成 1 點傷害，抽一張牌。
static SHIV: CardDef = targeted(
    TargetReq::Character,
    Effect::Seq(&[damage(1), Effect::Draw(1)]),
);

/// 猛擊：對一個手下造成 2 點傷害；若其存活，抽一張牌。
static SLAM: CardDef = targeted(
    TargetReq::Minion,
    Effect::Seq(&[damage(2), Effect::IfTargetSurvived(&Effect::Draw(1))]),
);

/// 一視同仁：將全部手下的生命值改為 1 點。
static EQUALITY: CardDef = untargeted(Effect::SetHealth {
    to: Sel::AllMinions,
    value: 1,
});

/// 吸取靈魂：生命竊取（卡片關鍵字），對一個手下造成 3 點傷害。
static DRAIN_SOUL: CardDef = targeted(TargetReq::Minion, damage(3));

/// 動物夥伴：召喚一隻隨機的野獸夥伴（要有空位）。
/// 抽法（同 RosettaStone）：`Int(0, 2)` 依序對應米莎、雷歐克、霍弗。
static ANIMAL_COMPANION: CardDef = CardDef {
    usable: Usable::BoardSlot,
    play: Some(Effect::SummonRandom(&["NEW1_032", "NEW1_033", "NEW1_034"])),
    ..CardDef::EMPTY
};

/// 狂野怒氣：二選一：1 本回合賦予你的英雄 +4 攻擊力；2 獲得 8 點護甲值。
static FERAL_RAGE: CardDef = CardDef {
    choose: Some([
        Choice {
            target: TargetReq::None,
            effect: Effect::HeroAttack(4),
        },
        Choice {
            target: TargetReq::None,
            effect: Effect::Armor(8),
        },
    ]),
    ..CardDef::EMPTY
};

/// 惡魔襲擊：造成 3 點傷害，召喚兩個有嘲諷的 1/3 虛無行者。
static DEMONIC_ASSAULT: CardDef = targeted(
    TargetReq::Character,
    Effect::Seq(&[
        damage(3),
        Effect::Summon {
            card: "CS2_065",
            count: 2,
        },
    ]),
);

/// 第一道火焰：對一個手下造成 2 點傷害，放一張第二道火焰到你的手中。
static FIRST_FLAME: CardDef = targeted(
    TargetReq::Minion,
    Effect::Seq(&[damage(2), Effect::AddToHand("SW_108t")]),
);

/// 第二道火焰：對一個手下造成 2 點傷害。
static SECOND_FLAME: CardDef = targeted(TargetReq::Minion, damage(2));

/// 聖光閃現：恢復 4 點生命值，抽一張牌。
static FLASH_OF_LIGHT: CardDef = targeted(
    TargetReq::Character,
    Effect::Seq(&[
        Effect::Heal {
            to: Sel::Target,
            amount: 4,
        },
        Effect::Draw(1),
    ]),
);

/// 揮尾好朋友：二選一：1 召喚一個有嘲諷的 6/6 虎鯨；2 召喚六個有衝刺的 1/1 海獺。
static FLIPPER_FRIENDS: CardDef = CardDef {
    choose: Some([
        Choice {
            target: TargetReq::None,
            effect: Effect::Summon {
                card: "TSC_650t",
                count: 1,
            },
        },
        Choice {
            target: TargetReq::None,
            effect: Effect::Summon {
                card: "TSC_650t4",
                count: 6,
            },
        },
    ]),
    ..CardDef::EMPTY
};
