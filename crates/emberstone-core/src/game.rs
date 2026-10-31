//! 對局狀態、回合流程、出牌、戰鬥與合法行動（第 1 階段：只有數值、關鍵字和幸運幣）。

use emberstone_data::{Card, CardType, Keyword, card_by_id};

use crate::rng::{Desync, Rng, RngEvent};

/// 實體編號（`Game` 內實體陣列的索引）。
pub type EntityId = usize;

const HAND_LIMIT: usize = 10;
const BOARD_LIMIT: usize = 7;
const MANA_LIMIT: i32 = 10;
/// 幸運幣。開局規則的一部分，所以由 core 處理；第 2 階段有效果註冊後再搬到 cards。
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

/// 英雄、手下、武器或手牌／牌庫中的卡。
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub card: &'static Card,
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
    /// 本回合剛上場（沒有衝鋒就不能攻擊；有衝刺只能攻擊手下）。
    pub summoning_sick: bool,
    pub attacks_this_turn: i32,
    /// 被致命劇毒等效果消滅，等死亡階段移出戰場。
    pub destroyed: bool,
}

impl Entity {
    fn new(card: &'static Card, owner: usize) -> Self {
        let has = |k: Keyword| card.keywords.contains(&k);
        let health = card.health.unwrap_or(0);
        Entity {
            card,
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
            summoning_sick: false,
            attacks_this_turn: 0,
            destroyed: false,
        }
    }
}

/// 一位玩家。牌庫第 0 張是下一張要抽的牌。
#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub hero: EntityId,
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
}

