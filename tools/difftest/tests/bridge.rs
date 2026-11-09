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

// 橋接程式回報的 patch 清單＝patches 目錄裡全部的 .patch（依檔名排序），新增 patch 不會漏套
#[test]
fn r5_version_patches_match_directory() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rosetta-bridge/patches");
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".patch"))
        .collect();
    names.sort();
    let mut b = bridge();
    assert_eq!(b.version().expect("取不到版本").patches, names);
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

// 第 2 階段 D6（橋接程式，patch）：兩隻不穩定的食屍鬼同時死亡，第一個打死的魚人留在戰場，
// 第二個照樣打到它，白沫狂戰士也照算
#[test]
fn stage2_mortally_wounded_hit_by_later_deathrattle_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CORE_EX1_604", "CS2_168", "CORE_CS2_032"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["FP1_024", "FP1_024"]),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (1, play_minion(1, 0)), // 魚人劫掠者 2/1
        (4, play_minion(0, 0)), // 不穩定的食屍鬼 ×2
        (5, play_minion(0, 1)), // 白沫狂戰士 2/4
        (6, play_minion(0, 1)),
        (13, cast(0)), // 烈焰風暴打死兩隻食屍鬼
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let board: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["CORE_EX1_604"]);
    assert!(s.players[1].board.is_empty());
    // 烈焰風暴 2 隻、兩個死亡之聲各打到狂戰士和魚人：+2、+2、+2
    assert_eq!(s.players[0].board[0].attack, 8);
    assert_eq!(s.players[0].board[0].health, 2);
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

fn run_difftest(args: &[&str]) -> String {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_difftest"))
        .args(args)
        .output()
        .expect("執行 difftest 失敗");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

/// 先手照劇本行動、後手每回合直接結束；回傳每一步的快照。
fn script_first_player(b: &mut Bridge, setup: &GameSetup, plan: &[(i32, Action)]) -> Snapshot {
    let mut s = b.new_game(setup).unwrap();
    for (turn, action) in plan {
        while s.turn < *turn {
            s = b.act(&Action::End).unwrap();
        }
        s = b
            .act(action)
            .unwrap_or_else(|e| panic!("第 {turn} 回合 {action:?} 失敗：{e:?}"));
    }
    s
}

// 第 2 階段 A10（橋接程式，patch）：召喚圖騰候選是灼熱、石爪、風懲、治療；4 種都在也能用
#[test]
fn stage2_a10_totemic_call_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "SHAMAN".into(),
            deck: deck(&[]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let power = Action::Power { target: None };
    let plan: Vec<(i32, Action)> = [3, 5, 7, 9].iter().map(|&t| (t, power.clone())).collect();
    let s = script_first_player(&mut b, &setup, &plan);
    let mut totems: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    totems.sort();
    assert_eq!(totems, vec!["CS2_050", "CS2_051", "CS2_052", "NEW1_009"]);
    let mut s = s;
    while s.turn < 11 {
        s = b.act(&Action::End).unwrap();
    }
    assert!(b.options().unwrap().contains(&power));
    let s = b.act(&power).unwrap();
    assert_eq!(s.players[0].board.len(), 5);
}

// 第 2 階段 D3（橋接程式，patch）：麥田魔像（後下、左）和鬼怪蠕行者（先下、右）一起死 → 蜘蛛、蜘蛛、受損的魔像
#[test]
fn stage2_d3_deathrattle_position_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARLOCK".into(),
            deck: deck(&["FP1_002", "EX1_556", "CS2_062"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (3, play_minion(0, 0)), // 鬼怪蠕行者
        (5, play_minion(0, 0)), // 麥田魔像放在左邊
        (7, cast(0)),           // 地獄烈焰
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let board: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["FP1_002t", "FP1_002t", "skele21"]);
}

// 第 2 階段 F5（橋接程式，patch）：裝備武器的英雄在對手回合攻擊力是 0，被攻擊時不反擊
#[test]
fn stage2_f5_weapon_inactive_on_opponent_turn_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "ROGUE".into(),
            deck: p1_deck(),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    // 第 3 回合先手用匕首專精（1/2 匕首）；後手第 4 回合出魚人劫掠者（2/1），第 6 回合攻擊先手英雄
    let mut s = b.new_game(&setup).unwrap();
    while s.turn < 3 {
        s = b.act(&Action::End).unwrap();
    }
    s = b.act(&Action::Power { target: None }).unwrap();
    assert_eq!(s.players[0].hero.attack, 1);
    s = b.act(&Action::End).unwrap();
    assert_eq!(s.players[0].hero.attack, 0);
    b.act(&play_minion(0, 0)).unwrap();
    b.act(&Action::End).unwrap();
    s = b.act(&Action::End).unwrap();
    let health = s.players[0].hero.health;
    let attack = Action::Attack {
        attacker: emberstone_difftest::actions::Target::Minion {
            player: 2,
            index: 0,
        },
        target: emberstone_difftest::actions::Target::Hero { player: 1 },
    };
    s = b.act(&attack).unwrap();
    assert_eq!(s.players[0].hero.health, health - 2);
    assert_eq!(s.players[1].board.len(), 1, "劫掠者不應該被反擊打死");
}

