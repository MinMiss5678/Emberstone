//! difftest：卡片批次分類與差分測試驗收。
//!
//! - `difftest batches --rosetta <RosettaStone 原始碼目錄> [--en data/en_all.json] [--out data/batches.tsv]`
//! - `difftest run --batch A --games N --seed S`：用指定批次的卡組隨機牌組，
//!   在 RosettaStone 上記錄一局再用記錄的亂數重播，逐步比對狀態。
//!   （Emberstone 引擎完成後改成和 Emberstone 比對。）

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use emberstone_data::{CardClass, CardType, Rarity, card_by_id};
use emberstone_difftest::actions::Action;
use emberstone_difftest::batches::{Batch, Row, classify, implemented_ids, parse_tsv, to_tsv};
use emberstone_difftest::bridge::{Bridge, GameSetup, PlayerSetup, RngMode};
use emberstone_difftest::generate::{OptionSource, SplitMix64, random_actions};
use emberstone_difftest::snapshot::{Snapshot, first_difference};

const MAX_ACTIONS: usize = 1000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("讀不到 {}：{e}", path.display()))
}

fn cmd_batches(args: &[String]) -> Result<(), String> {
    let rosetta =
        PathBuf::from(arg(args, "--rosetta").ok_or("需要 --rosetta <RosettaStone 原始碼目錄>")?);
    let en = arg(args, "--en").map_or_else(|| root().join("data/en_all.json"), PathBuf::from);
    let out = arg(args, "--out").map_or_else(|| root().join("data/batches.tsv"), PathBuf::from);
    let lists: Vec<String> = ["Classic", "Standard", "Wild"]
        .iter()
        .map(|f| read(&rosetta.join(format!("Documents/CardList - {f}.md"))))
        .collect::<Result<_, _>>()?;
    let refs: Vec<&str> = lists.iter().map(String::as_str).collect();
    let rows = classify(
        &read(&en)?,
        &read(&rosetta.join("Resources/cards.json"))?,
        &implemented_ids(&refs),
    )?;
    std::fs::write(&out, to_tsv(&rows)).map_err(|e| format!("寫不進 {}：{e}", out.display()))?;
    let count = |b: Batch| rows.iter().filter(|r| r.batch == b).count();
    println!(
        "共 {} 張：A {}、B {}、C {}；已寫入 {}",
        rows.len(),
        count(Batch::A),
        count(Batch::B),
        count(Batch::C),
        out.display()
    );
    Ok(())
}

fn rosetta_class(class: CardClass) -> Option<&'static str> {
    Some(match class {
        CardClass::DemonHunter => "DEMONHUNTER",
        CardClass::Druid => "DRUID",
        CardClass::Hunter => "HUNTER",
        CardClass::Mage => "MAGE",
        CardClass::Paladin => "PALADIN",
        CardClass::Priest => "PRIEST",
        CardClass::Rogue => "ROGUE",
        CardClass::Shaman => "SHAMAN",
        CardClass::Warlock => "WARLOCK",
        CardClass::Warrior => "WARRIOR",
        _ => return None,
    })
}

/// 指定批次中可放進牌組的卡（我們的卡號）。
struct Pool {
    cards: Vec<&'static emberstone_data::Card>,
}

impl Pool {
    fn new(rows: &[Row], batch: Batch) -> Self {
        let cards = rows
            .iter()
            .filter(|r| r.batch == batch && r.rosetta_id.is_some())
            .filter_map(|r| card_by_id(&r.id))
            .filter(|c| {
                matches!(
                    c.card_type,
                    CardType::Minion | CardType::Spell | CardType::Weapon
                )
            })
            .collect();
        Self { cards }
    }

    fn classes(&self) -> Vec<CardClass> {
        let mut v: Vec<CardClass> = Vec::new();
        for c in &self.cards {
            if let Some(class) = c.class.filter(|k| rosetta_class(*k).is_some())
                && !v.contains(&class)
            {
                v.push(class);
            }
        }
        v.sort_by_key(|k| rosetta_class(*k));
        v
    }

