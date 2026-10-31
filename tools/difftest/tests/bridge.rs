//! RosettaStone 橋接程式（case R1–R3、R5、I1–I5、S1、C4）。
//! 需要先編好橋接程式（tools/rosetta-bridge/build.bat）；主要 CI 不跑這個測試檔。
//! 橋接程式位置可用環境變數 EMBERSTONE_ROSETTA_BRIDGE 指定。

use emberstone_difftest::actions::Action;
use emberstone_difftest::bridge::{Bridge, GameSetup, PlayerSetup, RngMode};
use emberstone_difftest::rng::RngEvent;
use emberstone_difftest::snapshot::Snapshot;

const FILL: [&str; 9] = [
    "CS2_120", "CS2_119", "CS2_179", "CS2_118", "CS2_131", "CS2_186", "CS2_200", "CS2_201",
    "EX1_582",
];

/// 前幾張固定，其餘用白板手下補滿 30 張（照抽牌順序）。
fn deck(head: &[&str]) -> Vec<String> {
    let mut d: Vec<String> = head.iter().map(|s| s.to_string()).collect();
    let mut i = 0;
    while d.len() < 30 {
        d.push(FILL[i % FILL.len()].to_string());
        i += 1;
    }
    d
}

fn p1_deck() -> Vec<String> {
    // 魚人襲擊者(1)、冰風雪人(4)、秘法飛彈(1)、血沼迅猛龍(2)
    deck(&["CS2_168", "CS2_182", "EX1_277", "CS2_172"])
}

fn p2_deck() -> Vec<String> {
    deck(&["CS2_168", "CS2_168", "CS2_120", "CS2_182"])
}

fn setup(rng: RngMode) -> GameSetup {
    GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: p1_deck(),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng,
    }
}

fn bridge() -> Bridge {
    Bridge::spawn_default().expect("啟動不了橋接程式（先執行 tools/rosetta-bridge/build.bat）")
}

fn play_minion(hand: usize, pos: usize) -> Action {
    Action::Play {
        hand,
        pos: Some(pos),
        target: None,
        choose_one: 0,
    }
}

fn cast(hand: usize) -> Action {
    Action::Play {
        hand,
        pos: None,
        target: None,
        choose_one: 0,
    }
}

/// 固定劇本：先手結束 → 後手下魚人襲擊者、結束 → 先手施放秘法飛彈（有兩個敵方目標）。
fn script() -> Vec<Action> {
    vec![Action::End, play_minion(0, 0), Action::End, cast(2)]
}

fn run_script(b: &mut Bridge, rng: RngMode) -> (Vec<Snapshot>, Vec<RngEvent>) {
    let mut snaps = vec![b.new_game(&setup(rng)).expect("開局失敗")];
    for a in script() {
        snaps.push(
            b.act(&a)
                .unwrap_or_else(|e| panic!("行動 {a:?} 失敗：{e:?}")),
        );
    }
    (snaps, b.events().expect("取不到亂數事件"))
}

fn ids(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

// R5：橋接程式回報固定的 RosettaStone 版本與套用的 patch
#[test]
fn r5_version_reports_pinned_rosetta_and_patches() {
    let mut b = bridge();
    let v = b.version().expect("取不到版本");
    assert_eq!(v.rosetta_commit, "1690208");
    assert!(!v.patches.is_empty());
}

// I1、I2：照抽牌順序、不換牌、固定先手、後手拿幸運幣
#[test]
fn i1_i2_deck_order_no_mulligan_coin_for_second() {
    let mut b = bridge();
    let s = b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    assert_eq!(s.turn, 1);
    assert_eq!(s.current_player, 1);
    let p1 = &s.players[0];
    let p2 = &s.players[1];
    assert_eq!(
        ids(&p1.hand),
        vec!["CS2_168", "CS2_182", "EX1_277", "CS2_172"]
    );
    assert_eq!(
        ids(&p2.hand),
        vec!["CS2_168", "CS2_168", "CS2_120", "CS2_182", "GAME_005"]
    );
    assert_eq!((p1.mana.available, p1.mana.total), (1, 1));
    assert_eq!((p1.deck_count, p2.deck_count), (26, 26));
}

// I3、S1：每一步之後輸出完整快照
#[test]
fn i3_s1_snapshot_after_action() {
    let mut b = bridge();
    b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    let s = b.act(&play_minion(0, 0)).unwrap();
    let p1 = &s.players[0];
    assert_eq!(p1.hand.len(), 3);
    assert_eq!(p1.mana.available, 0);
    assert_eq!(p1.board.len(), 1);
    let m = &p1.board[0];
    assert_eq!(m.card, "CS2_168");
    assert_eq!((m.attack, m.health, m.max_health), (2, 1, 1));
    assert!(!m.taunt && !m.divine_shield && !m.frozen && !m.stealth && !m.windfury);
    assert_eq!(m.attacks_this_turn, 0);
    assert!(!p1.hero.card.is_empty() && !s.players[1].hero.card.is_empty());
    assert_eq!((p1.hero.health, p1.hero.armor), (30, 0));
    assert!(p1.secrets.is_empty());
    assert_eq!(p1.fatigue, 0);
    assert!(!s.game_over);
}

// I4：不合法的行動回報錯誤，狀態不變
#[test]
fn i4_illegal_action_is_rejected_and_state_unchanged() {
    let mut b = bridge();
    let before = b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    let err = b.act(&play_minion(1, 0)).unwrap_err(); // 冰風雪人 4 費，只有 1 顆水晶
    assert_eq!(err.kind, "illegal_action");
    assert_eq!(b.snapshot().unwrap(), before);
}

// I5：列出目前所有合法行動
#[test]
fn i5_legal_actions_are_listed() {
    let mut b = bridge();
    b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    let opts = b.options().unwrap();
    assert!(opts.contains(&Action::End));
    assert!(opts.contains(&play_minion(0, 0)));
    assert!(opts.contains(&cast(2)));
    assert!(
        !opts
            .iter()
            .any(|a| matches!(a, Action::Play { hand: 1 | 3, .. }))
    );
}

// R1：每次抽亂數都記成事件，結果在參數範圍內
#[test]
fn r1_random_draws_are_recorded_as_events() {
    let mut b = bridge();
    b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    for a in &script()[..3] {
        b.act(a).unwrap();
    }
    let before = b.events().unwrap().len();
    b.act(&cast(2)).unwrap();
    let events = b.events().unwrap();
    assert!(events.len() > before, "施放秘法飛彈應該產生亂數事件");
    for e in &events {
        match e {
            RngEvent::Int { lo, hi, value } => assert!(lo <= value && value <= hi, "{e:?}"),
            RngEvent::Float { lo, hi, value } => assert!(lo <= value && value <= hi, "{e:?}"),
            RngEvent::Shuffle { n, perm } => {
                let mut sorted = perm.clone();
                sorted.sort();
                assert_eq!(sorted, (0..*n).collect::<Vec<_>>(), "{e:?}");
            }
        }
    }
}

// R2：同一個種子記錄兩次，事件序列與快照都相同
#[test]
fn r2_same_seed_same_events() {
    let mut b = bridge();
    let first = run_script(&mut b, RngMode::Record { seed: 7 });
    let second = run_script(&mut b, RngMode::Record { seed: 7 });
    assert_eq!(first, second);
}

// R3：用記錄的序列重播，每一步狀態都和記錄時相同
#[test]
fn r3_replay_reproduces_recorded_states() {
    let mut b = bridge();
    let (recorded, events) = run_script(&mut b, RngMode::Record { seed: 11 });
    let (replayed, _) = run_script(&mut b, RngMode::Replay { events });
    assert_eq!(replayed, recorded);
}

// C4：驗收命令跑完，回報一致的局數
#[test]
fn c4_run_command_reports_consistent_games() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_difftest"))
        .args(["run", "--batch", "A", "--games", "2", "--seed", "1"])
        .output()
        .expect("執行 difftest 失敗");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("一致 2/2"), "{stdout}");
}