fn target_minion(player: u8, index: usize) -> emberstone_difftest::actions::Target {
    emberstone_difftest::actions::Target::Minion { player, index }
}

// 第 1 階段 D5（橋接程式，patch）：致命劇毒對英雄沒有特殊效果——英雄拿武器攻擊劇毒手下不會死
#[test]
fn stage2_poisonous_defender_does_not_destroy_hero_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "ROGUE".into(),
            deck: p1_deck(),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["EX1_170"]),
        },
        rng: RngMode::Record { seed: 7 },
    };
    // 後手第 6 回合出帝王眼鏡蛇（2/3 劇毒）；先手第 7 回合用匕首專精，英雄攻擊眼鏡蛇
    let mut s = b.new_game(&setup).unwrap();
    while s.turn < 6 {
        s = b.act(&Action::End).unwrap();
    }
    b.act(&play_minion(0, 0)).unwrap();
    b.act(&Action::End).unwrap();
    b.act(&Action::Power { target: None }).unwrap();
    s = b
        .act(&Action::Attack {
            attacker: emberstone_difftest::actions::Target::Hero { player: 1 },
            target: target_minion(2, 0),
        })
        .unwrap();
    assert!(!s.game_over, "英雄不應該被劇毒消滅");
    assert_eq!(s.players[0].hero.health, 28);
    assert_eq!(s.players[1].board[0].health, 2);
}

