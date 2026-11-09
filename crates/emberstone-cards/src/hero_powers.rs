//! 十個職業的預設英雄技能（和 RosettaStone 相同的卡號）。

use emberstone_core::{CardDef, Ctx, Effect, Game, Sel, TargetReq, Usable};

pub(crate) fn lookup(id: &str) -> Option<&'static CardDef> {
    Some(match id {
        "HERO_01bp" => &ARMOR_UP,
        "HERO_02bp" => &TOTEMIC_CALL,
        "HERO_03bp" => &DAGGER_MASTERY,
        "HERO_04bp" => &REINFORCE,
        "HERO_05bp" => &STEADY_SHOT,
        "HERO_06bp" => &SHAPESHIFT,
        "HERO_07bp" => &LIFE_TAP,
        "HERO_08bp" => &FIREBLAST,
        "HERO_09bp" => &LESSER_HEAL,
        "HERO_10bp" => &DEMON_CLAWS,
        _ => return None,
    })
}

/// 護甲提升！：護甲 +2。
static ARMOR_UP: CardDef = CardDef {
    play: Some(Effect::Armor(2)),
    ..CardDef::EMPTY
};

/// 召喚圖騰：從場上還沒有的基本圖騰中隨機召喚 1 個；4 種都在時從四種中隨機。
static TOTEMIC_CALL: CardDef = CardDef {
    play: Some(Effect::Custom(totemic_call)),
    usable: Usable::BoardSlot,
    ..CardDef::EMPTY
};

/// 基本圖騰，順序同 RosettaStone 的候選清單（灼熱、石爪、風懲、治療）。
const BASIC_TOTEMS: [&str; 4] = ["CS2_050", "CS2_051", "CS2_052", "NEW1_009"];

/// 抽法（同 RosettaStone）：候選依 `BASIC_TOTEMS` 順序，去掉場上已有的；
/// 候選為空時用全部四種。從 n 個中挑一個：`Int(0, n - 1)`。召喚在最右邊。
fn totemic_call(g: &mut Game, ctx: &Ctx) {
    let on_board = |g: &Game, id: &str| {
        g.player(ctx.owner)
            .board
            .iter()
            .any(|&m| g.entity(m).card.id == id)
    };
    let mut candidates: Vec<&str> = BASIC_TOTEMS
        .iter()
        .copied()
        .filter(|id| !on_board(g, id))
        .collect();
    if candidates.is_empty() {
        candidates = BASIC_TOTEMS.to_vec();
    }
    let i = g.random_int(0, candidates.len() as i64 - 1) as usize;
    let pos = g.player(ctx.owner).board.len();
    g.summon(ctx.owner, candidates[i], pos);
}

/// 匕首專精：裝備 1/2 匕首。
static DAGGER_MASTERY: CardDef = CardDef {
    play: Some(Effect::Equip("CS2_082")),
    ..CardDef::EMPTY
};

/// 援軍：召喚 1/1 白銀之手新兵。
static REINFORCE: CardDef = CardDef {
    play: Some(Effect::Summon {
        card: "CS2_101t",
        count: 1,
    }),
    usable: Usable::BoardSlot,
    ..CardDef::EMPTY
};

/// 穩固射擊：對敵方英雄造成 2 點傷害。
static STEADY_SHOT: CardDef = CardDef {
    play: Some(Effect::Damage {
        to: Sel::EnemyHero,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 變身：本回合攻擊力 +1、護甲 +1。
static SHAPESHIFT: CardDef = CardDef {
    play: Some(Effect::Seq(&[Effect::HeroAttack(1), Effect::Armor(1)])),
    ..CardDef::EMPTY
};

/// 生命分流：先受到 2 點傷害，再抽一張牌。
static LIFE_TAP: CardDef = CardDef {
    play: Some(Effect::Seq(&[
        Effect::Damage {
            to: Sel::OwnHero,
            amount: 2,
        },
        Effect::Draw(1),
    ])),
    ..CardDef::EMPTY
};

/// 火焰衝擊：造成 1 點傷害。
static FIREBLAST: CardDef = CardDef {
    target: TargetReq::Character,
    play: Some(Effect::Damage {
        to: Sel::Target,
        amount: 1,
    }),
    ..CardDef::EMPTY
};

/// 次級治療術：恢復 2 點生命值。
static LESSER_HEAL: CardDef = CardDef {
    target: TargetReq::Character,
    play: Some(Effect::Heal {
        to: Sel::Target,
        amount: 2,
    }),
    ..CardDef::EMPTY
};

/// 惡魔之爪：本回合攻擊力 +1。
static DEMON_CLAWS: CardDef = CardDef {
    play: Some(Effect::HeroAttack(1)),
    ..CardDef::EMPTY
};
