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

#[test]
fn test_config_parsing_and_coinbase_output_constraint() {
    let config = PleblotteryConfig::from_file("tests/test_data/good_config.toml")
        .expect("Failed to parse config");

    assert!(
        config
            .template_distribution_config
            .mining_server_config
            .is_some(),
        "Mining server config should be present in the template distribution config"
    );

    assert_eq!(
        config
            .mining_server_config
            .calculate_coinbase_output_constraints(),
        (31, 0),
        "Coinbase output constraints should match the expected values"
    );
}