// 第 2 階段 D1、E4（橋接程式，patch）：生命被動加成下死亡的手下，死亡之聲只結算一次
#[test]
fn stage2_health_aura_deathrattle_once_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARLOCK".into(),
            deck: deck(&["CORE_EX1_096", "CORE_CS2_222", "CS2_062"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (3, play_minion(0, 0)),  // 掠寶囤積者
        (13, play_minion(0, 1)), // 暴風城勇士：囤積者 3/2
        (15, cast(0)),           // 地獄烈焰：囤積者死亡，抽 1 張
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    // 第 13 回合出完勇士剩 8 張；第 15 回合抽 1、打出地獄烈焰、死亡之聲抽 1
    assert_eq!(s.players[0].hand.len(), 9);
}

// 第 2 階段 E2、E6（橋接程式，patch）：本回合 +2 攻擊加在被動加成之上，不重複計算被動加成
#[test]
fn stage2_one_turn_buff_under_aura_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARLOCK".into(),
            deck: deck(&["CORE_CS2_122", "CS2_120", "CORE_CS2_188"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let sergeant = Action::Play {
        hand: 0,
        pos: Some(2),
        target: Some(target_minion(1, 1)),
        choose_one: 0,
    };
    let plan = vec![
        (13, play_minion(0, 0)), // 團隊領袖
        (13, play_minion(0, 1)), // 淡水鱷 2/3 → 3/3
        (13, sergeant),          // 嚴厲的士官給淡水鱷 +2
    ];
    let mut s = script_first_player(&mut b, &setup, &plan);
    assert_eq!(s.players[0].board[1].attack, 5);
    while s.turn < 15 {
        s = b.act(&Action::End).unwrap();
    }
    assert_eq!(s.players[0].board[1].attack, 3);
}

// 第 2 階段 E2、E5（橋接程式，patch）：永久 +2/+3 加在生命被動加成之上，不重複計算被動加成
#[test]
fn stage2_permanent_buff_under_aura_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "DRUID".into(),
            deck: deck(&["CORE_CS2_222", "CS2_120", "CORE_CS2_009"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let mark = Action::Play {
        hand: 0,
        pos: None,
        target: Some(target_minion(1, 1)),
        choose_one: 0,
    };
    let plan = vec![
        (17, play_minion(0, 0)), // 暴風城勇士
        (17, play_minion(0, 1)), // 淡水鱷 2/3 → 3/4
        (19, mark),              // 野性印記 +2/+3
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let croc = &s.players[0].board[1];
    assert_eq!((croc.attack, croc.health), (5, 7));
}

// 第 2 階段 I2（橋接程式，patch）：寒冰護體只在對手回合觸發——自己的手下攻擊敵方英雄時不觸發
#[test]
fn stage2_ice_barrier_not_on_own_turn_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CS2_120", "CORE_EX1_289"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (3, play_minion(0, 0)), // 淡水鱷
        (5, cast(0)),           // 寒冰護體
        (
            7,
            Action::Attack {
                attacker: target_minion(1, 0),
                target: emberstone_difftest::actions::Target::Hero { player: 2 },
            },
        ),
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    assert_eq!(s.players[0].hero.armor, 0);
    assert_eq!(s.players[0].secrets.len(), 1);
}

// 第 2 階段 H2（橋接程式，patch）：沉默後目前生命只在超過新上限時才降——受傷的手下不會因此變成 0 生命
#[test]
fn stage2_silence_keeps_current_health_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARLOCK".into(),
            deck: deck(&["CS2_120", "CS2_004", "CS2_062", "CORE_CS2_203"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let shield = Action::Play {
        hand: 0,
        pos: None,
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let owl = Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let plan = vec![
        (3, play_minion(0, 0)), // 淡水鱷 2/3
        (5, shield),            // 真言術：盾 → 2/5
        (9, cast(0)),           // 地獄烈焰 → 2/2（上限 5）
        (11, owl),              // 鐵喙貓頭鷹沉默淡水鱷 → 上限 3，目前 2
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let croc = &s.players[0].board[0];
    assert_eq!((croc.attack, croc.health), (2, 2));
}

// 第 2 階段 F4a（patch）：瘋狂炸彈客不會打到已經被打到 0 生命的英雄（種子 84、116 曾因此不同步）
#[test]
fn stage2_mad_bomber_skips_dead_hero() {
    for seed in ["84", "116"] {
        let out = run_difftest(&[
            "run",
            "--engine",
            "emberstone",
            "--pool",
            "stage2",
            "--powers",
            "--games",
            "1",
            "--seed",
            seed,
        ]);
        assert!(!out.contains("Desync"), "{out}");
    }
}

// 第 2 階段 E7（橋接程式，patch）：血蹄勇戰者受傷時 +3 攻擊，回滿就失效
#[test]
fn stage2_enrage_ends_when_healed_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "PRIEST".into(),
            deck: deck(&["CORE_OG_218", "CORE_CS2_189"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let archer = Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let heal = Action::Power {
        target: Some(target_minion(1, 0)),
    };
    let plan = vec![
        (7, play_minion(0, 0)), // 血蹄勇戰者 2/6
        (9, archer),            // 精靈弓箭手打 1 點 → 5/5
        (9, heal),              // 次級治療術回滿 → 2/6
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let brave = &s.players[0].board[0];
    assert_eq!((brave.attack, brave.health), (2, 6));
}

// 第 2 階段 I3a、I3b（patch）：冰凍陷阱後爆炸陷阱照樣觸發（種子 15、51、238）；
// 爆炸陷阱打死攻擊方英雄時遊戲直接結束、攻擊不發生（種子 34、329）
#[test]
fn stage2_attack_secrets_seeds_consistent() {
    for seed in ["15", "51", "238", "34", "329"] {
        let out = run_difftest(&[
            "run",
            "--engine",
            "emberstone",
            "--pool",
            "stage2",
            "--powers",
            "--games",
            "1",
            "--seed",
            seed,
        ]);
        assert!(out.contains("一致 1/1"), "種子 {seed}：{out}");
    }
}

/// 結束回合直到第 turn 回合。
fn end_until(b: &mut Bridge, mut s: Snapshot, turn: i32) -> Snapshot {
    while s.turn < turn {
        s = b.act(&Action::End).unwrap();
    }
    s
}

/// 手牌中最後一張 id 的位置。
fn hand_pos(s: &Snapshot, player: usize, id: &str) -> usize {
    s.players[player]
        .hand
        .iter()
        .rposition(|c| c == id)
        .unwrap_or_else(|| panic!("手牌沒有 {id}"))
}

fn hunter_with_freezing_trap(p1_head: &[&str], p2_head: &[&str]) -> GameSetup {
    GameSetup {
        p1: PlayerSetup {
            class: "HUNTER".into(),
            deck: deck(p1_head),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(p2_head),
        },
        rng: RngMode::Record { seed: 7 },
    }
}

// 第 2 階段 E6、I4（橋接程式，patch）：有本回合 +2 的手下被冰凍陷阱送回手牌，回合結束不再扣它的攻擊力
#[test]
fn stage2_returned_minion_loses_one_turn_buff_on_bridge() {
    let mut b = bridge();
    let setup = hunter_with_freezing_trap(&["CORE_EX1_611"], &["CS2_120", "CORE_CS2_188"]);
    let s = b.new_game(&setup).unwrap();
    let mut s = end_until(&mut b, s, 3);
    b.act(&cast(0)).unwrap(); // 冰凍陷阱
    s = end_until(&mut b, s, 4);
    b.act(&play_minion(0, 0)).unwrap(); // 淡水鱷
    end_until(&mut b, s, 6);
    b.act(&Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(2, 0)),
        choose_one: 0,
    })
    .unwrap(); // 嚴厲的士官：淡水鱷 +2
    s = b
        .act(&Action::Attack {
            attacker: target_minion(2, 0),
            target: emberstone_difftest::actions::Target::Hero { player: 1 },
        })
        .unwrap(); // 冰凍陷阱送回手牌
    s = end_until(&mut b, s, 8);
    let croc = hand_pos(&s, 1, "CS2_120");
    s = b.act(&play_minion(croc, 0)).unwrap();
    let m = &s.players[1].board[0];
    assert_eq!((m.card.as_str(), m.attack), ("CS2_120", 2));
}

// 第 2 階段 H1、I4（橋接程式，patch）：被沉默的手下回到手牌後重新打出，死亡之聲恢復
#[test]
fn stage2_returned_minion_loses_silence_on_bridge() {
    let mut b = bridge();
    let setup =
        hunter_with_freezing_trap(&["CORE_EX1_611", "CS2_062"], &["EX1_556", "CORE_CS2_203"]);
    let s = b.new_game(&setup).unwrap();
    let mut s = end_until(&mut b, s, 3);
    b.act(&cast(0)).unwrap(); // 冰凍陷阱
    s = end_until(&mut b, s, 6);
    b.act(&play_minion(0, 0)).unwrap(); // 麥田魔像
    end_until(&mut b, s, 8);
    b.act(&Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(2, 0)),
        choose_one: 0,
    })
    .unwrap(); // 鐵喙貓頭鷹沉默麥田魔像
    s = b
        .act(&Action::Attack {
            attacker: target_minion(2, 0),
            target: emberstone_difftest::actions::Target::Hero { player: 1 },
        })
        .unwrap(); // 冰凍陷阱送回手牌
    s = end_until(&mut b, s, 10);
    let golem = hand_pos(&s, 1, "EX1_556");
    b.act(&play_minion(golem, 0)).unwrap();
    end_until(&mut b, s, 11);
    s = b.act(&cast(0)).unwrap(); // 地獄烈焰：麥田魔像和貓頭鷹死亡
    let board: Vec<&str> = s.players[1].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["skele21"]);
}

// 第 2 階段 E7、H1（橋接程式，patch）：沉默受傷的血蹄勇戰者，回到原始攻擊力 2
// 第 2 階段 I3c（橋接程式，patch）：秘密觸發後先進入死亡階段：爆炸陷阱打死憎惡，
// 憎惡的死亡之聲打死攻擊者，攻擊取消
#[test]
fn stage2_death_phase_before_combat_on_bridge() {
    let mut b = bridge();
    let setup = hunter_with_freezing_trap(&["CS2_120", "CORE_EX1_610"], &["CS2_120", "EX1_097"]);
    let s = b.new_game(&setup).unwrap();
    let s = end_until(&mut b, s, 3);
    b.act(&play_minion(0, 0)).unwrap(); // 先手淡水鱷
    let s = end_until(&mut b, s, 4);
    b.act(&play_minion(0, 0)).unwrap(); // 後手淡水鱷
    let s = end_until(&mut b, s, 10);
    b.act(&play_minion(0, 1)).unwrap(); // 後手憎惡 4/4
    end_until(&mut b, s, 11);
    b.act(&Action::Attack {
        attacker: target_minion(1, 0),
        target: target_minion(2, 1),
    })
    .unwrap(); // 先手淡水鱷撞憎惡 → 憎惡 4/2
    let s = b.act(&cast(0)).unwrap(); // 爆炸陷阱
    end_until(&mut b, s, 12);
    let s = b
        .act(&Action::Attack {
            attacker: target_minion(2, 0),
            target: emberstone_difftest::actions::Target::Hero { player: 1 },
        })
        .unwrap();
    assert!(s.players[1].board.is_empty());
    assert_eq!(s.players[0].hero.health, 28);
    // 後手英雄吃到爆炸陷阱 2 點和憎惡 2 點
    assert_eq!(s.players[1].hero.health, 26);
}

// 第 2 階段 E7、H1（橋接程式，patch）：沉默受傷的血蹄勇戰者，回到原始攻擊力 2
#[test]
fn stage2_silence_enraged_minion_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["CORE_OG_218", "CORE_CS2_189", "CORE_CS2_203"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let archer = Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let owl = Action::Play {
        hand: 0,
        pos: Some(2),
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let plan = vec![
        (7, play_minion(0, 0)), // 血蹄勇戰者 2/6
        (9, archer),            // 精靈弓箭手打 1 點 → 5/5
        (9, owl),               // 鐵喙貓頭鷹沉默 → 2/5
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let brave = &s.players[0].board[0];
    assert_eq!((brave.attack, brave.health), (2, 5));
}

fn replay(events: Vec<emberstone_difftest::rng::RngEvent>) -> RngMode {
    RngMode::Replay { events }
}

fn int(lo: i64, hi: i64, value: i64) -> emberstone_difftest::rng::RngEvent {
    emberstone_difftest::rng::RngEvent::Int { lo, hi, value }
}

// 第 2 階段 C6（橋接程式）：瘋狂炸彈客打死暴風城勇士後先進入死亡階段，爆炸符文觸發時勇士的 +1/+1 已經不在
#[test]
fn stage2_death_phase_before_after_play_secret_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARLOCK".into(),
            deck: deck(&["CORE_CS2_222", "CS2_062", "CS2_062", "CORE_EX1_082"]),
        },
        p2: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CORE_LOOT_101"]),
        },
        // 飛彈：勇士（候選：勇士、先手英雄、後手英雄）→ 後手英雄 → 後手英雄
        rng: replay(vec![int(0, 2, 0), int(0, 1, 1), int(0, 1, 1)]),
    };
    let plan = vec![
        (13, play_minion(0, 0)), // 暴風城勇士 7/7
        (14, cast(0)),           // 後手爆炸符文（勇士上場後才打出）
        (15, cast(0)),           // 地獄烈焰 ×2：勇士 7/1，雙方英雄各 -6
        (15, cast(0)),
        (17, play_minion(0, 1)), // 瘋狂炸彈客 3/2＋1/1
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    assert!(s.players[0].board.is_empty());
    // 勇士已移出戰場，炸彈客 3/2 吃 6 點，溢出 4 點
    assert_eq!(s.players[0].hero.health, 30 - 6 - 4);
    assert_eq!(s.players[1].hero.health, 30 - 6 - 2);
    assert!(s.players[1].secrets.is_empty(), "爆炸符文應該已經觸發");
}

// 第 2 階段 I4a（橋接程式，patch）：爆炸符文打到聖盾手下，聖盾只吸收等於手下生命的傷害，其餘打英雄
#[test]
fn stage2_explosive_runes_divine_shield_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["EX1_020"]),
        },
        p2: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CORE_LOOT_101"]),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (6, cast(0)),           // 後手爆炸符文
        (7, play_minion(0, 0)), // 血色十字軍 3/1 聖盾術
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    assert_eq!(s.players[0].hero.health, 25);
    let crusader = &s.players[0].board[0];
    assert_eq!((crusader.health, crusader.divine_shield), (1, false));
}

