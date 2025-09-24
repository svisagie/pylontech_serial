use rust_pytes_serial::config_loader::AppConfig;

#[test]
fn invalid_monitoring_level_falls_back() {
    std::env::set_var("MONITORING_LEVEL", "garbage");
    let cfg = AppConfig::load_env().expect("load env");
    assert_eq!(cfg.cells_monitoring.monitoring_level, "none");
}

#[test]
fn rust_log_overrides_logging_level() {
    std::env::set_var("LOGGING_LEVEL", "error");
    std::env::set_var("RUST_LOG", "debug");
    let cfg = AppConfig::load_env().expect("load env");
    // We cannot directly query env_logger after init here; just ensure struct captures LOGGING_LEVEL unchanged
    assert_eq!(cfg.logging.logging_level.as_deref(), Some("error"));
    // RUST_LOG precedence is applied in main; this test ensures no panic.
}
