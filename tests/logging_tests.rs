use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cubert::logging::{Level, Logger};

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cubert_log_{}_{}", tag, nanos));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_per_strategy_files_are_written() {
    let root = temp_dir("per_strategy");
    let main_log = root.join("trades.log");
    let strategy_dir = root.join("strategies");

    let logger = Logger::new(
        Level::Debug,
        Some(main_log.to_str().unwrap()),
        Some(strategy_dir.to_str().unwrap()),
    )
    .unwrap();

    let stable_id = "bc8f15d5-9fe6-47ee-9389-9c204894e8c2";
    let momentum_id = "4272ba90-6d4b-4814-9e98-a07330860a3c";

    logger.strategy_info(
        stable_id,
        "mean_reversion_stable",
        "BUY COIN (strength: 0.42)",
    );
    logger.strategy_info(momentum_id, "momentum_ev_1", "BUY BETA (strength: 0.12)");
    logger.strategy_debug(stable_id, "mean_reversion_stable", "COIN: z=-2.10");

    let stable =
        fs::read_to_string(strategy_dir.join(format!("mean_reversion_stable_{}.log", stable_id)))
            .unwrap();
    let momentum =
        fs::read_to_string(strategy_dir.join(format!("momentum_ev_1_{}.log", momentum_id)))
            .unwrap();

    assert!(stable.contains("BUY COIN"), "{stable}");
    assert!(stable.contains("z=-2.10"), "{stable}");
    assert!(
        !stable.contains("BETA"),
        "strategies must not bleed into each other: {stable}"
    );
    assert!(momentum.contains("BUY BETA"), "{momentum}");

    let combined = fs::read_to_string(&main_log).unwrap();
    assert!(
        combined.contains("BUY COIN") && combined.contains("BUY BETA"),
        "the combined log must still carry everything: {combined}"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_strategy_name_is_sanitized_into_a_filename() {
    let root = temp_dir("sanitize");
    let strategy_dir = root.join("strategies");

    let logger = Logger::new(
        Level::Info,
        Some(root.join("trades.log").to_str().unwrap()),
        Some(strategy_dir.to_str().unwrap()),
    )
    .unwrap();

    logger.strategy_info("id-1", "../etc/passwd", "escape attempt");
    logger.strategy_info("../../id", "spaces and:colons", "odd name");

    let names: Vec<String> = fs::read_dir(&strategy_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();

    assert!(
        names.contains(&"___etc_passwd_id-1.log".to_string()),
        "path separators must not escape the directory: {names:?}"
    );
    assert!(
        names.contains(&"spaces_and_colons_______id.log".to_string()),
        "a hostile id must be sanitized too: {names:?}"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_strategy_logs_respect_level() {
    let root = temp_dir("level");
    let strategy_dir = root.join("strategies");

    let logger = Logger::new(
        Level::Info,
        Some(root.join("trades.log").to_str().unwrap()),
        Some(strategy_dir.to_str().unwrap()),
    )
    .unwrap();

    logger.strategy_debug("quiet-id", "quiet", "should not appear");

    assert!(
        !strategy_dir.join("quiet_quiet-id.log").exists(),
        "a filtered message must not even create the file"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_strategy_dir_defaults_beside_the_main_log() {
    let root = temp_dir("default_dir");

    let logger = Logger::new(
        Level::Info,
        Some(root.join("trades.log").to_str().unwrap()),
        None,
    )
    .unwrap();
    logger.strategy_info("derived-id", "derived", "hello");

    let derived = root.join("strategies").join("derived_derived-id.log");
    assert!(derived.exists(), "expected {:?} to exist", derived);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_renaming_a_strategy_keeps_one_file_per_id() {
    let root = temp_dir("rename");
    let strategy_dir = root.join("strategies");

    let logger = Logger::new(
        Level::Info,
        Some(root.join("trades.log").to_str().unwrap()),
        Some(strategy_dir.to_str().unwrap()),
    )
    .unwrap();

    logger.strategy_info("id-abc", "original_name", "first");
    logger.strategy_info("id-abc", "renamed_in_streamlit", "second");

    let names: Vec<String> = fs::read_dir(&strategy_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();

    assert_eq!(names.len(), 1, "a rename must not fork the log: {names:?}");

    let body = fs::read_to_string(strategy_dir.join(&names[0])).unwrap();
    assert!(body.contains("first") && body.contains("second"), "{body}");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn test_same_name_different_ids_do_not_collide() {
    let root = temp_dir("collide");
    let strategy_dir = root.join("strategies");

    let logger = Logger::new(
        Level::Info,
        Some(root.join("trades.log").to_str().unwrap()),
        Some(strategy_dir.to_str().unwrap()),
    )
    .unwrap();

    logger.strategy_info("id-one", "same_name", "from one");
    logger.strategy_info("id-two", "same_name", "from two");

    let one = fs::read_to_string(strategy_dir.join("same_name_id-one.log")).unwrap();
    let two = fs::read_to_string(strategy_dir.join("same_name_id-two.log")).unwrap();

    assert!(
        one.contains("from one") && !one.contains("from two"),
        "{one}"
    );
    assert!(two.contains("from two"), "{two}");

    fs::remove_dir_all(&root).ok();
}