// 第 2 階段 F4b（橋接程式，patch）：雙方英雄都剩 1 血，瘋狂炸彈客前兩發打死雙方，第三發不造成傷害
#[test]
fn stage2_mad_bomber_no_targets_left_on_bridge() {
    let mut b = bridge();
    let mut head = vec!["CS2_062"; 9];
    head.push("CORE_EX1_082");
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARLOCK".into(),
            deck: deck(&head),
        },
        p2: PlayerSetup {
            class: "WARLOCK".into(),
            deck: p2_deck(),
        },
        // 飛彈：後手英雄（候選：先手英雄、後手英雄）→ 先手英雄；第三發沒有候選
        rng: replay(vec![int(0, 1, 1), int(0, 0, 0)]),
    };
    let mut plan = Vec::new();
    for turn in [7, 9, 11, 13, 15, 15, 17, 17] {
        plan.push((turn, cast(0))); // 地獄烈焰 ×8：雙方英雄各 6 血
    }
    plan.push((18, Action::Power { target: None })); // 後手生命分流：4 血
    plan.push((19, cast(0))); // 地獄烈焰：先手 3、後手 1
    plan.push((19, Action::Power { target: None })); // 先手生命分流：1 血
    plan.push((19, play_minion(0, 0))); // 瘋狂炸彈客
    let s = script_first_player(&mut b, &setup, &plan);
    assert!(s.game_over);
    assert_eq!((s.players[0].hero.health, s.players[1].hero.health), (0, 0));
}

