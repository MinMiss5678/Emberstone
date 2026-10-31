//! difftest：卡片批次分類與差分測試驗收。
//!
//! - `difftest batches --rosetta <RosettaStone 原始碼目錄> [--en data/en_all.json] [--out data/batches.tsv]`
//! - `difftest run --batch A --games N --seed S`：用指定批次的卡組隨機牌組，
//!   在 RosettaStone 上記錄一局再用記錄的亂數重播，逐步比對狀態。
//! - `difftest run --engine emberstone --pool keywords --games N --seed S`：
//!   在 RosettaStone 記錄隨機對局（不使用英雄技能），再讓 Emberstone 用同樣的行動和亂數重播，
//!   逐步比對狀態與合法行動。
//! - `difftest keywords --rosetta <RosettaStone 原始碼目錄> [--en data/en_all.json] [--out data/keyword_pool.txt]`：
//!   產生第 1 階段的純關鍵字卡池。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use emberstone_data::{CardClass, CardType, Rarity, card_by_id};
use emberstone_difftest::actions::Action;
use emberstone_difftest::batches::{Batch, Row, classify, implemented_ids, parse_tsv, to_tsv};
use emberstone_difftest::bridge::{Bridge, GameSetup, PlayerSetup, RngMode};
use emberstone_difftest::emberstone;
use emberstone_difftest::generate::{OptionSource, SplitMix64, random_actions};
use emberstone_difftest::pool::{keyword_pool, parse_pool};
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
    let rows = classify(
        &read(&en)?,
        &read(&rosetta.join("Resources/cards.json"))?,
        &rosetta_implemented(&rosetta)?,
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

fn rosetta_implemented(rosetta: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let lists: Vec<String> = ["Classic", "Standard", "Wild"]
        .iter()
        .map(|f| read(&rosetta.join(format!("Documents/CardList - {f}.md"))))
        .collect::<Result<_, _>>()?;
    let refs: Vec<&str> = lists.iter().map(String::as_str).collect();
    Ok(implemented_ids(&refs))
}

fn cmd_keywords(args: &[String]) -> Result<(), String> {
    let rosetta =
        PathBuf::from(arg(args, "--rosetta").ok_or("需要 --rosetta <RosettaStone 原始碼目錄>")?);
    let en = arg(args, "--en").map_or_else(|| root().join("data/en_all.json"), PathBuf::from);
    let out =
        arg(args, "--out").map_or_else(|| root().join("data/keyword_pool.txt"), PathBuf::from);
    let ids = keyword_pool(
        &read(&en)?,
        &read(&rosetta.join("Resources/cards.json"))?,
        &rosetta_implemented(&rosetta)?,
    )?;
    std::fs::write(&out, emberstone_difftest::pool::to_text(&ids))
        .map_err(|e| format!("寫不進 {}：{e}", out.display()))?;
    println!("卡池 {} 張；已寫入 {}", ids.len(), out.display());
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

    /// 卡號清單（卡池檔）組成的卡池。
    fn from_ids(ids: &[String]) -> Result<Self, String> {
        let cards = ids
            .iter()
            .map(|id| card_by_id(id).ok_or_else(|| format!("卡表沒有 {id}")))
            .collect::<Result<_, _>>()?;
        Ok(Self { cards })
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
    /// 每一步的合法行動（和 Emberstone 比對用）。
    options: Vec<Vec<Action>>,
    /// 不使用英雄技能（第 1 階段）。
    skip_power: bool,
    error: Option<String>,
}

impl OptionSource for Recorder<'_> {
    fn options(&mut self) -> Vec<Action> {
        match self.bridge.options() {
            Ok(mut v) => {
                if self.skip_power {
                    v.retain(|a| !matches!(a, Action::Power { .. }));
                }
                self.options.push(v.clone());
                v
            }
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

/// 一局的設定：雙方牌組（RosettaStone 卡號）、記錄用種子、行動種子。
struct Plan {
    p1: PlayerSetup,
    p2: PlayerSetup,
    record_seed: u64,
    action_seed: u64,
}

fn plan(pool: &Pool, to_rosetta: &HashMap<&str, &str>, seed: u64) -> Result<Plan, String> {
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
    Ok(Plan {
        p1,
        p2,
        record_seed,
        action_seed,
    })
}

/// 在 RosettaStone 記錄一局：回傳行動、每步快照、每步合法行動與亂數事件；出錯回傳錯誤局。
struct Recorded {
    actions: Vec<Action>,
    snapshots: Vec<Snapshot>,
    options: Vec<Vec<Action>>,
    events: Vec<emberstone_difftest::rng::RngEvent>,
}

fn record(
    bridge: &mut Bridge,
    plan: &Plan,
    skip_power: bool,
) -> Result<Result<Recorded, String>, String> {
    let setup = GameSetup {
        p1: plan.p1.clone(),
        p2: plan.p2.clone(),
        rng: RngMode::Record {
            seed: plan.record_seed,
        },
    };
    let first = match bridge.new_game(&setup) {
        Ok(s) => s,
        Err(e) => return Ok(Err(format!("開局失敗：{}：{}", e.kind, e.message))),
    };
    let mut rec = Recorder {
        bridge,
        snapshots: vec![first],
        options: Vec::new(),
        skip_power,
        error: None,
    };
    let actions = random_actions(plan.action_seed, &mut rec, MAX_ACTIONS);
    if let Some(e) = rec.error {
        return Ok(Err(format!("記錄時：{e}")));
    }
    let snapshots = rec.snapshots;
    let options = rec.options;
    let events = bridge
        .events()
        .map_err(|e| format!("取不到亂數事件：{}", e.message))?;
    Ok(Ok(Recorded {
        actions,
        snapshots,
        options,
        events,
    }))
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
    let plan = plan(pool, &to_rosetta, seed)?;
    let rec = match record(bridge, &plan, false)? {
        Ok(r) => r,
        Err(e) => return Ok(Outcome::Error(e)),
    };
    let Recorded {
        actions,
        snapshots: recorded,
        events,
        ..
    } = rec;
    let setup = |rng_mode: RngMode| GameSetup {
        p1: plan.p1.clone(),
        p2: plan.p2.clone(),
        rng: rng_mode,
    };

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
    let emberstone_engine = match arg(args, "--engine").as_deref() {
        None | Some("rosetta") => false,
        Some("emberstone") => true,
        other => {
            return Err(format!(
                "--engine 只能是 rosetta 或 emberstone（收到 {other:?}）"
            ));
        }
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
    let pool = match (
        arg(args, "--pool").as_deref(),
        arg(args, "--batch").as_deref(),
    ) {
        (Some("keywords"), None) => {
            Pool::from_ids(&parse_pool(&read(&root().join("data/keyword_pool.txt"))?))?
        }
        (None, Some("A")) => Pool::new(&rows, Batch::A),
        (None, Some("B")) => Pool::new(&rows, Batch::B),
        (p, b) => {
            return Err(format!(
                "要指定 --batch A|B 或 --pool keywords 其中一個（收到 {p:?}、{b:?}）"
            ));
        }
    };
    let mut bridge = Bridge::spawn_default()?;

    let (mut consistent, mut different) = (0, 0);
    for g in 0..games {
        // 上一局讓橋接程式逾時或當掉時，重新啟動一個。
        if !bridge.is_alive() {
            bridge = Bridge::spawn_default()?;
        }
        let game_seed = seed.wrapping_mul(1_000_003).wrapping_add(g as u64);
        let outcome = if emberstone_engine {
            play_emberstone(&mut bridge, &pool, game_seed)?
        } else {
            play_game(&mut bridge, &pool, &rows, game_seed)?
        };
        match outcome {
            Outcome::Consistent => {
                consistent += 1;
                println!("第 {} 局：一致", g + 1);
            }
            Outcome::Different(d) => {
                different += 1;
                println!("第 {} 局：不一致，{d}", g + 1);
            }
            Outcome::Error(e) => println!("第 {} 局：錯誤，{e}", g + 1),
        }
    }
    println!("一致 {consistent}/{games}");
    if emberstone_engine {
        // RosettaStone 本身的錯誤（逾時、當機）不算不一致（case G3）。
        println!(
            "不一致 {different} 局；RosettaStone 錯誤 {} 局",
            games - consistent - different
        );
        Ok(different == 0)
    } else {
        Ok(consistent == games)
    }
}

/// 依 Debug 字串排序，比對合法行動清單用。
fn sorted_actions(v: &[Action]) -> Vec<String> {
    let mut s: Vec<String> = v.iter().map(|a| format!("{a:?}")).collect();
    s.sort();
    s
}

/// 在 RosettaStone 記錄一局（不使用英雄技能），再讓 Emberstone 用同樣的行動與亂數重播，
/// 逐步比對合法行動與快照。
fn play_emberstone(bridge: &mut Bridge, pool: &Pool, seed: u64) -> Result<Outcome, String> {
    let plan = plan(pool, &HashMap::new(), seed)?;
    let rec = match record(bridge, &plan, true)? {
        Ok(r) => r,
        Err(e) => return Ok(Outcome::Error(e)),
    };
    let setup = GameSetup {
        p1: plan.p1.clone(),
        p2: plan.p2.clone(),
        rng: RngMode::Replay {
            events: rec.events.clone(),
        },
    };
    let mut game = match emberstone::new_game(&setup) {
        Ok(g) => g,
        Err(e) => return Ok(Outcome::Different(format!("Emberstone 開局失敗：{e}"))),
    };
    let compare = |step: usize, ours: Snapshot| -> Option<Outcome> {
        let theirs = rec.snapshots.get(step)?;
        first_difference(std::slice::from_ref(theirs), &[ours])
            .map(|d| Outcome::Different(format!("第 {step} 步不同：{}", d.fields.join("、"))))
    };
    if let Some(d) = compare(0, emberstone::snapshot(&game)) {
        return Ok(d);
    }
    for (step, a) in rec.actions.iter().enumerate() {
        let theirs = sorted_actions(&rec.options[step]);
        let mine = sorted_actions(&emberstone::options(&game));
        if theirs != mine {
            let only = |x: &[String], y: &[String]| -> Vec<String> {
                x.iter().filter(|s| !y.contains(s)).cloned().collect()
            };
            return Ok(Outcome::Different(format!(
                "第 {step} 步合法行動不同：只有 RosettaStone {:?}；只有 Emberstone {:?}",
                only(&theirs, &mine),
                only(&mine, &theirs)
            )));
        }
        if let Err(e) = emberstone::act(&mut game, a) {
            return Ok(Outcome::Different(format!(
                "第 {step} 步 Emberstone 執行失敗：{e}（行動 {a:?}）"
            )));
        }
        if let Some(d) = compare(step + 1, emberstone::snapshot(&game)) {
            return Ok(d);
        }
    }
    Ok(Outcome::Consistent)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("batches") => cmd_batches(&args).map(|_| true),
        Some("keywords") => cmd_keywords(&args).map(|_| true),
        Some("run") => cmd_run(&args),
        _ => Err(
            "用法：difftest batches --rosetta <目錄> | difftest keywords --rosetta <目錄> | \
             difftest run [--engine emberstone] (--batch A | --pool keywords) --games N --seed S"
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
