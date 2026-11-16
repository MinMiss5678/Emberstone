//! 戰吼、死亡之聲、觸發的範例手下。

use emberstone_core::{Aura, CardDef, Choice, Ctx, Effect, Game, Sel, TargetReq, Trigger, When};

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
        "CORE_BT_120" => &WARMAUL_CHALLENGER,
        "CORE_CS2_042" => &FIRE_ELEMENTAL,
        "CORE_EX1_011" => &VOODOO_DOCTOR,
        "CORE_EX1_012" => &BLOODMAGE_THALNOS,
        "CORE_EX1_043" => &TWILIGHT_DRAKE,
        "CORE_EX1_059" => &CRAZED_ALCHEMIST,
        "CORE_EX1_103" => &COLDLIGHT_SEER,
        "CORE_EX1_414" => &GROMMASH,
        "CORE_EX1_507" => &MURLOC_WARLEADER,
        "CORE_GIL_622" => &LIFEDRINKER,
        "CORE_NEW1_018" => &BLOODSAIL_RAIDER,
        "CORE_NEW1_027" => &SOUTHSEA_CAPTAIN,
        "CORE_ONY_018" => &BOOMKIN,
        "CORE_ULD_165" => &RIFTCLEAVER,
        "CORE_ULD_191" => &BEAMING_SIDEKICK,
        "CORE_ULD_271" => &INJURED_TOLVIR,
        "CORE_ULD_280" => &SAHKET_SAPPER,
        "CORE_UNG_928" => &TAR_CREEPER,
        "CORE_WC_042" => &WAILING_VAPOR,
        "CS3_020" => &ILLIDARI_INQUISITOR,
        "CS3_022" => &FOGSAIL_FREEBOOTER,
        "NEW1_033" => &LEOKK,
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

/// 戰槌挑戰者：戰吼，選擇一個敵方手下，和它互相攻擊到一方死亡或離場，最多 30 次（case D6）。
static WARMAUL_CHALLENGER: CardDef = CardDef {
    target: TargetReq::EnemyMinion,
    play: Some(Effect::Custom(warmaul_duel)),
    ..CardDef::EMPTY
};

fn warmaul_duel(g: &mut Game, ctx: &Ctx) {
    let Some(t) = ctx.target else {
        return;
    };
    for _ in 0..30 {
        if !g.in_play(ctx.source) || !g.in_play(t) || g.outcome().is_some() {
            break;
        }
        g.forced_attack(ctx.source, t);
    }
}

/// 火元素：戰吼，造成 4 點傷害。
static FIRE_ELEMENTAL: CardDef = CardDef {
    target: TargetReq::Character,
    play: Some(Effect::Damage {
        to: Sel::Target,
        amount: 4,
    }),
    ..CardDef::EMPTY
};

/// 巫毒巫醫：戰吼，恢復 2 點生命值。
static VOODOO_DOCTOR: CardDef = CardDef {
    target: TargetReq::Character,
    play: Some(Effect::Heal {
        to: Sel::Target,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 血法師薩爾諾斯：法術傷害 +1（卡表）；死亡之聲，抽一張牌。
static BLOODMAGE_THALNOS: CardDef = CardDef {
    deathrattle: Some(Effect::Draw(1)),
    ..CardDef::EMPTY
};

/// 暮光飛龍：戰吼，你的每張手牌使其獲得 +1 生命值。
static TWILIGHT_DRAKE: CardDef = CardDef {
    play: Some(Effect::Custom(twilight_drake)),
    ..CardDef::EMPTY
};

fn twilight_drake(g: &mut Game, ctx: &Ctx) {
    let n = g.player(ctx.owner).hand.len() as i32;
    let e = g.entity_mut(ctx.source);
    e.max_health += n;
    e.health += n;
}

/// 瘋狂的鍊金師：戰吼，對調一個手下的攻擊力和生命值。
static CRAZED_ALCHEMIST: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::SwapAttackHealth(Sel::Target)),
    ..CardDef::EMPTY
};

/// 冷光先知：戰吼，賦予你的其他魚人 +2 生命值。
static COLDLIGHT_SEER: CardDef = CardDef {
    play: Some(Effect::Buff {
        to: Sel::OtherFriendlyMinions {
            race: Some("MURLOC"),
        },
        attack: 0,
        health: 2,
    }),
    ..CardDef::EMPTY
};

/// 葛羅瑪許‧地獄吼：衝鋒（卡表）；受傷時 +6 攻擊力。
static GROMMASH: CardDef = CardDef {
    aura: Some(Aura::WhileDamaged { attack: 6 }),
    ..CardDef::EMPTY
};

/// 魚人隊長：你的其他魚人獲得 +2 攻擊力。
static MURLOC_WARLEADER: CardDef = CardDef {
    aura: Some(Aura::Others {
        attack: 2,
        health: 0,
        race: Some("MURLOC"),
    }),
    ..CardDef::EMPTY
};

/// 吸血蚊：戰吼，對敵方英雄造成 3 點傷害，為你的英雄恢復 3 點生命值。
static LIFEDRINKER: CardDef = CardDef {
    play: Some(Effect::Seq(&[
        Effect::Damage {
            to: Sel::EnemyHero,
            amount: 3,
        },
        Effect::Heal {
            to: Sel::OwnHero,
            amount: 3,
        },
    ])),
    ..CardDef::EMPTY
};

/// 血帆劫掠者：戰吼，獲得等同你武器攻擊力的攻擊力。
static BLOODSAIL_RAIDER: CardDef = CardDef {
    play: Some(Effect::Custom(bloodsail_raider)),
    ..CardDef::EMPTY
};

fn bloodsail_raider(g: &mut Game, ctx: &Ctx) {
    let attack = g.player(ctx.owner).weapon.map_or(0, |w| g.entity(w).attack);
    g.entity_mut(ctx.source).attack += attack;
}

/// 南海船長：你的其他海盜獲得 +1/+1。
static SOUTHSEA_CAPTAIN: CardDef = CardDef {
    aura: Some(Aura::Others {
        attack: 1,
        health: 1,
        race: Some("PIRATE"),
    }),
    ..CardDef::EMPTY
};

/// 平衡梟獸：二選一（同 RosettaStone 的順序）：1 為你的英雄恢復 8 點生命值；2 造成 4 點傷害。
static BOOMKIN: CardDef = CardDef {
    choose: Some([
        Choice {
            target: TargetReq::None,
            effect: Effect::Heal {
                to: Sel::OwnHero,
                amount: 8,
            },
        },
        Choice {
            target: TargetReq::Character,
            effect: Effect::Damage {
                to: Sel::Target,
                amount: 4,
            },
        },
    ]),
    ..CardDef::EMPTY
};

/// 惡魔斬隙者：戰吼，摧毀一個手下，你的英雄受到等同其生命值的傷害。
static RIFTCLEAVER: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::Custom(riftcleaver)),
    ..CardDef::EMPTY
};