    /// 隨機組 30 張（傳說 1 張、其他最多 2 張），照抽牌順序。
    fn deck(&self, class: CardClass, rng: &mut SplitMix64) -> Result<Vec<&'static str>, String> {
        let mut slots: Vec<&'static str> = Vec::new();
        for c in &self.cards {
            if c.class == Some(class) || c.class == Some(CardClass::Neutral) {
                let copies = if c.rarity == Some(Rarity::Legendary) {
                    1
                } else {
                    2
                };
                slots.extend(std::iter::repeat_n(c.id, copies));
            }
        }
        for i in (1..slots.len()).rev() {
            slots.swap(i, rng.below(i + 1));
        }
        if slots.len() < 30 {
            return Err(format!("{class:?} 可用的卡不足 30 張"));
        }
        slots.truncate(30);
        Ok(slots)
    }
}

/// 透過橋接程式執行並記錄快照的對局。
struct Recorder<'a> {
    bridge: &'a mut Bridge,
    snapshots: Vec<Snapshot>,
    error: Option<String>,
}

impl OptionSource for Recorder<'_> {
    fn options(&mut self) -> Vec<Action> {
        match self.bridge.options() {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(format!("{}：{}", e.kind, e.message));
                Vec::new()
            }
        }
    }

    fn act(&mut self, action: &Action) -> bool {
        match self.bridge.act(action) {
            Ok(s) => {
                let over = s.game_over;
                self.snapshots.push(s);
                over
            }
            Err(e) => {
                self.error = Some(format!(
                    "{}：{}（行動 {action:?}{}）",
                    e.kind,
                    e.message,
                    describe(self.snapshots.last(), action)
                ));
                true
            }
        }
    }
}

/// 錯誤訊息用：行動牽涉到的卡號（依上一個快照）。
fn describe(last: Option<&Snapshot>, action: &Action) -> String {
    let Some(s) = last else {
        return String::new();
    };
    let me = &s.players[usize::from(s.current_player.saturating_sub(1))];
    match action {
        Action::Play { hand, .. } => me
            .hand
            .get(*hand)
            .map_or(String::new(), |id| format!("，手牌 {id}")),
        Action::Attack { attacker, .. } => format!("，攻擊者 {attacker:?}"),
        _ => String::new(),
    }
}

/// 把快照裡的 RosettaStone 卡號換回我們的卡號。
fn to_our_ids(mut s: Snapshot, back: &HashMap<String, String>) -> Snapshot {
    let map = |id: &mut String| {
        if let Some(ours) = back.get(id.as_str()) {
            *id = ours.clone();
        }
    };
    for p in &mut s.players {
        p.hand.iter_mut().for_each(map);
        p.secrets.iter_mut().for_each(map);
        p.board.iter_mut().for_each(|m| map(&mut m.card));
        if let Some(w) = &mut p.hero.weapon {
            map(&mut w.card);
        }
    }
    s
}

enum Outcome {
    Consistent,
    Different(String),
    Error(String),
}

