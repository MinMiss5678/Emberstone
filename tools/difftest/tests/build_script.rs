//! 橋接程式建置腳本 tools/rosetta-bridge/build.bat：新增 patch 只要新增 .patch 檔，不必改腳本。

use std::path::PathBuf;

fn bridge_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rosetta-bridge")
}

// build.bat 自動套用 patches/*.patch（依檔名順序），不在腳本裡寫死 patch 檔名
#[test]
fn build_script_does_not_list_patch_names() {
    let script = std::fs::read_to_string(bridge_dir().join("build.bat")).expect("讀不到 build.bat");
    let patches = std::fs::read_dir(bridge_dir().join("patches")).expect("讀不到 patches 目錄");
    for entry in patches {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(!script.contains(&name), "build.bat 寫死了 {name}");
    }
    assert!(
        script.contains("patches\\*.patch"),
        "build.bat 沒有列舉 patches 目錄"
    );
}