/// 一方的英雄與牌組（照抽牌順序）。
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
        let mut entities = Vec::new();
        let mut make_player = |p: usize, cfg: &PlayerConfig| -> Result<Player, GameError> {
            let hero = entities.len();
            entities.push(Entity::new(lookup(cfg.hero)?, p));
            let mut deck = Vec::with_capacity(cfg.deck.len());
            for id in &cfg.deck {
                deck.push(entities.len());
                entities.push(Entity::new(lookup(id)?, p));
            }
            Ok(Player {
                hero,
                weapon: None,
                hand: Vec::with_capacity(HAND_LIMIT),
                deck,
                board: Vec::with_capacity(BOARD_LIMIT),
                mana: 0,
                max_mana: 0,
                overload_locked: 0,
                overload_owed: 0,
                fatigue: 0,
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

    /// 加一張卡到手牌最右邊；手牌已滿回傳 None。
    pub fn add_to_hand(&mut self, player: usize, card_id: &str) -> Option<EntityId> {
        let card = card_by_id(card_id)?;
        if self.players[player].hand.len() >= HAND_LIMIT {
            return None;
        }
        let id = self.entities.len();
        self.entities.push(Entity::new(card, player));
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
        let id = self.entities.len();
        let mut e = Entity::new(card, player);
        e.summoning_sick = true;
        self.entities.push(e);
        self.players[player].board.insert(pos, id);
        Some(id)
    }

    /// 目前玩家的所有合法行動；遊戲結束時為空。
    pub fn legal_actions(&self) -> Vec<Action> {
        let mut out = Vec::new();
        if self.outcome.is_some() {
            return out;
        }
        let me = &self.players[self.current];
        for (i, &e) in me.hand.iter().enumerate() {
            let card = self.entities[e].card;
            if !self.affordable(card) {
                continue;
            }
            match card.card_type {
                CardType::Minion => {
                    if me.board.len() < BOARD_LIMIT {
                        for pos in 0..=me.board.len() {
                            out.push(play_action(i, Some(pos)));
                        }
                    }
                }
                CardType::Weapon | CardType::Spell => out.push(play_action(i, None)),
                _ => {}
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
        out.push(Action::End);
        out
    }

    /// 執行行動；不合法時狀態不變。
    pub fn act(&mut self, action: &Action) -> Result<(), ActError> {
        if self.outcome.is_some() {
            return Err(ActError::GameOver);
        }
        if !self.is_legal(action) {
            return Err(ActError::Illegal);
        }
        match *action {
            Action::Play { hand, pos, .. } => self.play(hand, pos),
            Action::Attack { attacker, target } => self.attack(attacker, target),
            Action::End => self.end_turn(),
            Action::Power { .. } | Action::Choose { .. } => unreachable!("已檢查為不合法"),
        }
        Ok(())
    }

    // ---- 合法性 ----

    fn affordable(&self, card: &Card) -> bool {
        card.cost.unwrap_or(0) <= self.players[self.current].mana
    }

    fn is_legal(&self, action: &Action) -> bool {
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
                let card = self.entities[e].card;
                if target.is_some() || choose_one != 0 || !self.affordable(card) {
                    return false;
                }
                match (card.card_type, pos) {
                    (CardType::Minion, Some(p)) => {
                        me.board.len() < BOARD_LIMIT && p <= me.board.len()
                    }
                    (CardType::Weapon | CardType::Spell, None) => true,
                    _ => false,
                }
            }
            Action::Attack { attacker, target } => {
                self.owner_of(attacker) == Some(self.current)
                    && self.can_attack(attacker)
                    && self.attack_targets(attacker).contains(&target)
            }
            Action::End => true,
            Action::Power { .. } | Action::Choose { .. } => false,
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
        attack > 0 && e.attacks_this_turn < if windfury { 2 } else { 1 }
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

    fn play(&mut self, hand: usize, pos: Option<usize>) {
        let p = self.current;
        let e = self.players[p].hand.remove(hand);
        let card = self.entities[e].card;
        self.players[p].mana -= card.cost.unwrap_or(0);
        self.players[p].overload_owed += card.overload;
        match card.card_type {
            CardType::Minion => {
                let pos = pos.expect("已檢查手下有位置");
                self.entities[e].summoning_sick = true;
                self.players[p].board.insert(pos, e);
            }
            CardType::Weapon => {
                self.players[p].weapon = Some(e);
            }
            _ => {
                if card.id == COIN {
                    self.players[p].mana += 1;
                }
            }
        }
        self.death_phase();
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
        let source_attack = if hero_attacking {
            self.hero_attack(p)
        } else {
            self.entities[a].attack
        };
        // 武器只在自己的回合有作用，所以被攻擊的英雄不反擊。
        let target_attack = match target {
            Target::Minion { .. } => self.entities[t].attack,
            Target::Hero { .. } => 0,
        };
        let has = |g: &Game, f: fn(&Entity) -> bool| {
            f(&g.entities[a]) || weapon.is_some_and(|w| f(&g.entities[w]))
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
            let hero = &mut self.entities[self.players[source_owner].hero];
            hero.health = (hero.health + amount).min(hero.max_health).max(hero.health);
        }
        amount
    }

    /// 死亡階段：移除耐久度歸零的武器、死亡的手下；判斷勝負。
    fn death_phase(&mut self) {
        for p in 0..2 {
            if let Some(w) = self.players[p].weapon
                && self.entities[w].durability <= 0
            {
                self.players[p].weapon = None;
            }
            let entities = &self.entities;
            self.players[p]
                .board
                .retain(|&m| entities[m].health > 0 && !entities[m].destroyed);
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
        self.draw(p);
        self.death_phase();
    }

    fn end_turn(&mut self) {
        let p = self.current;
        // 本回合沒有攻擊（也不是剛上場）的凍結角色解凍。
        let hero = self.players[p].hero;
        let h = &mut self.entities[hero];
        if h.frozen && h.attacks_this_turn == 0 {
            h.frozen = false;
        }
        for &m in &self.players[p].board {
            let e = &mut self.entities[m];
            if e.frozen && e.attacks_this_turn == 0 && !e.summoning_sick {
                e.frozen = false;
            }
        }
        self.current = 1 - p;
        self.turn += 1;
        self.start_turn();
    }
}

fn play_action(hand: usize, pos: Option<usize>) -> Action {
    Action::Play {
        hand,
        pos,
        target: None,
        choose_one: 0,
    }
}
