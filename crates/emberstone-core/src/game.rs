//! 對局狀態、回合流程、出牌、戰鬥與合法行動。
//!
//! 第 1 階段：數值與關鍵字。第 2 階段：卡片效果（英雄技能、指定目標、法術、戰吼……），
//! 效果定義由 `emberstone-cards` 透過 `GameConfig::effects` 提供。

use emberstone_data::{Card, CardClass, CardType, Keyword, all_cards, card_by_id, is_standard_set};

use crate::effects::{Aura, CardDef, Ctx, Effect, EffectLookup, Sel, TargetReq, Usable, When};
use crate::rng::{Desync, Rng, RngEvent};

/// 實體編號（`Game` 內實體陣列的索引）。
pub type EntityId = usize;

const HAND_LIMIT: usize = 10;
const BOARD_LIMIT: usize = 7;
const MANA_LIMIT: i32 = 10;
const MAX_SECRETS: usize = 5;
/// 幸運幣。後手開局拿到是遊戲規則，所以由 core 放進手牌；效果在 cards。
const COIN: &str = "GAME_005";

/// 指定的角色：英雄或戰場上第 index 隻手下。player 為 0（先手）或 1（後手）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Hero { player: usize },
    Minion { player: usize, index: usize },
}

/// 一個行動（用位置描述，和差分測試的格式相同）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    /// 打出手牌第 hand 張；手下要指定放在戰場第 pos 格；二選一卡 choose_one 為 1 或 2，其餘為 0。
    Play {
        hand: usize,
        pos: Option<usize>,
        target: Option<Target>,
        choose_one: u8,
    },
    Attack {
        attacker: Target,
        target: Target,
    },
    Power {
        target: Option<Target>,
    },
    /// 發現等選擇畫面，選第 index 個。
    Choose {
        index: usize,
    },
    End,
}

/// 行動失敗的原因；失敗時狀態不變。
#[derive(Debug, Clone, PartialEq)]
pub enum ActError {
    Illegal,
    GameOver,
    Desync(Desync),
}

/// 開局失敗的原因。
#[derive(Debug, Clone, PartialEq)]
pub enum GameError {
    UnknownCard(String),
    Desync(Desync),
}

/// 對局結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won(usize),
    Draw,
}

/// 卡片效果定義的參照；比較時比位址（同一張卡的定義只有一份）。
#[derive(Clone, Copy)]
pub struct DefRef(pub &'static CardDef);

impl PartialEq for DefRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl std::fmt::Debug for DefRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DefRef")
    }
}

/// 英雄、手下、武器、英雄技能或手牌／牌庫中的卡。
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub card: &'static Card,
    /// 卡片效果；沒有效果的卡為 None。
    pub def: Option<DefRef>,
    pub owner: usize,
    pub attack: i32,
    pub health: i32,
    pub max_health: i32,
    pub armor: i32,
    pub durability: i32,
    pub taunt: bool,
    pub divine_shield: bool,
    pub stealth: bool,
    pub windfury: bool,
    pub charge: bool,
    pub rush: bool,
    pub poisonous: bool,
    pub lifesteal: bool,
    pub freeze: bool,
    pub frozen: bool,
    /// 法術傷害 +n。
    pub spell_damage: i32,
    /// 本回合剛上場（沒有衝鋒就不能攻擊；有衝刺只能攻擊手下）。
    pub summoning_sick: bool,
    pub attacks_this_turn: i32,
    /// 本回合效果要求的攻擊次數（伊利達瑞審判官、戰槌挑戰者）：算進 attacks_this_turn，但不佔攻擊次數。
    pub forced_attacks: i32,
    /// 「本回合」加的攻擊力，回合結束時扣回。
    pub temp_attack: i32,
    /// 被致命劇毒等效果消滅，等死亡階段移出戰場。
    pub destroyed: bool,
    /// 上場順序（越小越早）；死亡之聲依此結算。
    pub play_order: u32,
    /// 目前套用中的被動加成（攻擊力、生命）。
    pub aura_attack: i32,
    pub aura_health: i32,
    /// 已被沉默（效果、觸發、被動加成、死亡之聲都失效）。
    pub silenced: bool,
    /// 費用增減。
    pub cost_mod: i32,
    /// 費用設為固定值（例：阿拉巴斯特給的複製設為 1）；之後才套用費用增減。
    pub cost_set: Option<i32>,
    /// 休眠：還要經過幾個自己的回合開始才甦醒；0 為沒有休眠。
    pub dormant: i32,
}

impl Entity {
    fn new(card: &'static Card, def: Option<&'static CardDef>, owner: usize) -> Self {
        let has = |k: Keyword| card.keywords.contains(&k);
        let health = card.health.unwrap_or(0);
        Entity {
            card,
            def: def.map(DefRef),
            owner,
            attack: card.attack.unwrap_or(0),
            health,
            max_health: health,
            armor: card.armor.unwrap_or(0),
            durability: card.durability.unwrap_or(0),
            taunt: has(Keyword::Taunt),
            divine_shield: has(Keyword::DivineShield),
            stealth: has(Keyword::Stealth),
            windfury: has(Keyword::Windfury),
            charge: has(Keyword::Charge),
            rush: has(Keyword::Rush),
            poisonous: has(Keyword::Poisonous),
            lifesteal: has(Keyword::Lifesteal),
            freeze: has(Keyword::Freeze),
            frozen: false,
            spell_damage: card.spell_damage,
            summoning_sick: false,
            attacks_this_turn: 0,
            forced_attacks: 0,
            temp_attack: 0,
            destroyed: false,
            play_order: 0,
            aura_attack: 0,
            aura_health: 0,
            silenced: false,
            cost_mod: 0,
            cost_set: None,
            dormant: 0,
        }
    }

    fn def(&self) -> Option<&'static CardDef> {
        if self.silenced {
            None
        } else {
            self.def.map(|d| d.0)
        }
    }

    /// 本回合自己用掉的攻擊次數（不含效果要求的攻擊）。
    fn used_attacks(&self) -> i32 {
        self.attacks_this_turn - self.forced_attacks
    }

    /// 目前費用（不低於 0）。
    pub fn cost(&self) -> i32 {
        (self.card.cost.unwrap_or(0) + self.cost_mod).max(0)
    }
}

/// 一位玩家。牌庫第 0 張是下一張要抽的牌。
#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub hero: EntityId,
    pub hero_power: EntityId,
    pub weapon: Option<EntityId>,
    pub hand: Vec<EntityId>,
    pub deck: Vec<EntityId>,
    pub board: Vec<EntityId>,
    /// 目前可用的法力（含幸運幣等暫時法力）。
    pub mana: i32,
    pub max_mana: i32,
    pub overload_locked: i32,
    pub overload_owed: i32,
    /// 疲勞累計：上一次疲勞傷害的點數。
    pub fatigue: i32,
    /// 本回合已使用英雄技能。
    pub power_used: bool,
    /// 秘密區（依打出順序）。
    pub secrets: Vec<EntityId>,
    /// 本回合下一個法術的減費（準備）；打出任何法術或回合結束就歸零。
    pub spell_discount: i32,
    /// 本回合打出的法術（含被反制的、秘密、幸運幣）。
    pub spells_this_turn: Vec<&'static str>,
    /// 上一個自己的回合打出的法術（奎格瓦）。
    pub spells_last_turn: Vec<&'static str>,
}

/// 一方的英雄與牌組（照抽牌順序）。英雄技能是英雄卡號加上 `bp`。
#[derive(Debug, Clone)]
pub struct PlayerConfig {
    pub hero: &'static str,
    pub deck: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub enum RngSource {
    Seed(u64),
    Scripted(Vec<RngEvent>),
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub players: [PlayerConfig; 2],
    /// 開局是否洗牌（先洗先手再洗後手）。
    pub shuffle: bool,
    pub rng: RngSource,
    /// 卡片效果（由 emberstone-cards 提供）；None 表示只有數值和關鍵字。
    pub effects: Option<EffectLookup>,
    /// 從牌庫發現時的排序鍵；None 為 dbfId。差分測試用來讓 RosettaStone 以原版卡號實作的卡照原版 dbfId 排序。
    pub discover_order: Option<fn(&'static Card) -> u32>,
    /// 隨機產生（發現等）的候選卡，依序；None 為標準模式的可收藏卡。差分測試用來讓兩個引擎的候選相同。
    pub random_pool: Option<&'static [&'static str]>,
}

/// 卡片效果的查詢函式；比較對局時不比它（同一局一定相同）。
#[derive(Clone, Copy)]
struct Effects(Option<EffectLookup>);

impl PartialEq for Effects {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for Effects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Effects")
    }
}

/// 從牌庫發現時的排序鍵；比較對局時不比它（同一局一定相同）。
#[derive(Clone, Copy)]
struct DiscoverOrder(Option<fn(&'static Card) -> u32>);

impl PartialEq for DiscoverOrder {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for DiscoverOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiscoverOrder")
    }
}

/// 隨機產生的候選卡；比較對局時不比它（同一局一定相同）。
#[derive(Clone, Copy)]
struct RandomPool(Option<&'static [&'static str]>);

impl PartialEq for RandomPool {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for RandomPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RandomPool")
    }
}

