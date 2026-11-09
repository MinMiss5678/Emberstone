//! 戰吼、死亡之聲、觸發的範例手下。

use emberstone_core::{Aura, CardDef, Effect, Sel, TargetReq, Trigger, When};

pub(crate) fn lookup(id: &str) -> Option<&'static CardDef> {
    Some(match id {
        "CORE_EX1_362" => &ARGENT_PROTECTOR,
        "CORE_CS2_188" => &ABUSIVE_SERGEANT,
        "CORE_EX1_506" => &MURLOC_TIDEHUNTER,
        "CORE_EX1_509" => &MURLOC_TIDECALLER,
        "CORE_EX1_058" => &SUNFURY_PROTECTOR,
        "EX1_556" => &HARVEST_GOLEM,
        "FP1_002" => &HAUNTED_CREEPER,
        "OG_241" => &POSSESSED_VILLAGER,
        "FP1_007" => &NERUBIAN_EGG,
        "OG_256" => &SPAWN_OF_NZOTH,
        "EX1_097" => &ABOMINATION,
        "CORE_EX1_096" => &LOOT_HOARDER,
        "CORE_SW_068" => &MOARG_FORGEFIEND,
        "CORE_EX1_604" => &FROTHING_BERSERKER,
        "CORE_CS2_222" => &STORMWIND_CHAMPION,
        "CORE_CS2_122" => &RAID_LEADER,
        "CORE_EX1_162" => &DIRE_WOLF_ALPHA,
        "CORE_OG_218" => &BLOODHOOF_BRAVE,
        "CORE_EX1_082" => &MAD_BOMBER,
        "NEW1_009" => &HEALING_TOTEM,
        _ => return None,
    })
}

const fn deathrattle_summon(card: &'static str, count: i32) -> CardDef {
    CardDef {
        deathrattle: Some(Effect::Summon { card, count }),
        ..CardDef::EMPTY
    }
}

/// 銀色黎明保衛者：戰吼，賦予一個友方手下聖盾術。
static ARGENT_PROTECTOR: CardDef = CardDef {
    target: TargetReq::FriendlyMinion,
    play: Some(Effect::DivineShield(Sel::Target)),
    ..CardDef::EMPTY
};

/// 嚴厲的士官：戰吼，本回合賦予一個手下 +2 攻擊力。
static ABUSIVE_SERGEANT: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::TurnAttack {
        to: Sel::Target,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 獵潮魚人：戰吼，召喚一個 1/1 魚人斥候（核心系列的衍生物是 CORE_EX1_506a）。
static MURLOC_TIDEHUNTER: CardDef = CardDef {
    play: Some(Effect::Summon {
        card: "CORE_EX1_506a",
        count: 1,
    }),
    ..CardDef::EMPTY
};

/// 魚人招潮者：每當你召喚魚人，獲得 +1 攻擊力。
static MURLOC_TIDECALLER: CardDef = CardDef {
    triggers: &[Trigger {
        when: When::FriendlySummon {
            race: Some("MURLOC"),
        },
        effect: Effect::Buff {
            to: Sel::Itself,
            attack: 1,
            health: 0,
        },
    }],
    ..CardDef::EMPTY
};

/// 日怒保衛者：戰吼，賦予兩側的手下嘲諷。
static SUNFURY_PROTECTOR: CardDef = CardDef {
    play: Some(Effect::Taunt(Sel::Adjacent)),
    ..CardDef::EMPTY
};

/// 麥田魔像：死亡之聲，召喚一個 2/1 受損的魔像。
static HARVEST_GOLEM: CardDef = deathrattle_summon("skele21", 1);

/// 鬼怪蠕行者：死亡之聲，召喚兩隻 1/1 鬼靈蜘蛛。
static HAUNTED_CREEPER: CardDef = deathrattle_summon("FP1_002t", 2);

/// 著魔的村民：死亡之聲，召喚一個 1/1 暗影野獸。
static POSSESSED_VILLAGER: CardDef = deathrattle_summon("OG_241a", 1);

/// 奈幽之卵：死亡之聲，召喚一個 4/4 奈幽蟲族。
static NERUBIAN_EGG: CardDef = deathrattle_summon("FP1_007t", 1);

/// 恩若司之子：死亡之聲，賦予你的手下 +1/+1。
static SPAWN_OF_NZOTH: CardDef = CardDef {
    deathrattle: Some(Effect::Buff {
        to: Sel::FriendlyMinions,
        attack: 1,
        health: 1,
    }),
    ..CardDef::EMPTY
};

/// 憎惡體：死亡之聲，對全部角色造成 2 點傷害。
static ABOMINATION: CardDef = CardDef {
    deathrattle: Some(Effect::Damage {
        to: Sel::AllCharacters,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 掠寶囤積者：死亡之聲，抽一張牌。
static LOOT_HOARDER: CardDef = CardDef {
    deathrattle: Some(Effect::Draw(1)),
    ..CardDef::EMPTY
};

/// 莫阿格鍛造魔：死亡之聲，獲得 8 點護甲值。
static MOARG_FORGEFIEND: CardDef = CardDef {
    deathrattle: Some(Effect::Armor(8)),
    ..CardDef::EMPTY
};

/// 白沫狂戰士：每當一個手下受到傷害，獲得 +1 攻擊力。
static FROTHING_BERSERKER: CardDef = CardDef {
    triggers: &[Trigger {
        when: When::MinionDamaged,
        effect: Effect::Buff {
            to: Sel::Itself,
            attack: 1,
            health: 0,
        },
    }],
    ..CardDef::EMPTY
};

/// 暴風城勇士：你的其他手下獲得 +1/+1。
static STORMWIND_CHAMPION: CardDef = CardDef {
    aura: Some(Aura::Others {
        attack: 1,
        health: 1,
        race: None,
    }),
    ..CardDef::EMPTY
};

/// 團隊隊長：你的其他手下獲得 +1 攻擊力。
static RAID_LEADER: CardDef = CardDef {
    aura: Some(Aura::Others {
        attack: 1,
        health: 0,
        race: None,
    }),
    ..CardDef::EMPTY
};

/// 恐狼首領：兩側的手下 +1 攻擊力。
static DIRE_WOLF_ALPHA: CardDef = CardDef {
    aura: Some(Aura::Adjacent { attack: 1 }),
    ..CardDef::EMPTY
};

/// 血蹄勇戰者：受傷時 +3 攻擊力。
static BLOODHOOF_BRAVE: CardDef = CardDef {
    aura: Some(Aura::WhileDamaged { attack: 3 }),
    ..CardDef::EMPTY
};

/// 瘋狂炸彈客：戰吼，造成 3 點傷害，隨機分給其他角色。
/// 抽法（同 RosettaStone）：3 次各從「自己的其他手下、自己的英雄、對手的手下、對手的英雄」挑一個，`Int(0, n - 1)`。
static MAD_BOMBER: CardDef = CardDef {
    play: Some(Effect::RandomSplitDamage(3)),
    ..CardDef::EMPTY
};

/// 治療圖騰：在你的回合結束時，替全部友方手下恢復 1 點生命值。
static HEALING_TOTEM: CardDef = CardDef {
    triggers: &[Trigger {
        when: When::OwnTurnEnd,
        effect: Effect::Heal {
            to: Sel::FriendlyMinions,
            amount: 1,
        },
    }],
    ..CardDef::EMPTY
};