fn riftcleaver(g: &mut Game, ctx: &Ctx) {
    let Some(t) = ctx.target else {
        return;
    };
    let health = g.entity(t).health;
    g.entity_mut(t).destroyed = true;
    let hero = g.player(ctx.owner).hero;
    g.deal_damage(hero, health, ctx.owner);
}

/// 開朗小幫手：戰吼，賦予一個友方手下 +2 生命值。
static BEAMING_SIDEKICK: CardDef = CardDef {
    target: TargetReq::FriendlyMinion,
    play: Some(Effect::Buff {
        to: Sel::Target,
        attack: 0,
        health: 2,
    }),
    ..CardDef::EMPTY
};

/// 受傷的托維爾人：嘲諷（卡表）；戰吼，對此手下造成 3 點傷害。
static INJURED_TOLVIR: CardDef = CardDef {
    play: Some(Effect::Damage {
        to: Sel::Itself,
        amount: 3,
    }),
    ..CardDef::EMPTY
};

/// 薩克特盜匪：死亡之聲，使一個隨機的敵方手下返回對手的手中。
/// 抽法（同 RosettaStone）：從還活著的敵方手下 `Int(0, n - 1)`。
static SAHKET_SAPPER: CardDef = CardDef {
    deathrattle: Some(Effect::ReturnRandomEnemyMinion),
    ..CardDef::EMPTY
};

/// 焦油蠕行者：嘲諷（卡表）；在對手的回合獲得 +2 攻擊力。
static TAR_CREEPER: CardDef = CardDef {
    aura: Some(Aura::OpponentTurn { attack: 2 }),
    ..CardDef::EMPTY
};

/// 哀嚎水氣：在你打出元素後，獲得 +1 攻擊力。
static WAILING_VAPOR: CardDef = CardDef {
    triggers: &[Trigger {
        when: When::AfterFriendlyPlay {
            race: Some("ELEMENTAL"),
        },
        effect: Effect::Buff {
            to: Sel::Itself,
            attack: 1,
            health: 0,
        },
    }],
    ..CardDef::EMPTY
};

/// 伊利達瑞審判官：衝刺（卡表）；在你的英雄攻擊一個敵人後，此手下也會攻擊該目標。
static ILLIDARI_INQUISITOR: CardDef = CardDef {
    triggers: &[Trigger {
        when: When::AfterOwnHeroAttacks,
        effect: Effect::AttackTarget,
    }],
    ..CardDef::EMPTY
};

/// 霧帆掠劫者：戰吼，若你裝備了武器，造成 2 點傷害（沒有武器時不用選目標）。
static FOGSAIL_FREEBOOTER: CardDef = CardDef {
    target: TargetReq::CharacterIfWeapon,
    play: Some(Effect::Damage {
        to: Sel::Target,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 雷歐克（動物夥伴）：你的其他手下獲得 +1 攻擊力。
static LEOKK: CardDef = CardDef {
    aura: Some(Aura::Others {
        attack: 1,
        health: 0,
        race: None,
    }),
    ..CardDef::EMPTY
};