/// 一局對戰。整份可複製。
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    entities: Vec<Entity>,
    players: [Player; 2],
    current: usize,
    turn: u32,
    outcome: Option<Outcome>,
    rng: Rng,
    /// 效果執行中亂數不同步（之後的行動都拒絕）。
    desync: Option<Desync>,
    effects: Effects,
    discover_order: DiscoverOrder,
    random_pool: RandomPool,
    /// 下一個上場的順序編號。
    next_order: u32,
    /// 死亡階段中，已移出戰場、死亡之聲還沒結算完的手下：(手下, 擁有者, 召喚位置)。
    dr_markers: Vec<(EntityId, usize, usize)>,
    /// 死亡階段中，已死亡的被動加成來源給的加成：(手下, 攻擊力, 生命)，死亡之聲都結算完才移除。
    lingering: Vec<(EntityId, i32, i32)>,
    /// 秘密結算中：被動加成等同一次觸發的秘密都結算完才更新（case I3d）。
    defer_auras: bool,
    /// 發現等選擇畫面：(玩家, 選項)。
    choice: Option<(usize, Vec<EntityId>)>,
    /// 目前施放的法術被反制。
    countered: bool,
    /// 選擇畫面出現時暫停的出牌：選完才結算它之後的流程（死亡階段、秘密；第 3b 階段 C3b）。
    pending: Option<EntityId>,
}