// 第 1 階段 B2（橋接程式）：後手第 1 回合使用幸運幣 → 可用法力 2/1
#[test]
fn stage1_b2_coin_adds_one_mana() {
    let mut b = bridge();
    b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    let s = b.act(&Action::End).unwrap();
    let coin = s.players[1]
        .hand
        .iter()
        .position(|c| c == "GAME_005")
        .expect("後手應該有幸運幣");
    let s = b.act(&cast(coin)).unwrap();
    assert_eq!(
        (s.players[1].mana.available, s.players[1].mana.total),
        (2, 1)
    );
}

// 第 1 階段 G4：10/10 時使用幸運幣 → 11（patch 0004，照現行規則）
#[test]
fn stage1_g4_coin_exceeds_ten() {
    let mut b = bridge();
    let mut s = b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    while s.turn < 20 {
        s = b.act(&Action::End).unwrap();
    }
    let p2 = &s.players[1];
    assert_eq!((p2.mana.available, p2.mana.total), (10, 10));
    let coin = p2
        .hand
        .iter()
        .position(|c| c == "GAME_005")
        .expect("幸運幣還在");
    let s = b.act(&cast(coin)).unwrap();
    assert_eq!(
        (s.players[1].mana.available, s.players[1].mana.total),
        (11, 10)
    );
}

// 第 1 階段 B5（橋接程式）：快照的疲勞累計照實回報（第 1 次 1、第 2 次 2）
#[test]
fn stage1_b5_snapshot_reports_fatigue() {
    let mut b = bridge();
    let mut s = b.new_game(&setup(RngMode::Record { seed: 7 })).unwrap();
    while s.players[1].hero.health == 30 {
        s = b.act(&Action::End).unwrap();
    }
    assert_eq!((s.players[1].hero.health, s.players[1].fatigue), (29, 1));
    s = b.act(&Action::End).unwrap();
    assert_eq!((s.players[0].hero.health, s.players[0].fatigue), (29, 1));
    s = b.act(&Action::End).unwrap();
    assert_eq!((s.players[1].hero.health, s.players[1].fatigue), (27, 2));
}

// 第 1 階段 C3（橋接程式）：剛出場的衝刺手下，合法行動裡沒有攻擊英雄
#[test]
fn stage1_c3_rush_options_exclude_hero() {
    let mut b = bridge();
    let rush = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CS3_038"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    b.new_game(&rush).unwrap();
    b.act(&Action::End).unwrap();
    b.act(&Action::End).unwrap();
    b.act(&play_minion(0, 0)).unwrap();
    let attacks: Vec<Action> = b
        .options()
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a, Action::Attack { .. }))
        .collect();
    assert!(attacks.is_empty(), "{attacks:?}");
}

// 第 1 階段 G2、G3：Emberstone 和 RosettaStone 在純關鍵字卡池上逐步一致
#[test]
fn stage1_g2_emberstone_matches_rosetta() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_difftest"))
        .args([
            "run",
            "--engine",
            "emberstone",
            "--pool",
            "keywords",
            "--games",
            "3",
            "--seed",
            "1",
        ])
        .output()
        .expect("執行 difftest 失敗");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("一致 3/3"), "{stdout}");
}
