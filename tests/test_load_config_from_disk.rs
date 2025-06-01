use pleblottery::config::PleblotteryConfig;

fn config_path(name: &str) -> std::path::PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "test_data", name]
        .iter()
        .collect()
}

#[test]
fn test_good_config() {
    let config = PleblotteryConfig::from_file(config_path("good_config.toml"))
        .expect("Should load good config");
    assert!(config.mining_server_config.listening_port > 0);
    assert!(!config
        .mining_server_config
        .coinbase_output_script
        .is_empty());
    assert_eq!(
        config
            .mining_server_config
            .coinbase_output_script
            .to_hex_string(),
        "001471c73b2276f42a7be9d4c3f68f3b3e43044b6481"
    )
}

#[test]
#[should_panic(expected = "Invalid coinbase output address")]
fn test_bad_address() {
    let _ = PleblotteryConfig::from_file(config_path("bad_address.toml")).unwrap();
}