fn play_game(bridge: &mut Bridge, pool: &Pool, rows: &[Row], seed: u64) -> Result<Outcome, String> {
    let to_rosetta: HashMap<&str, &str> = rows
        .iter()
        .filter_map(|r| r.rosetta_id.as_deref().map(|rid| (r.id.as_str(), rid)))
        .collect();
    let back: HashMap<String, String> = rows
        .iter()
        .filter_map(|r| r.rosetta_id.clone().map(|rid| (rid, r.id.clone())))
        .collect();
    let classes = pool.classes();
    if classes.is_empty() {
        return Err("批次裡沒有可用的職業".to_string());
    }
    let mut rng = SplitMix64::new(seed);
    let player = |rng: &mut SplitMix64| -> Result<PlayerSetup, String> {
        let class = classes[rng.below(classes.len())];
        let deck = pool
            .deck(class, rng)?
            .into_iter()
            .map(|id| to_rosetta.get(id).copied().unwrap_or(id).to_string())
            .collect();
        Ok(PlayerSetup {
            class: rosetta_class(class).unwrap().to_string(),
            deck,
        })
    };
    let p1 = player(&mut rng)?;
    let p2 = player(&mut rng)?;
    let record_seed = rng.next_u64() & 0xFFFF_FFFF;
    let action_seed = rng.next_u64();

    let setup = |rng_mode: RngMode| GameSetup {
        p1: p1.clone(),
        p2: p2.clone(),
        rng: rng_mode,
    };
    let first = match bridge.new_game(&setup(RngMode::Record { seed: record_seed })) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Outcome::Error(format!(
                "開局失敗：{}：{}",
                e.kind, e.message
            )));
        }
    };
    let mut rec = Recorder {
        bridge,
        snapshots: vec![first],
        error: None,
    };
    let actions = random_actions(action_seed, &mut rec, MAX_ACTIONS);
    let recorded = rec.snapshots;
    if let Some(e) = rec.error {
        return Ok(Outcome::Error(format!("記錄時：{e}")));
    }
    let events = bridge
        .events()
        .map_err(|e| format!("取不到亂數事件：{}", e.message))?;

    let mut replayed = Vec::with_capacity(recorded.len());
    match bridge.new_game(&setup(RngMode::Replay { events })) {
        Ok(s) => replayed.push(s),
        Err(e) => {
            return Ok(Outcome::Error(format!(
                "重播開局：{}：{}",
                e.kind, e.message
            )));
        }
    }
    for a in &actions {
        match bridge.act(a) {
            Ok(s) => replayed.push(s),
            Err(e) => {
                return Ok(Outcome::Error(format!(
                    "重播時：{}：{}（行動 {a:?}）",
                    e.kind, e.message
                )));
            }
        }
    }

    let a: Vec<Snapshot> = recorded.into_iter().map(|s| to_our_ids(s, &back)).collect();
    let b: Vec<Snapshot> = replayed.into_iter().map(|s| to_our_ids(s, &back)).collect();
    Ok(match first_difference(&a, &b) {
        None => Outcome::Consistent,
        Some(d) => Outcome::Different(format!("第 {} 步不同：{}", d.step, d.fields.join("、"))),
    })
}

fn cmd_run(args: &[String]) -> Result<bool, String> {
    let batch = match arg(args, "--batch").as_deref() {
        Some("A") => Batch::A,
        Some("B") => Batch::B,
        other => return Err(format!("--batch 只能是 A 或 B（收到 {other:?}）")),
    };
    let games: usize = arg(args, "--games")
        .ok_or("需要 --games")?
        .parse()
        .map_err(|e| format!("--games：{e}"))?;
    let seed: u64 = arg(args, "--seed")
        .ok_or("需要 --seed")?
        .parse()
        .map_err(|e| format!("--seed：{e}"))?;
    let rows = parse_tsv(&read(&root().join("data/batches.tsv"))?)?;
    let pool = Pool::new(&rows, batch);
    let mut bridge = Bridge::spawn_default()?;

    let mut consistent = 0;
    for g in 0..games {
        // 上一局讓橋接程式逾時或當掉時，重新啟動一個。
        if !bridge.is_alive() {
            bridge = Bridge::spawn_default()?;
        }
        let game_seed = seed.wrapping_mul(1_000_003).wrapping_add(g as u64);
        match play_game(&mut bridge, &pool, &rows, game_seed)? {
            Outcome::Consistent => {
                consistent += 1;
                println!("第 {} 局：一致", g + 1);
            }
            Outcome::Different(d) => println!("第 {} 局：不一致，{d}", g + 1),
            Outcome::Error(e) => println!("第 {} 局：錯誤，{e}", g + 1),
        }
    }
    println!("一致 {consistent}/{games}");
    Ok(consistent == games)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("batches") => cmd_batches(&args).map(|_| true),
        Some("run") => cmd_run(&args),
        _ => Err(
            "用法：difftest batches --rosetta <目錄> | difftest run --batch A --games N --seed S"
                .to_string(),
        ),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("錯誤：{e}");
            ExitCode::FAILURE
        }
    }
}
