//! 狀態比對與隨機對局產生（case C1–C3）。

use emberstone_difftest::actions::Action;
use emberstone_difftest::generate::{OptionSource, random_actions};
use emberstone_difftest::snapshot::{Snapshot, first_difference};

fn snapshot(p1_hand: &[&str], p2_health: i32) -> Snapshot {
    let hand: Vec<String> = p1_hand.iter().map(|s| format!("{s:?}")).collect();
    let json = format!(
        r#"{{
          "turn": 1, "current_player": 1, "game_over": false, "winner": null,
          "players": [
            {{"hero": {{"card": "HERO_08", "health": 30, "armor": 0, "attack": 0, "weapon": null, "power_used": false}},
              "mana": {{"available": 1, "total": 1, "overload_locked": 0, "overload_owed": 0}},
              "hand": [{}],
              "board": [{{"card": "CS2_168", "attack": 2, "health": 1, "max_health": 1, "taunt": false,
                          "divine_shield": false, "frozen": false, "stealth": false, "windfury": false,
                          "attacks_this_turn": 0}}],
              "deck_count": 26, "secrets": [], "fatigue": 0}},
            {{"hero": {{"card": "HERO_01", "health": {p2_health}, "armor": 0, "attack": 0, "weapon": null, "power_used": false}},
              "mana": {{"available": 0, "total": 0, "overload_locked": 0, "overload_owed": 0}},
              "hand": ["GAME_005"], "board": [], "deck_count": 26, "secrets": [], "fatigue": 0}}
          ]
        }}"#,
        hand.join(", ")
    );
    serde_json::from_str(&json).expect("快照 JSON 格式錯誤")
}

// C1：兩邊快照完全相同 → 一致
#[test]
fn c1_identical_sequences_have_no_difference() {
    let a = vec![snapshot(&["CS2_182"], 30), snapshot(&[], 28)];
    let b = a.clone();
    assert_eq!(first_difference(&a, &b), None);
}

// C2：第 n 步不同 → 回報第一個不同的步數與欄位
#[test]
fn c2_first_difference_reports_step_and_fields() {
    let a = vec![
        snapshot(&["CS2_182"], 30),
        snapshot(&[], 28),
        snapshot(&["EX1_277"], 25),
        snapshot(&[], 20),
    ];
    let mut b = a.clone();
    b[2] = snapshot(&["CS2_172"], 24);
    b[3] = snapshot(&["CS2_120"], 1);
    let diff = first_difference(&a, &b).expect("應該有差異");
    assert_eq!(diff.step, 2);
    assert_eq!(
        diff.fields,
        vec![
            "players[0].hand".to_string(),
            "players[1].hero.health".to_string()
        ]
    );
}

/// 假的合法行動來源：每步提供 3 個出牌選項加結束回合，第 10 步後遊戲結束。
struct FakeGame {
    step: usize,
    offered: Vec<Vec<Action>>,
}

impl OptionSource for FakeGame {
    fn options(&mut self) -> Vec<Action> {
        let mut opts: Vec<Action> = (0..3)
            .map(|i| Action::Play {
                hand: i + self.step,
                pos: None,
                target: None,
                choose_one: 0,
            })
            .collect();
        opts.push(Action::End);
        self.offered.push(opts.clone());
        opts
    }

    fn act(&mut self, _action: &Action) -> bool {
        self.step += 1;
        self.step >= 10
    }
}

fn run(seed: u64) -> (Vec<Action>, Vec<Vec<Action>>) {
    let mut game = FakeGame {
        step: 0,
        offered: Vec::new(),
    };
    let actions = random_actions(seed, &mut game, 100);
    (actions, game.offered)
}

// C3：同一個種子產生同一份行動序列，而且每一步都選自當下的合法行動
#[test]
fn c3_same_seed_same_actions_from_offered_options() {
    let (a, offered) = run(42);
    let (b, _) = run(42);
    assert_eq!(a, b);
    assert_eq!(a.len(), 10);
    for (action, opts) in a.iter().zip(&offered) {
        assert!(opts.contains(action));
    }
}
