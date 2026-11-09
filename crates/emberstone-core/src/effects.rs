//! 卡片效果的資料表示：卡片定義（`CardDef`）、效果（`Effect`）、目標條件（`TargetReq`）、觸發（`Trigger`）。
//!
//! 具體卡片的定義在 `emberstone-cards`，開局時透過 `GameConfig::effects` 交給 core。

use crate::game::{EntityId, Game};

/// 依卡號查卡片效果。
pub type EffectLookup = fn(&str) -> Option<&'static CardDef>;

/// 一張卡的效果定義。沒寫的欄位用 `..CardDef::EMPTY`。
#[derive(Debug)]
pub struct CardDef {
    /// 打出（戰吼、法術、英雄技能、武器戰吼）時的目標條件。
    pub target: TargetReq,
    /// 打出時的效果：手下的戰吼、法術效果、英雄技能效果。
    pub play: Option<Effect>,
    /// 能不能打出／使用的額外條件。
    pub usable: Usable,
    /// 抽中時施放（施放後補抽一張）。
    pub when_drawn: Option<Effect>,
    /// 死亡之聲。
    pub deathrattle: Option<Effect>,
    /// 在場上時的觸發。
    pub triggers: &'static [Trigger],
    /// 在場上時的被動加成。
    pub aura: Option<Aura>,
    /// 二選一：choose_one 1、2 分別對應（順序同 RosettaStone）。
    pub choose: Option<[Choice; 2]>,
    /// 秘密：在對手回合符合條件時觸發（打出時放進秘密區，不執行 `play`）。
    pub secret: Option<Trigger>,
}

impl CardDef {
    pub const EMPTY: CardDef = CardDef {
        target: TargetReq::None,
        play: None,
        usable: Usable::Always,
        when_drawn: None,
        deathrattle: None,
        triggers: &[],
        aura: None,
        choose: None,
        secret: None,
    };
}

/// 二選一的一個選項。
#[derive(Debug, Clone, Copy)]
pub struct Choice {
    pub target: TargetReq,
    pub effect: Effect,
}

/// 被動加成（來源在場上才有效）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aura {
    /// 你的其他手下（race 有值時只限該種族）。
    Others {
        attack: i32,
        health: i32,
        race: Option<&'static str>,
    },
    /// 兩側的手下。
    Adjacent { attack: i32 },
    /// 自己受傷時。
    WhileDamaged { attack: i32 },
}

/// 目標條件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetReq {
    /// 不需要目標。
    None,
    /// 任一角色（雙方英雄和手下）。
    Character,
    /// 任一手下。
    Minion,
    FriendlyMinion,
    EnemyMinion,
    FriendlyCharacter,
    EnemyCharacter,
}

/// 能不能打出／使用的額外條件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usable {
    Always,
    /// 戰場還有空位（召喚類英雄技能）。
    BoardSlot,
}

/// 效果作用的對象。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sel {
    /// 打出時選的目標。
    Target,
    /// 效果來源自己（觸發的手下）。
    Itself,
    OwnHero,
    EnemyHero,
    /// 來源左右相鄰的手下。
    Adjacent,
    /// 自己的全部手下。
    FriendlyMinions,
    /// 全部角色：自己的手下、自己的英雄、對手的手下、對手的英雄（同 RosettaStone 的順序）。
    AllCharacters,
    /// 全部敵人：對手的手下、對手的英雄。
    EnemyCharacters,
}

/// 觸發的時機。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// 你召喚（其他）手下時；race 有值時只限該種族。
    FriendlySummon { race: Option<&'static str> },
    /// 任一手下受到傷害時（每隻受傷的手下觸發一次）。
    MinionDamaged,
    /// 你的回合結束時。
    OwnTurnEnd,
    /// 秘密：你的英雄被攻擊時（目標是攻擊者）。
    OwnHeroAttacked,
    /// 秘密：敵方手下攻擊時（目標是攻擊者）。
    EnemyMinionAttacks,
    /// 秘密：對手施放法術時（反制）。
    OpponentCastsSpell,
    /// 秘密：對手打出手下後（目標是那隻手下）。
    AfterOpponentPlaysMinion,
}

/// 在場上時的觸發。
#[derive(Debug, Clone, Copy)]
pub struct Trigger {
    pub when: When,
    pub effect: Effect,
}

/// 效果。
#[derive(Debug, Clone, Copy)]
pub enum Effect {
    /// 造成傷害；來源是法術時加上法術傷害。
    Damage { to: Sel, amount: i32 },
    /// 恢復生命，不超過上限。
    Heal { to: Sel, amount: i32 },
    /// 自己的英雄獲得護甲。
    Armor(i32),
    /// 自己的英雄本回合攻擊力 +n。
    HeroAttack(i32),
    /// 抽 n 張牌。
    Draw(i32),
    /// 本回合可用法力 +n（可以超過 10）。
    TempMana(i32),
    /// 召喚手下：戰吼放在來源右邊、死亡之聲放在死亡的位置、其他放最右邊。
    Summon { card: &'static str, count: i32 },
    /// 裝備武器（舊武器被摧毀）。
    Equip(&'static str),
    /// 消滅（死亡階段移出戰場）。
    Destroy(Sel),
    /// 永久加攻擊力／生命。
    Buff { to: Sel, attack: i32, health: i32 },
    /// 本回合加攻擊力。
    TurnAttack { to: Sel, amount: i32 },
    /// 賦予嘲諷。
    Taunt(Sel),
    /// 賦予聖盾術。
    DivineShield(Sel),
    /// 變形成另一張卡。
    Transform { to: Sel, card: &'static str },
    /// 從牌庫發現一張牌（抽法同 RosettaStone）。
    DiscoverFromDeck,
    /// 造成 n 次 1 點傷害，每次從「其他角色」隨機挑一個（抽法同 RosettaStone）。
    RandomSplitDamage(i32),
    /// 沉默。
    Silence(Sel),
    /// 回到擁有者的手牌，費用 +cost（手牌滿了就消滅）。
    ReturnToHand { to: Sel, cost: i32 },
    /// 反制：讓觸發這個秘密的法術無效。
    Counter,
    /// 依序執行。
    Seq(&'static [Effect]),
    /// 特殊卡的效果函式。
    Custom(fn(&mut Game, &Ctx)),
}

impl Effect {
    /// 效果裡有召喚手下（二選一的召喚選項在戰場已滿時不能選）。
    pub fn summons(&self) -> bool {
        match self {
            Effect::Summon { .. } => true,
            Effect::Seq(effects) => effects.iter().any(Effect::summons),
            _ => false,
        }
    }
}

/// 效果執行時的情境。
#[derive(Debug, Clone, Copy)]
pub struct Ctx {
    /// 效果來源（卡片、英雄技能、觸發的手下）。
    pub source: EntityId,
    /// 來源的擁有者。
    pub owner: usize,
    /// 打出時選的目標。
    pub target: Option<EntityId>,
    /// 來源是法術（傷害加上法術傷害）。
    pub spell: bool,
    /// 死亡之聲（召喚放在死亡的位置）。
    pub deathrattle: bool,
}
