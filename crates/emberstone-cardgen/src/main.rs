//! `cargo run -p emberstone-cardgen --bin gen-cards`：由 `data/tw_all.json` 重新產生卡表。

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let read = |rel: &str| {
        std::fs::read_to_string(root.join(rel)).map_err(|e| format!("讀不到 {rel}：{e}"))
    };
    let result = (|| {
        let cards = read("data/tw_all.json")?;
        let sets = read("data/standard_sets.txt")?;
        let generated = emberstone_cardgen::generate(&cards, &sets)?;
        let out = root.join("crates/emberstone-data/src/generated/cards.rs");
        std::fs::create_dir_all(out.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&out, generated.cards_rs)
            .map_err(|e| format!("寫不進 {}：{e}", out.display()))?;
        print!("{}", generated.report);
        Ok::<(), String>(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("產生失敗：{e}");
            ExitCode::FAILURE
        }
    }
}
