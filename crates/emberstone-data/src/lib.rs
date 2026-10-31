//! 卡片資料：數值、關鍵字、台版卡名。
//!
//! 卡表由 `emberstone-cardgen` 從 HearthstoneJSON 的 `tw_all.json` 產生並納入版本控制，
//! 編譯時不依賴外部路徑。

// 產生檔有 11 MB，rustfmt 處理會記憶體不足，所以略過。
#[rustfmt::skip]
#[path = "generated/cards.rs"]
mod generated;

use generated::{CARDS, DBF_INDEX, STANDARD_SETS};

/// 卡片類型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    Minion,
    Spell,
    Weapon,
    Hero,
    HeroPower,
    Location,
    Enchantment,
    Other(&'static str),
}

/// 職業。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardClass {
    Neutral,
    DeathKnight,
    DemonHunter,
    Druid,
    Hunter,
    Mage,
    Paladin,
    Priest,
    Rogue,
    Shaman,
    Warlock,
    Warrior,
    Other(&'static str),
}

/// 稀有度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    Free,
    Common,
    Rare,
    Epic,
    Legendary,
    Other(&'static str),
}

/// 關鍵字（HearthstoneJSON 的 mechanics）。沒有對應列舉的收成 `Other`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Taunt,
    DivineShield,
    Charge,
    Rush,
    Windfury,
    Poisonous,
    Lifesteal,
    Stealth,
    Freeze,
    Battlecry,
    Deathrattle,
    SpellPower,
    Overload,
    Secret,
    Silence,
    Discover,
    ChooseOne,
    Combo,
    Reborn,
    Other(&'static str),
}

/// 死亡騎士符文需求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuneCost {
    pub blood: u8,
    pub frost: u8,
    pub unholy: u8,
}

/// 一張卡的靜態資料。
#[derive(Debug)]
pub struct Card {
    pub id: &'static str,
    pub dbf_id: u32,
    pub name: &'static str,
    pub card_type: CardType,
    pub class: Option<CardClass>,
    /// 多職業卡的職業清單；單一職業的卡為空。
    pub classes: &'static [CardClass],
    pub cost: Option<i32>,
    pub attack: Option<i32>,
    pub health: Option<i32>,
    pub durability: Option<i32>,
    pub armor: Option<i32>,
    pub rarity: Option<Rarity>,
    pub set: &'static str,
    pub collectible: bool,
    pub races: &'static [&'static str],
    pub spell_school: Option<&'static str>,
    pub keywords: &'static [Keyword],
    pub overload: i32,
    pub spell_damage: i32,
    pub rune_cost: RuneCost,
    /// 重印卡：組牌時和這張 dbfId 的卡合併計算張數上限。
    pub count_as_copy_of: Option<u32>,
    /// 純文字敘述（拿掉 HTML 標記與 `$`、`#` 數值標記）。
    pub text: &'static str,
}

impl Card {
    /// 是否屬於目前的標準模式系列。
    pub fn is_standard(&self) -> bool {
        is_standard_set(self.set)
    }
}

/// 全部卡片（依卡號排序）。
pub fn all_cards() -> &'static [Card] {
    CARDS
}

/// 依卡號查卡。
pub fn card_by_id(id: &str) -> Option<&'static Card> {
    CARDS
        .binary_search_by(|c| c.id.cmp(id))
        .ok()
        .map(|i| &CARDS[i])
}

/// 依 dbfId 查卡。
pub fn card_by_dbf_id(dbf_id: u32) -> Option<&'static Card> {
    DBF_INDEX
        .binary_search_by_key(&dbf_id, |&(d, _)| d)
        .ok()
        .map(|i| &CARDS[DBF_INDEX[i].1 as usize])
}

/// 系列是否在目前的標準模式（清單來自 `data/standard_sets.txt`）。
pub fn is_standard_set(set: &str) -> bool {
    STANDARD_SETS.contains(&set)
}