// 第 1 階段 D9a（橋接程式）：風怒手下攻擊一次後被凍結，還有一次攻擊 → 本回合結束就解凍
#[test]
fn stage1_d9a_windfury_frozen_thaws_this_turn_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["EX1_033"]),
        },
        p2: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CS2_033"]),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (8, play_minion(0, 0)),  // 後手水元素 3/6 凍結
        (11, play_minion(0, 0)), // 先手風怒鷹身人 4/5 風怒
        (
            13,
            Action::Attack {
                attacker: target_minion(1, 0),
                target: target_minion(2, 0),
            },
        ),
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    assert!(s.players[0].board[0].frozen);
    let s = end_until(&mut b, s, 15);
    assert!(!s.players[0].board[0].frozen);
}

// 第 1 階段 B3a（橋接程式）：超載超過水晶上限時可用法力是負的，但 0 費的牌照樣能打出
// 第 2 階段 H1（橋接程式，patch）：沉默移除劇毒——被沉默的帝王眼鏡蛇不會消滅攻擊它的手下
#[test]
fn stage2_silence_removes_poisonous_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CS2_120", "CORE_CS2_203"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: deck(&["EX1_170"]),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let owl = Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(2, 0)),
        choose_one: 0,
    };
    let plan = vec![
        (3, play_minion(0, 0)), // 淡水鱷 2/3
        (6, play_minion(0, 0)), // 後手帝王眼鏡蛇 2/3 劇毒
        (7, owl),               // 鐵喙貓頭鷹沉默眼鏡蛇
        (
            7,
            Action::Attack {
                attacker: target_minion(1, 0),
                target: target_minion(2, 0),
            },
        ), // 淡水鱷攻擊眼鏡蛇
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let croc = &s.players[0].board[0];
    assert_eq!((croc.card.as_str(), croc.health), ("CS2_120", 1));
}

