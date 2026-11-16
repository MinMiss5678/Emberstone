//! 秘密與沉默的範例卡。

use emberstone_core::{CardDef, Ctx, Effect, Game, Sel, TargetReq, Trigger, When};

pub(crate) fn lookup(id: &str) -> Option<&'static CardDef> {
    Some(match id {
        "CS2_203" | "CORE_CS2_203" => &IRONBEAK_OWL,
        "CORE_EX1_289" => &ICE_BARRIER,
        "CORE_EX1_610" => &EXPLOSIVE_TRAP,
        "CORE_EX1_611" => &FREEZING_TRAP,
        "CORE_EX1_287" => &COUNTERSPELL,
        "CORE_LOOT_101" => &EXPLOSIVE_RUNES,
        "CORE_BAR_812" => &OASIS_ALLY,
        "CORE_ULD_152" => &PRESSURE_PLATE,
        _ => return None,
    })
}

const fn secret(when: When, effect: Effect) -> CardDef {
    CardDef {
        secret: Some(Trigger { when, effect }),
        ..CardDef::EMPTY
    }
}

/// 鐵喙貓頭鷹：戰吼，沉默一個手下。
static IRONBEAK_OWL: CardDef = CardDef {
    target: TargetReq::Minion,
    play: Some(Effect::Silence(Sel::Target)),
    ..CardDef::EMPTY
};

/// 寒冰護體：秘密，當你的英雄被攻擊時，獲得 8 點護甲值。
static ICE_BARRIER: CardDef = secret(When::OwnHeroAttacked, Effect::Armor(8));

/// 爆炸陷阱：秘密，當你的英雄被攻擊時，對全部敵人造成 2 點傷害。
static EXPLOSIVE_TRAP: CardDef = secret(
    When::OwnHeroAttacked,
    Effect::Damage {
        to: Sel::EnemyCharacters,
        amount: 2,
    },
);

/// 冰凍陷阱：秘密，當敵方手下攻擊時，使其返回擁有者的手牌，且消耗增加 2。
static FREEZING_TRAP: CardDef = secret(
    When::EnemyMinionAttacks,
    Effect::ReturnToHand {
        to: Sel::Target,
        cost: 2,
    },
);

/// 法術反制：秘密，當對手施放法術時，反制。
static COUNTERSPELL: CardDef = secret(When::OpponentCastsSpell, Effect::Counter);

/// 綠洲盟友：秘密，當一個友方手下受到攻擊時，召喚一個 3/6 水元素（戰場滿了不觸發）。
static OASIS_ALLY: CardDef = secret(
    When::FriendlyMinionAttacked,
    Effect::Summon {
        card: "CS2_033",
        count: 1,
    },
);

/// 踏板陷阱：秘密，在對手施放法術後，摧毀一個隨機敵方手下（對手沒有手下時不觸發）。
/// 抽法（同 RosettaStone）：從還活著的敵方手下 `Int(0, n - 1)`。
static PRESSURE_PLATE: CardDef = secret(
    When::AfterOpponentCastsSpell,
    Effect::DestroyRandomEnemyMinion,
);

/// 爆炸符文：秘密，在對手打出手下後，對其造成 6 點傷害，並將溢傷分給敵方英雄（不受法術傷害影響）。
static EXPLOSIVE_RUNES: CardDef = secret(
    When::AfterOpponentPlaysMinion,
    Effect::Custom(explosive_runes),
);

fn explosive_runes(g: &mut Game, ctx: &Ctx) {
    let Some(m) = ctx.target else {
        return;
    };
    let health = g.entity(m).health;
    let owner = g.entity(m).owner;
    // 6 點加上秘密擁有者的法術傷害；溢出照總量算（第 3a 階段 A1）
    let total = 6 + g.spell_damage(ctx.owner);
    g.deal_damage(m, total, ctx.owner);
    let excess = total - health;
    if excess > 0 {
        let hero = g.player(owner).hero;
        g.deal_damage(hero, excess, ctx.owner);
    }
}