fn lookup(id: &str) -> Result<&'static Card, GameError> {
    card_by_id(id).ok_or_else(|| GameError::UnknownCard(id.to_string()))
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Game, GameError> {
        let rng = match config.rng {
            RngSource::Seed(s) => Rng::seeded(s),
            RngSource::Scripted(events) => Rng::scripted(events),
        };
        let effects = Effects(config.effects);
        let def_of = |id: &str| effects.0.and_then(|f| f(id));
        let mut entities = Vec::new();
        let mut make_player = |p: usize, cfg: &PlayerConfig| -> Result<Player, GameError> {
            let hero = entities.len();
            entities.push(Entity::new(lookup(cfg.hero)?, def_of(cfg.hero), p));
            let power_id = format!("{}bp", cfg.hero);
            let hero_power = entities.len();
            let power_card = lookup(&power_id)?;
            entities.push(Entity::new(power_card, def_of(power_card.id), p));
            let mut deck = Vec::with_capacity(cfg.deck.len());
            for id in &cfg.deck {
                deck.push(entities.len());
                entities.push(Entity::new(lookup(id)?, def_of(id), p));
            }
            Ok(Player {
                hero,
                hero_power,
                weapon: None,
                hand: Vec::with_capacity(HAND_LIMIT),
                deck,
                board: Vec::with_capacity(BOARD_LIMIT),
                mana: 0,
                max_mana: 0,
                overload_locked: 0,
                overload_owed: 0,
                fatigue: 0,
                power_used: false,
                secrets: Vec::new(),
                spell_discount: 0,
                spells_this_turn: Vec::new(),
                spells_last_turn: Vec::new(),
            })
        };
        let p0 = make_player(0, &config.players[0])?;
        let p1 = make_player(1, &config.players[1])?;
        let mut g = Game {
            entities,
            players: [p0, p1],
            current: 0,
            turn: 1,
            outcome: None,
            rng,
            desync: None,
            effects,
            discover_order: DiscoverOrder(config.discover_order),
            random_pool: RandomPool(config.random_pool),
            next_order: 1,
            dr_markers: Vec::new(),
            lingering: Vec::new(),
            defer_auras: false,
            choice: None,
            countered: false,
            pending: None,
        };
        if config.shuffle {
            for p in 0..2 {
                let deck = &g.players[p].deck;
                let perm = g.rng.shuffle(deck.len()).map_err(GameError::Desync)?;
                let shuffled = perm.iter().map(|&i| deck[i]).collect();
                g.players[p].deck = shuffled;
            }
        }
        for _ in 0..3 {
            g.draw(0);
        }
        for _ in 0..4 {
            g.draw(1);
        }
        g.add_to_hand(1, COIN);
        g.start_turn();
        if let Some(d) = g.desync.take() {
            return Err(GameError::Desync(d));
        }
        Ok(g)
    }

    pub fn turn(&self) -> u32 {
        self.turn
    }

    pub fn current_player(&self) -> usize {
        self.current
    }

    pub fn player(&self, player: usize) -> &Player {
        &self.players[player]
    }

    pub fn player_mut(&mut self, player: usize) -> &mut Player {
        &mut self.players[player]
    }

    pub fn entity(&self, id: EntityId) -> &Entity {
        &self.entities[id]
    }

    pub fn entity_mut(&mut self, id: EntityId) -> &mut Entity {
        &mut self.entities[id]
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// 手牌中一張牌目前的費用（含流放、準備等調整）。
    pub fn hand_cost(&self, id: EntityId) -> i32 {
        let e = &self.entities[id];
        let owner = &self.players[e.owner];
        let mut cost = e.cost_set.unwrap_or(e.card.cost.unwrap_or(0));
        // 流放的費用：在手牌最左或最右時
        if let Some(c) = e.def().and_then(|d| d.outcast_cost)
            && let Some(i) = owner.hand.iter().position(|&h| h == id)
            && (i == 0 || i + 1 == owner.hand.len())
        {
            cost = c;
        }
        // 先設為固定值，再套用增減（第 3b 階段 A6）
        cost += e.cost_mod;
        if e.card.card_type == CardType::Spell {
            cost -= owner.spell_discount;
        }
        cost.max(0)
    }

    /// 英雄目前的攻擊力（自己回合才算武器）。
    pub fn hero_attack(&self, player: usize) -> i32 {
        let base = self.entities[self.players[player].hero].attack;
        match self.players[player].weapon {
            Some(w) if player == self.current => base + self.entities[w].attack,
            _ => base,
        }
    }

    /// 一方場上手下的法術傷害總和。
    pub fn spell_damage(&self, player: usize) -> i32 {
        self.players[player]
            .board
            .iter()
            .map(|&m| self.entities[m].spell_damage)
            .sum()
    }

    fn def_of(&self, id: &str) -> Option<&'static CardDef> {
        self.effects.0.and_then(|f| f(id))
    }

    fn new_entity(&mut self, card: &'static Card, owner: usize) -> EntityId {
        let id = self.entities.len();
        let def = self.def_of(card.id);
        self.entities.push(Entity::new(card, def, owner));
        id
    }

    /// 加一張卡到手牌最右邊；手牌已滿回傳 None。
    pub fn add_to_hand(&mut self, player: usize, card_id: &str) -> Option<EntityId> {
        let card = card_by_id(card_id)?;
        if self.players[player].hand.len() >= HAND_LIMIT {
            return None;
        }
        let id = self.new_entity(card, player);
        self.players[player].hand.push(id);
        Some(id)
    }

    /// 召喚一隻手下到戰場第 pos 格；戰場已滿或卡號不明回傳 None。
    pub fn summon(&mut self, player: usize, card_id: &str, pos: usize) -> Option<EntityId> {
        let card = card_by_id(card_id)?;
        let board = &self.players[player].board;
        if board.len() >= BOARD_LIMIT || pos > board.len() {
            return None;
        }
        let id = self.new_entity(card, player);
        self.entities[id].summoning_sick = true;
        self.enter_board(player, id, pos);
        self.fire_summon(id);
        Some(id)
    }

    /// 手下上場：放進戰場並記下上場順序。
    fn enter_board(&mut self, player: usize, id: EntityId, pos: usize) {
        self.players[player].board.insert(pos, id);
        self.entities[id].play_order = self.next_order;
        self.next_order += 1;
        self.refresh_auras();
    }

    /// 把戰場上的手下變形成另一張卡（第 2 階段 F1）。
    pub fn transform(&mut self, id: EntityId, card_id: &str) {
        let Some(card) = card_by_id(card_id) else {
            return;
        };
        let old = &self.entities[id];
        let (owner, play_order) = (old.owner, old.play_order);
        let mut e = Entity::new(card, self.def_of(card.id), owner);
        e.play_order = play_order;
        // 視同剛上場
        e.summoning_sick = true;
        self.entities[id] = e;
        self.refresh_auras();
    }

    /// 一方戰場上有沒有不在休眠的手下。
    fn has_active_minions(&self, player: usize) -> bool {
        self.players[player]
            .board
            .iter()
            .any(|&m| self.entities[m].dormant == 0)
    }

    /// 目前的選擇畫面選項（發現等）。
    pub fn choice(&self) -> Option<&[EntityId]> {
        self.choice.as_ref().map(|(_, v)| v.as_slice())
    }

    /// 一張手牌可以怎麼打出：(choose_one, 目標條件)。二選一卡有兩個選項；
    /// 戰場已滿時召喚的選項不能選。
    fn play_options(&self, entity: &Entity) -> Vec<(u8, TargetReq)> {
        match entity.def() {
            Some(CardDef {
                choose: Some(c), ..
            }) => {
                let full = self.players[entity.owner].board.len() >= BOARD_LIMIT;
                [1, 2]
                    .into_iter()
                    .zip(c.iter())
                    .filter(|(_, choice)| !(full && choice.effect.summons()))
                    .map(|(which, choice)| (which, choice.target))
                    .collect()
            }
            Some(d) => vec![(0, d.target)],
            None => vec![(0, TargetReq::None)],
        }
    }

    /// 重算被動加成：每隻手下該有的加成和目前套用的差多少就補多少。
    /// 生命加成減少時，目前生命只在超過新上限時才降。
    fn refresh_auras(&mut self) {
        if self.defer_auras {
            return;
        }
        for p in 0..2 {
            for i in 0..self.players[p].board.len() {
                self.refresh_minion_aura(p, i);
            }
        }
        // 生命上限調整後可能剛好回滿：「受傷時」的加成要照調整後的生命再算一次（case E4、E7）
        for p in 0..2 {
            for i in 0..self.players[p].board.len() {
                let m = self.players[p].board[i];
                if matches!(
                    self.entities[m].def().and_then(|d| d.aura),
                    Some(Aura::WhileDamaged { .. })
                ) {
                    self.refresh_minion_aura(p, i);
                }
            }
        }
    }

    /// 重算 players[p].board[i] 的被動加成，和目前套用的差多少就補多少。
    fn refresh_minion_aura(&mut self, p: usize, i: usize) {
        let board = &self.players[p].board;
        let m = board[i];
        let (mut attack, mut health) = (0, 0);
        for j in 0..board.len() {
            let (a, h) = self.aura_bonus(board, j, i);
            attack += a;
            health += h;
        }
        for &(t, a, h) in &self.lingering {
            if t == m {
                attack += a;
                health += h;
            }
        }
        let e = &mut self.entities[m];
        e.attack += attack - e.aura_attack;
        e.aura_attack = attack;
        let dh = health - e.aura_health;
        e.max_health += dh;
        if dh > 0 {
            e.health += dh;
        } else {
            e.health = e.health.min(e.max_health);
        }
        e.aura_health = health;
    }

    /// 戰場上 board[j] 的被動加成給 board[i] 的（攻擊力, 生命）。
    fn aura_bonus(&self, board: &[EntityId], j: usize, i: usize) -> (i32, i32) {
        let (s, m) = (board[j], board[i]);
        // 休眠中的手下不給也不受被動加成（第 3b 階段 B3）
        if self.entities[s].dormant > 0 || self.entities[m].dormant > 0 {
            return (0, 0);
        }
        let Some(aura) = self.entities[s].def().and_then(|d| d.aura) else {
            return (0, 0);
        };
        let target = &self.entities[m];
        match aura {
            Aura::Others {
                attack,
                health,
                race,
            } if s != m && race.is_none_or(|r| target.card.races.contains(&r)) => (attack, health),
            Aura::Adjacent { attack } if i.abs_diff(j) == 1 => (attack, 0),
            Aura::WhileDamaged { attack } if s == m && target.health < target.max_health => {
                (attack, 0)
            }
            Aura::OpponentTurn { attack } if s == m && self.current != target.owner => (attack, 0),
            _ => (0, 0),
        }
    }

    /// 「手下受到傷害時」的觸發（雙方，依上場順序）。
    fn fire_damaged(&mut self) {
        let mut holders: Vec<EntityId> = self
            .players
            .iter()
            .flat_map(|p| p.board.iter().copied())
            .collect();
        holders.sort_by_key(|&m| self.entities[m].play_order);
        for m in holders {
            let Some(def) = self.entities[m].def() else {
                continue;
            };
            for t in def.triggers {
                if t.when == When::MinionDamaged {
                    let ctx = Ctx {
                        source: m,
                        owner: self.entities[m].owner,
                        target: None,
                        spell: false,
                        deathrattle: false,
                    };
                    self.run(&t.effect, &ctx);
                }
            }
        }
    }

    /// 「你召喚手下時」的觸發（依上場順序）。
    fn fire_summon(&mut self, summoned: EntityId) {
        self.fire_friendly_minion(summoned, |w| match w {
            When::FriendlySummon { race } => Some(race),
            _ => None,
        });
    }

    /// 「你打出手下後」的觸發（依上場順序）。
    fn fire_after_play(&mut self, played: EntityId) {
        self.fire_friendly_minion(played, |w| match w {
            When::AfterFriendlyPlay { race } => Some(race),
            _ => None,
        });
    }

    /// 友方手下 minion 引起的觸發（召喚、打出）：kind 從觸發時機取出種族條件，不是這類觸發回傳 None。
    fn fire_friendly_minion(
        &mut self,
        minion: EntityId,
        kind: fn(When) -> Option<Option<&'static str>>,
    ) {
        let summoned = minion;
        let owner = self.entities[summoned].owner;
        let races = self.entities[summoned].card.races;
        let mut holders: Vec<EntityId> = self.players[owner]
            .board
            .iter()
            .copied()
            .filter(|&m| m != summoned && self.entities[m].dormant == 0)
            .collect();
        holders.sort_by_key(|&m| self.entities[m].play_order);
        for m in holders {
            let Some(def) = self.entities[m].def() else {
                continue;
            };
            for t in def.triggers {
                let Some(race) = kind(t.when) else {
                    continue;
                };
                if race.is_none_or(|r| races.contains(&r)) {
                    let ctx = Ctx {
                        source: m,
                        owner,
                        target: None,
                        spell: false,
                        deathrattle: false,
                    };
                    self.run(&t.effect, &ctx);
                }
            }
        }
    }

    /// 依上場順序、不在休眠的觸發者：players 中各方戰場上，有 when 這種觸發的手下。
    fn trigger_holders(&self, players: &[usize], when: When) -> Vec<EntityId> {
        let mut holders: Vec<EntityId> = players
            .iter()
            .flat_map(|&p| self.players[p].board.iter().copied())
            .filter(|&m| {
                self.entities[m].dormant == 0
                    && self.entities[m]
                        .def()
                        .is_some_and(|d| d.triggers.iter().any(|t| t.when == when))
            })
            .collect();
        holders.sort_by_key(|&m| self.entities[m].play_order);
        holders
    }

    /// 執行 m 身上 when 這種觸發（目標是 target）。
    fn fire_holder(&mut self, m: EntityId, when: When, target: Option<EntityId>) {
        let Some(def) = self.entities[m].def() else {
            return;
        };
        for t in def.triggers {
            if t.when == when {
                let ctx = Ctx {
                    source: m,
                    owner: self.entities[m].owner,
                    target,
                    spell: false,
                    deathrattle: false,
                };
                self.run(&t.effect, &ctx);
            }
        }
    }

    /// 「任一玩家打出法術時」的觸發（博學行者阿洲；雙方，依上場順序）。
    fn fire_spell_played(&mut self, spell: EntityId) {
        for m in self.trigger_holders(&[0, 1], When::AnySpellPlayed) {
            self.fire_holder(m, When::AnySpellPlayed, Some(spell));
        }
    }

    /// 「對手抽到牌時」的觸發（大鑰師阿拉巴斯特）：p 抽到 card。
    fn fire_drawn(&mut self, p: usize, card: EntityId) {
        for m in self.trigger_holders(&[1 - p], When::OpponentDraws) {
            self.fire_holder(m, When::OpponentDraws, Some(card));
        }
    }

    /// 效果召喚的手下多召喚一隻（卡德加）：只有第一隻卡德加觸發（第 3b 階段 E4）。
    fn double_summon(&mut self, summoned: EntityId) {
        let owner = self.entities[summoned].owner;
        let holders = self.trigger_holders(&[owner], When::FriendlySummonByEffect);
        if let Some(&m) = holders.iter().find(|&&m| m != summoned) {
            self.fire_holder(m, When::FriendlySummonByEffect, Some(summoned));
        }
    }

    /// 角色還在場上而且活著（生命 > 0、沒有被消滅）。
    pub fn in_play(&self, id: EntityId) -> bool {
        let e = &self.entities[id];
        e.health > 0
            && !e.destroyed
            && (self.players[e.owner].board.contains(&id) || self.players[e.owner].hero == id)
    }

    /// 效果用的亂數：閉區間 [lo, hi]。不同步時記下並回傳 lo（這個行動結束後回報）。
    pub fn random_int(&mut self, lo: i64, hi: i64) -> i64 {
        match self.rng.int(lo, hi) {
            Ok(v) => v,
            Err(d) => {
                self.desync.get_or_insert(d);
                lo
            }
        }
    }

    /// 目前玩家的所有合法行動；遊戲結束時為空。
    pub fn legal_actions(&self) -> Vec<Action> {
        let mut out = Vec::new();
        if self.outcome.is_some() || self.desync.is_some() {
            return out;
        }
        if let Some((_, options)) = &self.choice {
            return (0..options.len())
                .map(|index| Action::Choose { index })
                .collect();
        }
        let me = &self.players[self.current];
        for (i, &e) in me.hand.iter().enumerate() {
            let entity = &self.entities[e];
            let card = entity.card;
            if !self.affordable(e) || !self.usable(entity) {
                continue;
            }
            for (which, req) in self.play_options(entity) {
                let targets = self.targets(req, self.current);
                match card.card_type {
                    CardType::Minion => {
                        if me.board.len() >= BOARD_LIMIT {
                            continue;
                        }
                        // 戰吼有合法目標時一定要選；沒有就照常出、戰吼不發動
                        let choices: Vec<Option<Target>> = if targets.is_empty() {
                            vec![None]
                        } else {
                            targets.into_iter().map(Some).collect()
                        };
                        for pos in 0..=me.board.len() {
                            for &t in &choices {
                                out.push(play_action(i, Some(pos), t, which));
                            }
                        }
                    }
                    CardType::Weapon | CardType::Spell => {
                        if req == TargetReq::None {
                            out.push(play_action(i, None, None, which));
                        } else {
                            for t in targets {
                                out.push(play_action(i, None, Some(t), which));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let attackers = (0..me.board.len())
            .map(|index| Target::Minion {
                player: self.current,
                index,
            })
            .chain(std::iter::once(Target::Hero {
                player: self.current,
            }));
        for attacker in attackers {
            if !self.can_attack(attacker) {
                continue;
            }
            for target in self.attack_targets(attacker) {
                out.push(Action::Attack { attacker, target });
            }
        }
        if let Some(req) = self.power_req() {
            if req == TargetReq::None {
                out.push(Action::Power { target: None });
            } else {
                for t in self.targets(req, self.current) {
                    out.push(Action::Power { target: Some(t) });
                }
            }
        }
        out.push(Action::End);
        out
    }

    /// 執行行動；不合法時狀態不變。
    pub fn act(&mut self, action: &Action) -> Result<(), ActError> {
        if self.outcome.is_some() {
            return Err(ActError::GameOver);
        }
        if let Some(d) = &self.desync {
            return Err(ActError::Desync(d.clone()));
        }
        if !self.is_legal(action) {
            return Err(ActError::Illegal);
        }
        match *action {
            Action::Play {
                hand,
                pos,
                target,
                choose_one,
            } => {
                let target = target.and_then(|t| self.resolve(t));
                self.play(hand, pos, target, choose_one)
            }
            Action::Attack { attacker, target } => self.attack(attacker, target),
            Action::Power { target } => {
                let target = target.and_then(|t| self.resolve(t));
                self.use_power(target)
            }
            Action::End => self.end_turn(),
            Action::Choose { index } => self.choose(index),
        }
        match &self.desync {
            Some(d) => Err(ActError::Desync(d.clone())),
            None => Ok(()),
        }
    }

    // ---- 合法性 ----

    fn affordable(&self, id: EntityId) -> bool {
        // 超載可以讓可用法力變成負的，但 0 費的牌一律能打出
        let cost = self.hand_cost(id);
        cost == 0 || cost <= self.players[self.current].mana
    }

    fn usable(&self, entity: &Entity) -> bool {
        if entity.card.keywords.contains(&Keyword::Secret) {
            let secrets = &self.players[entity.owner].secrets;
            if secrets.len() >= MAX_SECRETS
                || secrets
                    .iter()
                    .any(|&s| self.entities[s].card.id == entity.card.id)
            {
                return false;
            }
        }
        match entity.def().map_or(Usable::Always, |d| d.usable) {
            Usable::Always => true,
            Usable::BoardSlot => self.players[entity.owner].board.len() < BOARD_LIMIT,
            Usable::WeaponEquipped => self.players[entity.owner].weapon.is_some(),
        }
    }

    /// 英雄技能可以使用時回傳它的目標條件。
    fn power_req(&self) -> Option<TargetReq> {
        let me = &self.players[self.current];
        let power = &self.entities[me.hero_power];
        let def = power.def()?;
        if me.power_used || !self.affordable(me.hero_power) || !self.usable(power) {
            return None;
        }
        if def.target != TargetReq::None && self.targets(def.target, self.current).is_empty() {
            return None;
        }
        Some(def.target)
    }

    /// 符合目標條件的角色（player 為使用者）。對手的潛行手下不能選。
    fn targets(&self, req: TargetReq, player: usize) -> Vec<Target> {
        // 手下的受傷條件：None 不限、Some(true) 只限受傷、Some(false) 只限未受傷
        let mut need_damaged: Option<bool> = None;
        let (heroes, minions, friendly, enemy) = match req {
            TargetReq::None => return Vec::new(),
            TargetReq::Character => (true, true, true, true),
            TargetReq::Minion => (false, true, true, true),
            TargetReq::FriendlyMinion => (false, true, true, false),
            TargetReq::EnemyMinion => (false, true, false, true),
            TargetReq::FriendlyCharacter => (true, true, true, false),
            TargetReq::EnemyCharacter => (true, true, false, true),
            TargetReq::UndamagedMinion => {
                need_damaged = Some(false);
                (false, true, true, true)
            }
            TargetReq::DamagedEnemyMinion => {
                need_damaged = Some(true);
                (false, true, false, true)
            }
            TargetReq::CharacterIfWeapon => {
                if self.players[player].weapon.is_none() {
                    return Vec::new();
                }
                (true, true, true, true)
            }
        };
        let minion_ok = |m: EntityId| {
            let e = &self.entities[m];
            need_damaged.is_none_or(|d| d == (e.health < e.max_health))
        };
        let mut out = Vec::new();
        for side in [player, 1 - player] {
            let mine = side == player;
            if (mine && !friendly) || (!mine && !enemy) {
                continue;
            }
            if heroes {
                out.push(Target::Hero { player: side });
            }
            if minions {
                for (index, &m) in self.players[side].board.iter().enumerate() {
                    // 休眠中的手下不能被指定（第 3b 階段 B3）
                    if (mine || !self.entities[m].stealth)
                        && self.entities[m].dormant == 0
                        && minion_ok(m)
                    {
                        out.push(Target::Minion {
                            player: side,
                            index,
                        });
                    }
                }
            }
        }
        out
    }

    fn is_legal(&self, action: &Action) -> bool {
        if let Some((_, options)) = &self.choice {
            return matches!(*action, Action::Choose { index } if index < options.len());
        }
        let me = &self.players[self.current];
        match *action {
            Action::Play {
                hand,
                pos,
                target,
                choose_one,
            } => {
                let Some(&e) = me.hand.get(hand) else {
                    return false;
                };
                let entity = &self.entities[e];
                let card = entity.card;
                if !self.affordable(e) || !self.usable(entity) {
                    return false;
                }
                let Some(&(_, req)) = self
                    .play_options(entity)
                    .iter()
                    .find(|(w, _)| *w == choose_one)
                else {
                    return false;
                };
                let targets = self.targets(req, self.current);
                let target_ok = match target {
                    None => req == TargetReq::None || targets.is_empty(),
                    Some(t) => targets.contains(&t),
                };
                match (card.card_type, pos) {
                    (CardType::Minion, Some(p)) => {
                        target_ok && me.board.len() < BOARD_LIMIT && p <= me.board.len()
                    }
                    (CardType::Weapon | CardType::Spell, None) => {
                        // 法術、武器需要目標時一定要有
                        target_ok && (req == TargetReq::None || target.is_some())
                    }
                    _ => false,
                }
            }
            Action::Attack { attacker, target } => {
                self.owner_of(attacker) == Some(self.current)
                    && self.can_attack(attacker)
                    && self.attack_targets(attacker).contains(&target)
            }
            Action::Power { target } => match self.power_req() {
                None => false,
                Some(TargetReq::None) => target.is_none(),
                Some(req) => target.is_some_and(|t| self.targets(req, self.current).contains(&t)),
            },
            Action::End => true,
            Action::Choose { .. } => false,
        }
    }

    /// 角色的擁有者；位置不存在時為 None。
    fn owner_of(&self, t: Target) -> Option<usize> {
        self.resolve(t).map(|e| self.entities[e].owner)
    }

    fn resolve(&self, t: Target) -> Option<EntityId> {
        match t {
            Target::Hero { player } => self.players.get(player).map(|p| p.hero),
            Target::Minion { player, index } => self.players.get(player)?.board.get(index).copied(),
        }
    }

    fn can_attack(&self, attacker: Target) -> bool {
        let Some(id) = self.resolve(attacker) else {
            return false;
        };
        let e = &self.entities[id];
        if e.owner != self.current || e.frozen || e.dormant > 0 {
            return false;
        }
        let (attack, windfury) = match attacker {
            Target::Hero { player } => {
                let weapon_windfury = self.players[player]
                    .weapon
                    .is_some_and(|w| self.entities[w].windfury);
                (self.hero_attack(player), e.windfury || weapon_windfury)
            }
            Target::Minion { .. } => {
                if e.summoning_sick && !e.charge && !e.rush {
                    return false;
                }
                (e.attack, e.windfury)
            }
        };
        attack > 0 && e.used_attacks() < Self::max_attacks(windfury)
    }

    /// 每回合可攻擊次數。
    fn max_attacks(windfury: bool) -> i32 {
        if windfury { 2 } else { 1 }
    }

    /// 攻擊者可以選的目標：有嘲諷（非潛行）就只能打嘲諷；潛行的不能選；剛上場的衝刺不能打英雄。
    fn attack_targets(&self, attacker: Target) -> Vec<Target> {
        let Some(id) = self.resolve(attacker) else {
            return Vec::new();
        };
        let a = &self.entities[id];
        let opp = 1 - a.owner;
        let visible: Vec<(usize, &Entity)> = self.players[opp]
            .board
            .iter()
            .enumerate()
            .map(|(i, &m)| (i, &self.entities[m]))
            .filter(|(_, m)| !m.stealth && m.dormant == 0)
            .collect();
        let has_taunt = visible.iter().any(|(_, m)| m.taunt);
        let mut out: Vec<Target> = visible
            .iter()
            .filter(|(_, m)| !has_taunt || m.taunt)
            .map(|&(index, _)| Target::Minion { player: opp, index })
            .collect();
        let rush_only = matches!(attacker, Target::Minion { .. }) && a.summoning_sick && !a.charge;
        if !has_taunt && !rush_only {
            out.push(Target::Hero { player: opp });
        }
        out
    }

    // ---- 行動 ----

    fn play(&mut self, hand: usize, pos: Option<usize>, target: Option<EntityId>, choose_one: u8) {
        let p = self.current;
        // 流放：看打出當下的位置（第 3b 階段 A1）
        let outcast = hand == 0 || hand + 1 == self.players[p].hand.len();
        let cost = self.hand_cost(self.players[p].hand[hand]);
        let e = self.players[p].hand.remove(hand);
        let card = self.entities[e].card;
        self.players[p].mana -= cost;
        if card.card_type != CardType::Spell {
            self.players[p].overload_owed += card.overload;
        }
        if card.card_type == CardType::Spell {
            // 打出任何法術就用掉準備（含會被反制的；第 3b 階段 A4）
            self.players[p].spell_discount = 0;
            self.players[p].spells_this_turn.push(card.id);
        }
        let ctx = Ctx {
            source: e,
            owner: p,
            target,
            spell: card.card_type == CardType::Spell,
            deathrattle: false,
        };
        match card.card_type {
            CardType::Minion => {
                let pos = pos.expect("已檢查手下有位置");
                self.entities[e].summoning_sick = true;
                self.entities[e].dormant = self.entities[e].def().map_or(0, |d| d.dormant);
                self.enter_board(p, e, pos);
            }
            CardType::Weapon => self.equip_entity(p, e),
            _ => {}
        }
        if card.card_type == CardType::Spell {
            // 打出時、結算與反制前（博學行者阿洲；第 3b 階段 D2、D3）
            self.fire_spell_played(e);
            // 對手的反制秘密
            self.countered = false;
            self.reveal_secrets(1 - p, &[When::OpponentCastsSpell], e);
            if self.countered {
                self.countered = false;
                self.death_phase();
                return;
            }
            // 被反制的法術不套用超載（第 3a 階段 H3）
            self.players[p].overload_owed += card.overload;
            if card.keywords.contains(&Keyword::Secret) {
                self.players[p].secrets.push(e);
                self.death_phase();
                // 打出秘密也是施放法術：對手「施放法術後」的秘密照樣觸發（第 3a 階段 E2）
                self.reveal_secrets(1 - p, &[When::AfterOpponentCastsSpell], e);
                self.death_phase();
                return;
            }
        }
        let effect = self.entities[e].def().and_then(|d| match d.choose {
            Some(c) if choose_one > 0 => Some(c[usize::from(choose_one) - 1].effect),
            _ => d.play,
        });
        if let Some(effect) = effect {
            self.run(&effect, &ctx);
        }
        if outcast && let Some(effect) = self.entities[e].def().and_then(|d| d.outcast) {
            self.run(&effect, &ctx);
        }
        if self.choice.is_some() {
            // 選擇畫面：之後的流程等選完才結算（第 3b 階段 C3、C3b）
            self.pending = Some(e);
            return;
        }
        self.finish_play(e);
    }

    /// 出牌效果結算後的流程：死亡階段、「召喚後」觸發與對手的秘密。
    fn finish_play(&mut self, e: EntityId) {
        let card = self.entities[e].card;
        let p = self.entities[e].owner;
        if card.card_type == CardType::Minion {
            // 戰吼結算後先進入死亡階段，再做「召喚後」觸發與對手「打出手下後」的秘密（case C1、C6）；
            // 英雄在這個死亡階段死亡時流程照樣繼續，最後的死亡階段才定勝負（case C7）
            self.death_phase();
            // 以休眠上場不觸發（第 3b 階段 B5）
            if self.entities[e].dormant == 0 {
                self.fire_summon(e);
                self.fire_after_play(e);
                self.reveal_secrets(1 - p, &[When::AfterOpponentPlaysMinion], e);
            }
        }
        if card.card_type == CardType::Spell {
            // 法術結算完、死亡階段之後，對手「施放法術後」的秘密（第 3a 階段 E2）
            self.death_phase();
            self.reveal_secrets(1 - p, &[When::AfterOpponentCastsSpell], e);
        }
        self.death_phase();
    }

    /// 揭露並觸發 player 秘密區中條件在 whens 裡的秘密（不分條件，依打出順序）；target 是觸發者。
    /// 觸發者已死亡或離開戰場時，之後作用在觸發者上的秘密不再觸發；「英雄被攻擊」「手下被攻擊」照樣觸發
    /// （條件看被攻擊的一方；第 2 階段 I3a、第 3b 階段 H4）。
    fn reveal_secrets(&mut self, player: usize, whens: &[When], target: EntityId) {
        // 同一次觸發的秘密都結算完才更新被動加成（case I3d）
        let outer = std::mem::replace(&mut self.defer_auras, true);
        let mut i = 0;
        while i < self.players[player].secrets.len() {
            let s = self.players[player].secrets[i];
            let matches = self.entities[s]
                .def()
                .and_then(|d| d.secret)
                .filter(|t| whens.contains(&t.when));
            let gone = {
                let t = &self.entities[target];
                let on_board = self.players[t.owner].board.contains(&target)
                    || self.players[t.owner].hero == target;
                (t.health <= 0 || t.destroyed || !on_board)
                    && self.entities[target].card.card_type != CardType::Spell
            };
            match matches {
                Some(trigger)
                    if (!gone
                        || matches!(
                            trigger.when,
                            When::OwnHeroAttacked | When::FriendlyMinionAttacked
                        ))
                        && self.secret_can_fire(&trigger.effect, player) =>
                {
                    self.players[player].secrets.remove(i);
                    let ctx = Ctx {
                        source: s,
                        owner: player,
                        target: Some(target),
                        spell: true,
                        deathrattle: false,
                    };
                    self.run(&trigger.effect, &ctx);
                }
                _ => i += 1,
            }
        }
        self.defer_auras = outer;
        self.refresh_auras();
    }

    /// 秘密的效果能不能發揮；不能時秘密不觸發、留在場上（Wiki Secret；第 3a 階段 E1、E2）。
    fn secret_can_fire(&self, effect: &Effect, owner: usize) -> bool {
        if effect.summons() && self.players[owner].board.len() >= BOARD_LIMIT {
            return false;
        }
        if matches!(effect, Effect::DestroyRandomEnemyMinion) {
            return !self.living_minions(1 - owner).is_empty();
        }
        true
    }

    /// 一方戰場上還活著（生命 > 0、沒有被消滅）的手下。
    fn living_minions(&self, player: usize) -> Vec<EntityId> {
        self.players[player]
            .board
            .iter()
            .copied()
            .filter(|&m| {
                let e = &self.entities[m];
                e.health > 0 && !e.destroyed && e.dormant == 0
            })
            .collect()
    }

    /// 沉默：效果、觸發、被動加成、死亡之聲失效；數值回到卡牌原始值（加上別人給的被動加成）；
    /// 生命上限變高時目前生命增加同樣的量，變低時只在超過新上限時才降；解除凍結、拿掉關鍵字。
    fn silence(&mut self, id: EntityId) {
        let e = &mut self.entities[id];
        e.silenced = true;
        e.attack = e.card.attack.unwrap_or(0) + e.aura_attack;
        e.temp_attack = 0;
        let max = e.card.health.unwrap_or(0) + e.aura_health;
        if max > e.max_health {
            // 上限變高：目前生命增加同樣的量（第 3a 階段 H2）
            e.health += max - e.max_health;
        } else {
            e.health = e.health.min(max);
        }
        e.max_health = max;
        e.taunt = false;
        e.divine_shield = false;
        e.stealth = false;
        e.windfury = false;
        e.charge = false;
        e.rush = false;
        e.poisonous = false;
        e.lifesteal = false;
        e.freeze = false;
        e.frozen = false;
        e.spell_damage = 0;
        self.refresh_auras();
    }

    /// 手下回到擁有者的手牌（回到卡牌原始狀態，費用 +cost）；手牌滿了就消滅。
    fn return_to_hand(&mut self, id: EntityId, cost: i32) {
        let owner = self.entities[id].owner;
        if self.players[owner].hand.len() >= HAND_LIMIT {
            self.entities[id].destroyed = true;
            return;
        }
        let Some(pos) = self.players[owner].board.iter().position(|&m| m == id) else {
            return;
        };
        self.players[owner].board.remove(pos);
        let card = self.entities[id].card;
        let mut e = Entity::new(card, self.def_of(card.id), owner);
        // 之前的費用調整在打出時就失效，回到手牌只算這次的
        e.cost_mod = cost;
        self.entities[id] = e;
        self.players[owner].hand.push(id);
        self.refresh_auras();
    }

    /// 對角色造成傷害（卡片效果函式用），回傳實際扣掉的生命。
    pub fn deal_damage(&mut self, target: EntityId, amount: i32, source_owner: usize) -> i32 {
        self.damage(target, amount, false, source_owner)
    }

    /// 選擇畫面選第 index 個：選的牌放進手牌（手牌滿了摧毀），再結算暫停的出牌流程。
    fn choose(&mut self, index: usize) {
        let (p, options) = self.choice.take().expect("已檢查有選擇畫面");
        let picked = options[index];
        let player = &mut self.players[p];
        player.deck.retain(|&c| c != picked);
        if player.hand.len() < HAND_LIMIT {
            player.hand.push(picked);
        }
        if let Some(e) = self.pending.take() {
            self.finish_play(e);
        }
    }

    fn use_power(&mut self, target: Option<EntityId>) {
        let p = self.current;
        let power = self.players[p].hero_power;
        let cost = self.entities[power].card.cost.unwrap_or(0);
        self.players[p].mana -= cost;
        self.players[p].power_used = true;
        let ctx = Ctx {
            source: power,
            owner: p,
            target,
            spell: false,
            deathrattle: false,
        };
        if let Some(effect) = self.entities[power].def().and_then(|d| d.play) {
            self.run(&effect, &ctx);
        }
        self.death_phase();
    }

    fn equip_entity(&mut self, player: usize, weapon: EntityId) {
        self.players[player].weapon = Some(weapon);
    }

    fn attack(&mut self, attacker: Target, target: Target) {
        let a = self.resolve(attacker).expect("已檢查攻擊者");
        let t = self.resolve(target).expect("已檢查目標");
        let hero_attacking = matches!(attacker, Target::Hero { .. });
        self.attack_entity(a, t, hero_attacking, true);
        if hero_attacking {
            self.fire_after_hero_attack(t);
        }
    }

    /// 效果要求的攻擊（伊利達瑞審判官、戰槌挑戰者）：不檢查能不能攻擊、不算攻擊次數，照常觸發秘密。
    pub fn forced_attack(&mut self, attacker: EntityId, target: EntityId) {
        if self.in_play(attacker) && self.in_play(target) && self.outcome.is_none() {
            self.attack_entity(attacker, target, false, false);
        }
    }

    /// 「你的英雄攻擊敵人後」的觸發（依上場順序；目標已不在場上就不觸發）。
    fn fire_after_hero_attack(&mut self, target: EntityId) {
        let p = self.current;
        let mut holders = self.players[p].board.clone();
        holders.sort_by_key(|&m| self.entities[m].play_order);
        for m in holders {
            let Some(def) = self.entities[m].def() else {
                continue;
            };
            for t in def.triggers {
                if t.when == When::AfterOwnHeroAttacks && self.in_play(m) && self.in_play(target) {
                    let ctx = Ctx {
                        source: m,
                        owner: p,
                        target: Some(target),
                        spell: false,
                        deathrattle: false,
                    };
                    self.run(&t.effect, &ctx);
                }
            }
        }
    }

    /// 攻擊：a 攻擊 t（hero_attacking 時 a 是英雄、帶武器）；count 為 false 時不算攻擊次數。
    fn attack_entity(&mut self, a: EntityId, t: EntityId, hero_attacking: bool, count: bool) {
        let p = self.entities[a].owner;
        let target_is_hero = self.players[1 - p].hero == t;
        let weapon = if hero_attacking {
            self.players[p].weapon
        } else {
            None
        };
        let has = |g: &Game, f: fn(&Entity) -> bool| {
            f(&g.entities[a]) || weapon.is_some_and(|w| f(&g.entities[w]))
        };
        // 攻擊宣告：防守方的秘密依打出順序觸發；攻擊者或目標因此離場就取消攻擊
        let defender = 1 - p;
        let secrets_before = self.players[defender].secrets.len();
        let whens: &[When] = match (target_is_hero, hero_attacking) {
            (true, false) => &[When::OwnHeroAttacked, When::EnemyMinionAttacks],
            (true, true) => &[When::OwnHeroAttacked],
            (false, false) => &[When::EnemyMinionAttacks, When::FriendlyMinionAttacked],
            (false, true) => &[When::FriendlyMinionAttacked],
        };
        self.reveal_secrets(defender, whens, a);
        if self.players[defender].secrets.len() != secrets_before {
            self.death_phase();
        }
        let alive = |g: &Game, id: EntityId| {
            let e = &g.entities[id];
            e.health > 0
                && (g.players[e.owner].board.contains(&id) || g.players[e.owner].hero == id)
        };
        if self.outcome.is_some() || !alive(self, a) || !alive(self, t) {
            return;
        }
        // 戰鬥用秘密結算後當下的攻擊力；武器只在自己的回合有作用，所以被攻擊的英雄不反擊。
        let source_attack = if hero_attacking {
            self.hero_attack(p)
        } else {
            self.entities[a].attack
        };
        let target_attack = if target_is_hero {
            0
        } else {
            self.entities[t].attack
        };
        let a_lifesteal = has(self, |e| e.lifesteal);
        let a_poisonous = has(self, |e| e.poisonous);
        if let Some(w) = weapon {
            self.entities[w].durability -= 1;
        }

        let dealt = self.damage(t, source_attack, a_lifesteal, p);
        if dealt > 0 && !hero_attacking && self.entities[a].freeze {
            self.entities[t].frozen = true;
        }
        if dealt > 0 && a_poisonous && !target_is_hero {
            self.entities[t].destroyed = true;
        }

        if target_attack > 0 {
            let (t_lifesteal, t_poisonous, t_freeze) = {
                let e = &self.entities[t];
                (e.lifesteal, e.poisonous, e.freeze)
            };
            let back = self.damage(a, target_attack, t_lifesteal, 1 - p);
            if back > 0 {
                if t_freeze {
                    self.entities[a].frozen = true;
                }
                if t_poisonous && !hero_attacking {
                    self.entities[a].destroyed = true;
                }
            }
        }

        let e = &mut self.entities[a];
        e.stealth = false;
        e.attacks_this_turn += 1;
        if !count {
            e.forced_attacks += 1;
        }
        self.death_phase();
    }

    /// 對角色造成傷害，回傳實際扣掉的生命；聖盾術擋下為 0，英雄先扣護甲。
    fn damage(
        &mut self,
        target: EntityId,
        amount: i32,
        lifesteal: bool,
        source_owner: usize,
    ) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let is_hero = self.players.iter().any(|p| p.hero == target);
        let e = &mut self.entities[target];
        if !is_hero && e.divine_shield {
            e.divine_shield = false;
            return 0;
        }
        let mut amount = amount;
        if is_hero {
            let absorbed = e.armor.min(amount);
            e.armor -= absorbed;
            amount -= absorbed;
        }
        e.health -= amount;
        if lifesteal && amount > 0 {
            let hero = self.players[source_owner].hero;
            self.heal(hero, amount);
        }
        if amount > 0 && !is_hero {
            self.refresh_auras();
            self.fire_damaged();
        }
        amount
    }

    /// 恢復生命，不超過上限。
    fn heal(&mut self, target: EntityId, amount: i32) {
        let e = &mut self.entities[target];
        if amount > 0 && e.health < e.max_health {
            e.health = (e.health + amount).min(e.max_health);
            self.refresh_auras();
        }
    }

    // ---- 效果 ----

    fn select(&self, sel: Sel, ctx: &Ctx) -> Vec<EntityId> {
        let mut v = self.select_all(sel, ctx);
        // 休眠中的手下不受效果影響（第 3b 階段 B3）
        v.retain(|&m| self.entities[m].dormant == 0);
        v
    }

    fn select_all(&self, sel: Sel, ctx: &Ctx) -> Vec<EntityId> {
        let me = &self.players[ctx.owner];
        let opp = &self.players[1 - ctx.owner];
        match sel {
            Sel::Target => ctx.target.into_iter().collect(),
            Sel::Itself => vec![ctx.source],
            Sel::OwnHero => vec![me.hero],
            Sel::EnemyHero => vec![opp.hero],
            Sel::Adjacent => match me.board.iter().position(|&m| m == ctx.source) {
                Some(i) => {
                    let mut v = Vec::new();
                    if i > 0 {
                        v.push(me.board[i - 1]);
                    }
                    if let Some(&r) = me.board.get(i + 1) {
                        v.push(r);
                    }
                    v
                }
                None => Vec::new(),
            },
            Sel::FriendlyMinions => me.board.clone(),
            Sel::OtherFriendlyMinions { race } => me
                .board
                .iter()
                .copied()
                .filter(|&m| {
                    m != ctx.source && race.is_none_or(|r| self.entities[m].card.races.contains(&r))
                })
                .collect(),
            Sel::EnemyMinions => opp.board.clone(),
            Sel::AllMinions => me.board.iter().chain(&opp.board).copied().collect(),
            Sel::MinionsWithAttackAtLeast(n) => me
                .board
                .iter()
                .chain(&opp.board)
                .copied()
                .filter(|&m| self.entities[m].attack >= n)
                .collect(),
            Sel::AllCharacters => {
                let mut v = me.board.clone();
                v.push(me.hero);
                v.extend(opp.board.iter().copied());
                v.push(opp.hero);
                v
            }
            Sel::EnemyCharacters => {
                let mut v = opp.board.clone();
                v.push(opp.hero);
                v
            }
        }
    }

    /// 執行效果（不含死亡階段）。
    pub fn run(&mut self, effect: &Effect, ctx: &Ctx) {
        match *effect {
            Effect::Damage { to, amount } => {
                let bonus = if ctx.spell {
                    self.spell_damage(ctx.owner)
                } else {
                    0
                };
                let lifesteal = self.entities[ctx.source].lifesteal;
                for t in self.select(to, ctx) {
                    self.damage(t, amount + bonus, lifesteal, ctx.owner);
                }
            }
            Effect::Heal { to, amount } => {
                for t in self.select(to, ctx) {
                    self.heal(t, amount);
                }
            }
            Effect::Armor(n) => {
                let h = self.players[ctx.owner].hero;
                self.entities[h].armor += n;
            }
            Effect::HeroAttack(n) => {
                let h = self.players[ctx.owner].hero;
                self.entities[h].attack += n;
                self.entities[h].temp_attack += n;
            }
            Effect::Draw(n) => {
                for _ in 0..n {
                    self.draw(ctx.owner);
                }
            }
            Effect::TempMana(n) => self.players[ctx.owner].mana += n,
            Effect::Summon { card, count } => {
                for _ in 0..count {
                    let board = &self.players[ctx.owner].board;
                    if board.len() >= BOARD_LIMIT {
                        break;
                    }
                    let pos = if ctx.deathrattle {
                        let marker = self
                            .dr_markers
                            .iter()
                            .find(|m| m.0 == ctx.source)
                            .map_or(board.len(), |m| m.2);
                        marker.min(board.len())
                    } else {
                        match board.iter().position(|&m| m == ctx.source) {
                            Some(i) => i + 1,
                            None => board.len(),
                        }
                    };
                    let Some(summoned) = self.summon(ctx.owner, card, pos) else {
                        continue;
                    };
                    if ctx.deathrattle {
                        self.shift_markers(ctx.owner, pos);
                    }
                    self.double_summon(summoned);
                }
            }
            Effect::Equip(id) => {
                if let Some(card) = card_by_id(id) {
                    let w = self.new_entity(card, ctx.owner);
                    self.equip_entity(ctx.owner, w);
                }
            }
            Effect::Destroy(sel) => {
                for t in self.select(sel, ctx) {
                    self.entities[t].destroyed = true;
                }
            }
            Effect::Buff { to, attack, health } => {
                for t in self.select(to, ctx) {
                    let e = &mut self.entities[t];
                    e.attack += attack;
                    e.max_health += health;
                    e.health += health;
                }
            }
            Effect::TurnAttack { to, amount } => {
                for t in self.select(to, ctx) {
                    let e = &mut self.entities[t];
                    e.attack += amount;
                    e.temp_attack += amount;
                }
            }
            Effect::Taunt(sel) => {
                for t in self.select(sel, ctx) {
                    self.entities[t].taunt = true;
                }
            }
            Effect::DivineShield(sel) => {
                for t in self.select(sel, ctx) {
                    self.entities[t].divine_shield = true;
                }
            }
            Effect::Transform { to, card } => {
                for t in self.select(to, ctx) {
                    self.transform(t, card);
                }
            }
            Effect::DiscoverFromDeck => self.discover_from_deck(ctx.owner),
            Effect::Silence(sel) => {
                for t in self.select(sel, ctx) {
                    self.silence(t);
                }
            }
            Effect::ReturnToHand { to, cost } => {
                for t in self.select(to, ctx) {
                    self.return_to_hand(t, cost);
                }
            }
            Effect::Counter => self.countered = true,
            Effect::DiscoverSpell => self.discover_spell(ctx.owner),
            Effect::SpellDiscount(n) => self.players[ctx.owner].spell_discount = n,
            Effect::ShuffleIntoDeck { card, count } => {
                let Some(card) = card_by_id(card) else {
                    return;
                };
                for _ in 0..count {
                    let id = self.new_entity(card, ctx.owner);
                    // 同 RosettaStone：放進牌庫頂後整副重洗（RosettaStone 的牌庫順序和這裡相反）
                    let mut rs: Vec<EntityId> =
                        self.players[ctx.owner].deck.iter().rev().copied().collect();
                    rs.push(id);
                    let rs = self.shuffled(rs);
                    self.players[ctx.owner].deck = rs.into_iter().rev().collect();
                }
            }
            Effect::ReturnLastTurnSpells => {
                let mut list = self.players[ctx.owner].spells_last_turn.clone();
                if list.len() > 1 {
                    match self.rng.shuffle(list.len()) {
                        Ok(perm) => list = perm.iter().map(|&i| list[i]).collect(),
                        Err(d) => {
                            self.desync.get_or_insert(d);
                        }
                    }
                }
                for id in list {
                    if self.add_to_hand(ctx.owner, id).is_none() {
                        break;
                    }
                }
            }
            Effect::SummonCopy => {
                let Some(t) = ctx.target else {
                    return;
                };
                let owner = self.entities[t].owner;
                let board = &self.players[owner].board;
                if board.len() >= BOARD_LIMIT {
                    return;
                }
                let Some(i) = board.iter().position(|&m| m == t) else {
                    return;
                };
                let pos = i + 1;
                let card = self.entities[t].card.id;
                if self.summon(owner, card, pos).is_some() {
                    self.shift_markers(owner, pos);
                }
            }
            Effect::RandomSplitEnemyMinions(n) => {
                // 法術傷害增加次數，每次仍 1 點
                let n = if ctx.spell {
                    n + self.spell_damage(ctx.owner)
                } else {
                    n
                };
                let lifesteal = self.entities[ctx.source].lifesteal;
                for _ in 0..n {
                    let candidates = self.living_minions(1 - ctx.owner);
                    if candidates.is_empty() {
                        break;
                    }
                    let i = self.random_int(0, candidates.len() as i64 - 1) as usize;
                    self.damage(candidates[i], 1, lifesteal, ctx.owner);
                }
            }
            Effect::Freeze(sel) => {
                for t in self.select(sel, ctx) {
                    self.entities[t].frozen = true;
                }
            }
            Effect::IfHandEmpty(e) => {
                if self.players[ctx.owner].hand.is_empty() {
                    self.run(e, ctx);
                }
            }
            Effect::IfTargetSurvived(e) => {
                if ctx
                    .target
                    .is_some_and(|t| self.entities[t].health > 0 && !self.entities[t].destroyed)
                {
                    self.run(e, ctx);
                }
            }
            Effect::DrawReduceCost(n) => {
                // 只減抽到的那張；它是抽中時施放的牌（已施放）或被燒掉時就落空（第 3b 階段 H1）
                let drawn = self.players[ctx.owner].deck.first().copied();
                self.draw(ctx.owner);
                if let Some(c) = drawn.filter(|c| self.players[ctx.owner].hand.contains(c)) {
                    self.entities[c].cost_mod -= n;
                }
            }
            Effect::SummonRandom(list) => {
                if self.players[ctx.owner].board.len() < BOARD_LIMIT && !list.is_empty() {
                    let i = self.random_int(0, list.len() as i64 - 1) as usize;
                    let summon = Effect::Summon {
                        card: list[i],
                        count: 1,
                    };
                    self.run(&summon, ctx);
                }
            }
            Effect::SetHealth { to, value } => {
                for t in self.select(to, ctx) {
                    let e = &mut self.entities[t];
                    e.max_health = value + e.aura_health;
                    e.health = e.max_health;
                }
                self.refresh_auras();
            }
            Effect::SwapAttackHealth(sel) => {
                for t in self.select(sel, ctx) {
                    let e = &mut self.entities[t];
                    let (attack, health) = (e.attack, e.health);
                    e.attack = health + e.aura_attack;
                    e.temp_attack = 0;
                    e.max_health = attack + e.aura_health;
                    e.health = e.max_health;
                }
                self.refresh_auras();
            }
            Effect::AddToHand(id) => {
                self.add_to_hand(ctx.owner, id);
            }
            Effect::WeaponAttack(n) => {
                if let Some(w) = self.players[ctx.owner].weapon {
                    self.entities[w].attack += n;
                }
            }
            Effect::DestroyRandomEnemyMinion => {
                let candidates = self.living_minions(1 - ctx.owner);
                if !candidates.is_empty() {
                    let i = self.random_int(0, candidates.len() as i64 - 1) as usize;
                    self.entities[candidates[i]].destroyed = true;
                }
            }
            Effect::ReturnRandomEnemyMinion => {
                let candidates = self.living_minions(1 - ctx.owner);
                if !candidates.is_empty() {
                    let i = self.random_int(0, candidates.len() as i64 - 1) as usize;
                    self.return_to_hand(candidates[i], 0);
                }
            }
            Effect::AttackTarget => {
                if let Some(t) = ctx.target {
                    self.forced_attack(ctx.source, t);
                }
            }
            Effect::RandomSplitDamage(n) => {
                for _ in 0..n {
                    let me = &self.players[ctx.owner];
                    let opp = &self.players[1 - ctx.owner];
                    let mut candidates: Vec<EntityId> = me
                        .board
                        .iter()
                        .copied()
                        .filter(|&m| m != ctx.source)
                        .collect();
                    candidates.push(me.hero);
                    candidates.extend(opp.board.iter().copied());
                    candidates.push(opp.hero);
                    // 前幾發已經打死（生命 ≤ 0）的角色不再挑（case F4a）
                    candidates.retain(|&c| {
                        let e = &self.entities[c];
                        e.health > 0 && !e.destroyed && e.dormant == 0
                    });
                    if candidates.is_empty() {
                        break;
                    }
                    let i = self.random_int(0, candidates.len() as i64 - 1) as usize;
                    self.damage(candidates[i], 1, false, ctx.owner);
                }
            }
            Effect::Seq(effects) => {
                for e in effects {
                    self.run(e, ctx);
                }
            }
            Effect::Custom(f) => f(self, ctx),
        }
    }

    /// 從牌庫發現：抽法同 RosettaStone——牌庫由底到頂排列、依 dbfId 穩定排序、同一張卡只留第一張，
    /// 洗牌一次；超過 3 張再洗一次後取前 3 張。
    fn discover_from_deck(&mut self, p: usize) {
        let mut list: Vec<EntityId> = self.players[p].deck.iter().rev().copied().collect();
        let key = |card: &'static Card| self.discover_order.0.map_or(card.dbf_id, |f| f(card));
        list.sort_by_key(|&c| key(self.entities[c].card));
        list.dedup_by_key(|c| self.entities[*c].card.dbf_id);
        if list.is_empty() {
            return;
        }
        list = self.shuffled(list);
        if list.len() > 3 {
            list = self.shuffled(list);
            list.truncate(3);
        }
        self.choice = Some((p, list));
    }

    /// 照亂數洗牌（洗牌後第 i 個放原本的第 perm[i] 個）；不同步時記下並原樣回傳。
    fn shuffled(&mut self, list: Vec<EntityId>) -> Vec<EntityId> {
        match self.rng.shuffle(list.len()) {
            Ok(perm) => perm.iter().map(|&i| list[i]).collect(),
            Err(d) => {
                self.desync.get_or_insert(d);
                list
            }
        }
    }

    /// 死亡之聲結算中召喚了一隻到 owner 戰場第 pos 格：還沒結算的死亡手下中位置在插入點或右邊的往右推一格。
    fn shift_markers(&mut self, owner: usize, pos: usize) {
        for m in &mut self.dr_markers {
            if m.1 == owner && m.2 >= pos {
                m.2 += 1;
            }
        }
    }

    /// 發現一個法術：候選是你職業的法術加中立法術（差分測試用指定清單），去掉重複；
    /// 抽法同 RosettaStone：超過 3 張時洗牌一次取前 3 張，否則全部照順序列出。
    fn discover_spell(&mut self, p: usize) {
        let class = self.entities[self.players[p].hero].card.class;
        let ok = |c: &Card| {
            c.card_type == CardType::Spell
                && c.collectible
                && (c.class == class || c.class == Some(CardClass::Neutral))
        };
        let mut cards: Vec<&'static Card> = Vec::new();
        let mut push = |c: &'static Card| {
            if ok(c) && !cards.iter().any(|x| x.id == c.id) {
                cards.push(c);
            }
        };
        match self.random_pool.0 {
            Some(ids) => ids
                .iter()
                .filter_map(|id| card_by_id(id))
                .for_each(&mut push),
            None => all_cards()
                .iter()
                .filter(|c| is_standard_set(c.set))
                .for_each(&mut push),
        }
        if cards.is_empty() {
            return;
        }
        if cards.len() > 3 {
            match self.rng.shuffle(cards.len()) {
                Ok(perm) => cards = perm.iter().map(|&i| cards[i]).collect(),
                Err(d) => {
                    self.desync.get_or_insert(d);
                }
            }
            cards.truncate(3);
        }
        let options = cards.iter().map(|&c| self.new_entity(c, p)).collect();
        self.choice = Some((p, options));
    }

    /// 死亡階段：移除耐久度歸零的武器；死亡的手下依上場順序逐一移出戰場（記下移除時的位置），
    /// 再依上場順序結算死亡之聲；有新的死亡就重複。最後判斷勝負。
    fn death_phase(&mut self) {
        loop {
            for p in 0..2 {
                if let Some(w) = self.players[p].weapon
                    && self.entities[w].durability <= 0
                {
                    self.players[p].weapon = None;
                }
            }
            let mut dead: Vec<EntityId> = self
                .players
                .iter()
                .flat_map(|p| p.board.iter().copied())
                .filter(|&m| self.entities[m].health <= 0 || self.entities[m].destroyed)
                .collect();
            if dead.is_empty() {
                break;
            }
            dead.sort_by_key(|&m| self.entities[m].play_order);
            self.dr_markers.clear();
            // 死亡的被動加成來源：加成留在當時拿到的手下身上，死亡之聲都結算完才移除
            let mut lingering = Vec::new();
            for &d in &dead {
                let board = &self.players[self.entities[d].owner].board;
                let j = board
                    .iter()
                    .position(|&m| m == d)
                    .expect("死亡的手下在戰場上");
                for i in 0..board.len() {
                    if dead.contains(&board[i]) {
                        continue;
                    }
                    let (a, h) = self.aura_bonus(board, j, i);
                    if a != 0 || h != 0 {
                        lingering.push((board[i], a, h));
                    }
                }
            }
            self.lingering.extend(lingering);
            for &d in &dead {
                let owner = self.entities[d].owner;
                let board = &mut self.players[owner].board;
                let pos = board
                    .iter()
                    .position(|&m| m == d)
                    .expect("死亡的手下在戰場上");
                board.remove(pos);
                self.dr_markers.push((d, owner, pos));
            }
            self.refresh_auras();
            for &d in &dead {
                if let Some(effect) = self.entities[d].def().and_then(|def| def.deathrattle) {
                    let ctx = Ctx {
                        source: d,
                        owner: self.entities[d].owner,
                        target: None,
                        spell: false,
                        deathrattle: true,
                    };
                    self.run(&effect, &ctx);
                }
            }
            self.dr_markers.clear();
        }
        if !self.lingering.is_empty() {
            self.lingering.clear();
            self.refresh_auras();
        }
        let dead = [0, 1].map(|p| self.entities[self.players[p].hero].health <= 0);
        self.outcome = match dead {
            [true, true] => Some(Outcome::Draw),
            [true, false] => Some(Outcome::Won(1)),
            [false, true] => Some(Outcome::Won(0)),
            [false, false] => None,
        };
    }

    // ---- 回合 ----

    /// 抽一張牌：牌庫空了受疲勞傷害；手牌滿了摧毀；「抽中時施放」的牌施放後補抽。
    fn draw(&mut self, p: usize) {
        let player = &mut self.players[p];
        if player.deck.is_empty() {
            player.fatigue += 1;
            let amount = player.fatigue;
            let hero = player.hero;
            self.damage(hero, amount, false, p);
            return;
        }
        let card = player.deck.remove(0);
        // 手牌滿了燒掉（抽中時施放的也不施放、不補抽；第 3b 階段 F3）
        if self.players[p].hand.len() >= HAND_LIMIT {
            return;
        }
        if let Some(effect) = self.entities[card].def().and_then(|d| d.when_drawn) {
            let ctx = Ctx {
                source: card,
                owner: p,
                target: None,
                spell: true,
                deathrattle: false,
            };
            self.run(&effect, &ctx);
            self.draw(p);
            // 阿拉巴斯特：補抽的那張先複製，再複製抽中時施放的牌（第 3b 階段 D6）
            self.fire_drawn(p, card);
            return;
        }
        self.players[p].hand.push(card);
        self.fire_drawn(p, card);
    }

    fn start_turn(&mut self) {
        let p = self.current;
        // 「在對手的回合」等依目前玩家而定的被動加成
        self.refresh_auras();
        for q in 0..2 {
            let hero = self.players[q].hero;
            self.entities[hero].attacks_this_turn = 0;
            self.entities[hero].forced_attacks = 0;
            for &m in &self.players[q].board {
                self.entities[m].attacks_this_turn = 0;
                self.entities[m].forced_attacks = 0;
            }
        }
        for &m in &self.players[p].board {
            self.entities[m].summoning_sick = false;
        }
        // 休眠倒數；甦醒時視同剛上場（第 3b 階段 B2、B4、B6：不算召喚）
        let mut woke = false;
        for i in 0..self.players[p].board.len() {
            let m = self.players[p].board[i];
            if self.entities[m].dormant > 0 {
                self.entities[m].dormant -= 1;
                if self.entities[m].dormant == 0 {
                    self.entities[m].summoning_sick = true;
                    woke = true;
                }
            }
        }
        if woke {
            self.refresh_auras();
        }
        let me = &mut self.players[p];
        me.spells_last_turn = std::mem::take(&mut me.spells_this_turn);
        me.max_mana = (me.max_mana + 1).min(MANA_LIMIT);
        me.overload_locked = me.overload_owed;
        me.overload_owed = 0;
        me.mana = me.max_mana - me.overload_locked;
        me.power_used = false;
        self.draw(p);
        self.death_phase();
    }

    fn end_turn(&mut self) {
        let p = self.current;
        // 回合結束時的觸發（依上場順序）
        let mut holders = self.players[p].board.clone();
        holders.sort_by_key(|&m| self.entities[m].play_order);
        for m in holders {
            let Some(def) = self.entities[m].def() else {
                continue;
            };
            for t in def.triggers {
                if t.when == When::OwnTurnEnd {
                    let ctx = Ctx {
                        source: m,
                        owner: p,
                        target: None,
                        spell: false,
                        deathrattle: false,
                    };
                    self.run(&t.effect, &ctx);
                }
            }
        }
        self.death_phase();
        if self.outcome.is_some() {
            return;
        }
        // 本回合攻擊次數沒用完（也不是剛上場不能攻擊）的凍結角色解凍；用完才被凍結的要等下個自己的回合。
        // 剛上場的衝刺手下看回合結束當下對手有沒有手下（第 3b 階段 B4b）。
        let rush_targets = self.has_active_minions(1 - p);
        let hero = self.players[p].hero;
        let weapon_windfury = self.players[p]
            .weapon
            .is_some_and(|w| self.entities[w].windfury);
        let h = &mut self.entities[hero];
        if h.frozen && h.used_attacks() < Self::max_attacks(h.windfury || weapon_windfury) {
            h.frozen = false;
        }
        for &m in &self.players[p].board {
            let e = &mut self.entities[m];
            let could_attack = !e.summoning_sick || e.charge || (e.rush && rush_targets);
            if e.frozen && e.used_attacks() < Self::max_attacks(e.windfury) && could_attack {
                e.frozen = false;
            }
        }
        // 「本回合」的加成失效
        for e in &mut self.entities {
            if e.temp_attack != 0 {
                e.attack -= e.temp_attack;
                e.temp_attack = 0;
            }
        }
        // 準備的減費只到回合結束
        self.players[p].spell_discount = 0;
        self.current = 1 - p;
        self.turn += 1;
        self.start_turn();
    }
}

fn play_action(hand: usize, pos: Option<usize>, target: Option<Target>, choose_one: u8) -> Action {
    Action::Play {
        hand,
        pos,
        target,
        choose_one,
    }
}
