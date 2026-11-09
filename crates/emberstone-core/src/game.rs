//! 對局狀態、回合流程、出牌、戰鬥與合法行動。
//!
//! 第 1 階段：數值與關鍵字。第 2 階段：卡片效果（英雄技能、指定目標、法術、戰吼……），
//! 效果定義由 `emberstone-cards` 透過 `GameConfig::effects` 提供。

use emberstone_data::{Card, CardType, Keyword, card_by_id};

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
            temp_attack: 0,
            destroyed: false,
            play_order: 0,
            aura_attack: 0,
            aura_health: 0,
            silenced: false,
            cost_mod: 0,
        }
    }

    fn def(&self) -> Option<&'static CardDef> {
        if self.silenced {
            None
        } else {
            self.def.map(|d| d.0)
        }
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
            next_order: 1,
            dr_markers: Vec::new(),
            lingering: Vec::new(),
            defer_auras: false,
            choice: None,
            countered: false,
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
        let owner = self.entities[summoned].owner;
        let races = self.entities[summoned].card.races;
        let mut holders: Vec<EntityId> = self.players[owner]
            .board
            .iter()
            .copied()
            .filter(|&m| m != summoned)
            .collect();
        holders.sort_by_key(|&m| self.entities[m].play_order);
        for m in holders {
            let Some(def) = self.entities[m].def() else {
                continue;
            };
            for t in def.triggers {
                let When::FriendlySummon { race } = t.when else {
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
            if !self.affordable(entity) || !self.usable(entity) {
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

    fn affordable(&self, entity: &Entity) -> bool {
        // 超載可以讓可用法力變成負的，但 0 費的牌一律能打出
        let cost = entity.cost();
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
        }
    }

    /// 英雄技能可以使用時回傳它的目標條件。
    fn power_req(&self) -> Option<TargetReq> {
        let me = &self.players[self.current];
        let power = &self.entities[me.hero_power];
        let def = power.def()?;
        if me.power_used || !self.affordable(power) || !self.usable(power) {
            return None;
        }
        if def.target != TargetReq::None && self.targets(def.target, self.current).is_empty() {
            return None;
        }
        Some(def.target)
    }

    /// 符合目標條件的角色（player 為使用者）。對手的潛行手下不能選。
    fn targets(&self, req: TargetReq, player: usize) -> Vec<Target> {
        let (heroes, minions, friendly, enemy) = match req {
            TargetReq::None => return Vec::new(),
            TargetReq::Character => (true, true, true, true),
            TargetReq::Minion => (false, true, true, true),
            TargetReq::FriendlyMinion => (false, true, true, false),
            TargetReq::EnemyMinion => (false, true, false, true),
            TargetReq::FriendlyCharacter => (true, true, true, false),
            TargetReq::EnemyCharacter => (true, true, false, true),
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
                    if mine || !self.entities[m].stealth {
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
                if !self.affordable(entity) || !self.usable(entity) {
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
        if e.owner != self.current || e.frozen {
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
        attack > 0 && e.attacks_this_turn < Self::max_attacks(windfury)
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
            .filter(|(_, m)| !m.stealth)
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
        let e = self.players[p].hand.remove(hand);
        let card = self.entities[e].card;
        self.players[p].mana -= self.entities[e].cost();
        self.players[p].overload_owed += card.overload;
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
                self.enter_board(p, e, pos);
            }
            CardType::Weapon => self.equip_entity(p, e),
            _ => {}
        }
        if card.card_type == CardType::Spell {
            // 對手的反制秘密
            self.countered = false;
            self.reveal_secrets(1 - p, &[When::OpponentCastsSpell], e);
            if self.countered {
                self.countered = false;
                self.death_phase();
                return;
            }
            if card.keywords.contains(&Keyword::Secret) {
                self.players[p].secrets.push(e);
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
        if card.card_type == CardType::Minion {
            // 戰吼結算後先進入死亡階段，再做「召喚後」觸發與對手「打出手下後」的秘密（case C1、C6）；
            // 英雄在這個死亡階段死亡時流程照樣繼續，最後的死亡階段才定勝負（case C7）
            self.death_phase();
            self.fire_summon(e);
            self.reveal_secrets(1 - p, &[When::AfterOpponentPlaysMinion], e);
        }
        self.death_phase();
    }

    /// 揭露並觸發 player 秘密區中條件在 whens 裡的秘密（不分條件，依打出順序）；target 是觸發者。
    /// 觸發者已死亡或離開戰場時，之後作用在觸發者上的秘密不再觸發；「英雄被攻擊」照樣觸發。
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
                Some(trigger) if !gone || trigger.when == When::OwnHeroAttacked => {
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

    /// 沉默：效果、觸發、被動加成、死亡之聲失效；數值回到卡牌原始值（加上別人給的被動加成）；
    /// 目前生命只在超過新上限時才降；解除凍結、拿掉關鍵字。
    fn silence(&mut self, id: EntityId) {
        let e = &mut self.entities[id];
        e.silenced = true;
        e.attack = e.card.attack.unwrap_or(0) + e.aura_attack;
        e.temp_attack = 0;
        e.max_health = e.card.health.unwrap_or(0) + e.aura_health;
        e.health = e.health.min(e.max_health);
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

    /// 選擇畫面選第 index 個：從牌庫發現的牌放進手牌（手牌滿了摧毀）。
    fn choose(&mut self, index: usize) {
        let (p, options) = self.choice.take().expect("已檢查有選擇畫面");
        let picked = options[index];
        let player = &mut self.players[p];
        player.deck.retain(|&c| c != picked);
        if player.hand.len() < HAND_LIMIT {
            player.hand.push(picked);
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
        let p = self.current;
        let hero_attacking = matches!(attacker, Target::Hero { .. });
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
        let whens: &[When] = match (matches!(target, Target::Hero { .. }), hero_attacking) {
            (true, false) => &[When::OwnHeroAttacked, When::EnemyMinionAttacks],
            (true, true) => &[When::OwnHeroAttacked],
            (false, false) => &[When::EnemyMinionAttacks],
            (false, true) => &[],
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
        let target_attack = match target {
            Target::Minion { .. } => self.entities[t].attack,
            Target::Hero { .. } => 0,
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
        if dealt > 0 && a_poisonous && matches!(target, Target::Minion { .. }) {
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
                for t in self.select(to, ctx) {
                    self.damage(t, amount + bonus, false, ctx.owner);
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
                    if self.summon(ctx.owner, card, pos).is_some() && ctx.deathrattle {
                        // 放入一隻後，還沒結算的死亡手下中位置在插入點或右邊的往右推一格
                        for m in &mut self.dr_markers {
                            if m.1 == ctx.owner && m.2 >= pos {
                                m.2 += 1;
                            }
                        }
                    }
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
                    candidates
                        .retain(|&c| self.entities[c].health > 0 && !self.entities[c].destroyed);
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
        list.sort_by_key(|&c| self.entities[c].card.dbf_id);
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
            return;
        }
        let player = &mut self.players[p];
        if player.hand.len() < HAND_LIMIT {
            player.hand.push(card);
        }
    }

    fn start_turn(&mut self) {
        let p = self.current;
        for q in 0..2 {
            let hero = self.players[q].hero;
            self.entities[hero].attacks_this_turn = 0;
            for &m in &self.players[q].board {
                self.entities[m].attacks_this_turn = 0;
            }
        }
        for &m in &self.players[p].board {
            self.entities[m].summoning_sick = false;
        }
        let me = &mut self.players[p];
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
        // 本回合攻擊次數沒用完（也不是剛上場）的凍結角色解凍；用完才被凍結的要等下個自己的回合。
        let hero = self.players[p].hero;
        let weapon_windfury = self.players[p]
            .weapon
            .is_some_and(|w| self.entities[w].windfury);
        let h = &mut self.entities[hero];
        if h.frozen && h.attacks_this_turn < Self::max_attacks(h.windfury || weapon_windfury) {
            h.frozen = false;
        }
        for &m in &self.players[p].board {
            let e = &mut self.entities[m];
            if e.frozen && e.attacks_this_turn < Self::max_attacks(e.windfury) && !e.summoning_sick
            {
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
