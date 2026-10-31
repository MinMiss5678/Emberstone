//! Emberstone 命令列工具。
//!
//! - `emberstone bench [--games N] [--seed S]`：用第 1 階段的純關鍵字卡池隨機組牌、隨機對局 N 局，
//!   輸出每局平均時間，以及第 10 回合複製一次對局的時間（第 1 階段只輸出數字、不設門檻）。

use std::process::ExitCode;
use std::time::Instant;

use emberstone_core::{Game, GameConfig, Outcome, PlayerConfig, RngSource};

/// 第 1 階段差分測試的卡池（`difftest keywords` 產生）。
const KEYWORD_POOL: &str = include_str!("../../../data/keyword_pool.txt");
/// 一局最多幾個行動（防止隨機對局卡住）。
const MAX_ACTIONS: usize = 1000;
/// 第 10 回合複製量測的次數。
const CLONES: u32 = 1000;

/// 可重現的簡單亂數產生器（SplitMix64）。
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

fn pool() -> Vec<&'static str> {
    KEYWORD_POOL
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// 每張最多 2 張，隨機抽 30 張。
fn random_deck(pool: &[&'static str], rng: &mut SplitMix64) -> Vec<&'static str> {
    let mut slots: Vec<&'static str> = pool.iter().flat_map(|&id| [id, id]).collect();
    for i in (1..slots.len()).rev() {
        slots.swap(i, rng.below(i + 1));
    }
    slots.truncate(30);
    slots
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> Result<T, String> {
    match args.iter().position(|a| a == name) {
        None => Ok(default),
        Some(i) => args
            .get(i + 1)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| format!("{name} 的值不正確")),
    }
}

fn cmd_bench(args: &[String]) -> Result<(), String> {
    let games: usize = arg(args, "--games", 1000)?;
    let seed: u64 = arg(args, "--seed", 1)?;
    if games == 0 {
        return Err("--games 至少要 1".to_string());
    }
    let pool = pool();
    let mut rng = SplitMix64(seed);
    let mut wins = [0usize; 2];
    let mut draws = 0;
    let mut unfinished = 0;
    let mut actions_total = 0usize;
    let mut clone_total = 0.0f64;
    let mut clone_samples = 0u32;
    let mut game_time = 0.0f64;

    for _ in 0..games {
        let config = GameConfig {
            players: [
                PlayerConfig {
                    hero: "HERO_08",
                    deck: random_deck(&pool, &mut rng),
                },
                PlayerConfig {
                    hero: "HERO_01",
                    deck: random_deck(&pool, &mut rng),
                },
            ],
            shuffle: true,
            rng: RngSource::Seed(rng.next_u64()),
        };
        let start = Instant::now();
        let mut game = Game::new(config).map_err(|e| format!("開局失敗：{e:?}"))?;
        let mut cloned = false;
        let mut steps = 0;
        while game.outcome().is_none() && steps < MAX_ACTIONS {
            if !cloned && game.turn() >= 10 {
                let t = Instant::now();
                for _ in 0..CLONES {
                    std::hint::black_box(game.clone());
                }
                let elapsed = t.elapsed().as_secs_f64();
                clone_total += elapsed;
                clone_samples += CLONES;
                cloned = true;
                // 複製量測的時間不算進對局時間
                game_time -= elapsed;
            }
            let actions = game.legal_actions();
            let a = &actions[rng.below(actions.len())];
            game.act(a).map_err(|e| format!("行動 {a:?} 失敗：{e:?}"))?;
            steps += 1;
        }
        game_time += start.elapsed().as_secs_f64();
        actions_total += steps;
        match game.outcome() {
            Some(Outcome::Won(p)) => wins[p] += 1,
            Some(Outcome::Draw) => draws += 1,
            None => unfinished += 1,
        }
    }

    println!("對局數：{games}");
    println!("每局平均（ms）：{:.4}", game_time / games as f64 * 1e3);
    println!("每局平均行動數：{:.1}", actions_total as f64 / games as f64);
    if clone_samples > 0 {
        println!(
            "第 10 回合複製一次（μs）：{:.4}",
            clone_total / f64::from(clone_samples) * 1e6
        );
    } else {
        println!("第 10 回合複製一次（μs）：沒有對局進行到第 10 回合");
    }
    println!(
        "勝負：先手 {}、後手 {}、平手 {draws}、未結束 {unfinished}",
        wins[0], wins[1]
    );
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("bench") => cmd_bench(&args),
        _ => Err("用法：emberstone bench [--games N] [--seed S]".to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("錯誤：{e}");
            ExitCode::FAILURE
        }
    }
}