// 第 1 階段 B3a（橋接程式）：超載超過水晶上限時可用法力是負的，但 0 費的牌照樣能打出
// 第 2 階段 E4＋E7（橋接程式，patch）：暴風城勇士死亡、生命上限降回去後剛好回滿的血蹄勇戰者，受傷加成立刻失效
#[test]
fn stage2_health_aura_removed_ends_enrage_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&[
                "CORE_CS2_222",
                "CORE_OG_218",
                "CORE_CS2_189",
                "CORE_CS2_029",
                "CORE_CS2_189",
            ]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let archer = Action::Play {
        hand: 0,
        pos: Some(2),
        target: Some(target_minion(1, 1)),
        choose_one: 0,
    };
    let fireball = Action::Play {
        hand: 0,
        pos: None,
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let plan = vec![
        (2, play_minion(0, 0)),  // 後手魚人劫掠者 2/1
        (13, play_minion(0, 0)), // 暴風城勇士
        (15, play_minion(0, 1)), // 血蹄勇戰者 3/7
        (15, archer),            // 精靈弓箭手打勇戰者 1 點 → 6/6（受傷 +3）
        (17, fireball),          // 火球術：勇士 7/1
        (
            17,
            Action::Attack {
                attacker: target_minion(1, 0),
                target: target_minion(2, 0),
            },
        ), // 勇士撞劫掠者，在戰鬥中死亡
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let brave = &s.players[0].board[0];
    assert_eq!(brave.card, "CORE_OG_218");
    // 上限 7 → 6、目前 6：回滿，受傷加成失效
    assert_eq!((brave.attack, brave.health), (2, 6));
}

// 第 1 階段 B3a（橋接程式）：超載超過水晶上限時可用法力是負的，但 0 費的牌照樣能打出
// 第 2 階段 F1b（橋接程式，patch）：變形後保留原本的上場順序（使用者實測）
// 盤面 [魚人劫掠者→綿羊, 鬼怪蠕行者, 冰風雪人]，地獄烈焰：綿羊先移除 → 蜘蛛放在最左邊
#[test]
fn stage2_transform_keeps_play_order_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["CS2_168", "FP1_002", "CS2_182", "CS2_022", "CS2_062"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let polymorph = Action::Play {
        hand: 0,
        pos: None,
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let plan = vec![
        (1, play_minion(0, 0)), // 魚人劫掠者
        (3, play_minion(0, 1)), // 鬼怪蠕行者
        (7, play_minion(0, 2)), // 冰風雪人
        (9, polymorph),         // 變形術：劫掠者變成綿羊
        (11, cast(0)),          // 地獄烈焰
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let board: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["FP1_002t", "FP1_002t", "CS2_182"]);
    assert_eq!(s.players[0].board[2].health, 2); // 冰風雪人 4/5 吃 3 點
}

// 第 2 階段 F1（橋接程式，patch）：變形後原本的效果也跟著消失——被變成綿羊的治療圖騰不再於回合結束時治療
#[test]
fn stage2_transform_removes_trigger_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["NEW1_009", "CS2_182", "CS2_022"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let polymorph = Action::Play {
        hand: 0,
        pos: None,
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let ping = Action::Power {
        target: Some(target_minion(1, 1)),
    };
    let plan = vec![
        (1, play_minion(0, 0)), // 治療圖騰
        (7, play_minion(0, 1)), // 冰風雪人
        (11, polymorph),        // 變形術：圖騰變成綿羊
        (11, ping),             // 火焰衝擊打自己的雪人：4/4
        (12, Action::End),
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let board: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["CS2_tk1", "CS2_182"]);
    assert_eq!(s.players[0].board[1].health, 4);
}

// 第 2 階段 H1（橋接程式，patch）：剛打出的衝鋒手下被沉默就失去衝鋒，這回合不能攻擊
#[test]
fn stage2_silenced_charge_minion_cannot_attack_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "MAGE".into(),
            deck: deck(&["VAN_EX1_067", "CORE_CS2_203"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let owl = Action::Play {
        hand: 0,
        pos: Some(1),
        target: Some(target_minion(1, 0)),
        choose_one: 0,
    };
    let plan = vec![
        (17, play_minion(0, 0)), // 銀白指揮官（衝鋒、聖盾術）
        (17, owl),               // 鐵喙貓頭鷹沉默它
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let board: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["VAN_EX1_067", "CORE_CS2_203"]);
    let attacks: Vec<Action> = b
        .options()
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a, Action::Attack { .. }))
        .collect();
    assert!(attacks.is_empty(), "{attacks:?}");
}

// 第 1 階段 B3a（橋接程式）：超載超過水晶上限時可用法力是負的，但 0 費的牌照樣能打出
#[test]
fn stage1_b3a_zero_cost_with_negative_mana_on_bridge() {
    let mut b = bridge();
    let setup = GameSetup {
        p1: PlayerSetup {
            class: "SHAMAN".into(),
            deck: deck(&["EX1_243", "EX1_243", "CS2_231"]),
        },
        p2: PlayerSetup {
            class: "WARRIOR".into(),
            deck: p2_deck(),
        },
        rng: RngMode::Record { seed: 7 },
    };
    let plan = vec![
        (3, play_minion(0, 0)), // 塵魔 ×2：超載 4
        (3, play_minion(0, 1)),
        (5, play_minion(0, 2)), // 上限 3、鎖 4 → 可用 −1；小精靈 0 費
    ];
    let s = script_first_player(&mut b, &setup, &plan);
    let board: Vec<&str> = s.players[0].board.iter().map(|m| m.card.as_str()).collect();
    assert_eq!(board, vec!["EX1_243", "EX1_243", "CS2_231"]);
}

// 第 2 階段 G1：純關鍵字卡池加上英雄技能，Emberstone 和 RosettaStone 逐步一致
#[test]
fn stage2_g1_keywords_with_powers() {
    let out = run_difftest(&[
        "run",
        "--engine",
        "emberstone",
        "--pool",
        "keywords",
        "--powers",
        "--games",
        "3",
        "--seed",
        "1",
    ]);
    assert!(out.contains("一致 3/3"), "{out}");
}

// 第 2 階段 G2：範例卡池（加上純關鍵字卡補滿），Emberstone 和 RosettaStone 逐步一致
#[test]
fn stage2_g2_example_pool() {
    let out = run_difftest(&[
        "run",
        "--engine",
        "emberstone",
        "--pool",
        "stage2",
        "--powers",
        "--games",
        "3",
        "--seed",
        "1",
    ]);
    assert!(out.contains("一致 3/3"), "{out}");
}
