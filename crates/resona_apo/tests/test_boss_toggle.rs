use resona_apo::dsp::ResonaDspEngine;

#[test]
fn test_boss_rc505_toggle() {
    let mut engine = ResonaDspEngine::new(48000.0, 2);

    // 1. Process with enabled Slicer effect
    let enabled_cfg = r#"
BossRC505: Effect "BOSS RC-505 Mk2: Slicer (Beat Repeat)" Params "%7B%22rate%22%3A120%2C%22pattern%22%3A%221%2F8%20Note%22%2C%22depth%22%3A100%2C%22duty%22%3A50%7D"
"#;
    engine.sync_from_config_text(enabled_cfg);
    let mut buf_enabled = vec![0.8f32; 48000]; // 1 second of audio (120 BPM slicer will slice many beats)
    let _ = engine.process_interleaved(&mut buf_enabled, 2);

    let any_sliced = buf_enabled.iter().any(|&s| s.abs() < 0.1);
    assert!(any_sliced, "Slicer should have sliced audio when enabled");

    // 2. Process with disabled Slicer effect (# [Disabled])
    let disabled_cfg = r#"
# [Disabled] BossRC505: Effect "BOSS RC-505 Mk2: Slicer (Beat Repeat)" Params "%7B%22rate%22%3A120%2C%22pattern%22%3A%221%2F8%20Note%22%2C%22depth%22%3A100%2C%22duty%22%3A50%7D"
"#;
    engine.sync_from_config_text(disabled_cfg);
    let mut buf_disabled = vec![0.8f32; 4800];
    let original = buf_disabled.clone();
    let _ = engine.process_interleaved(&mut buf_disabled, 2);

    assert_eq!(buf_disabled, original, "When Boss RC-505 is disabled (# [Disabled]), buffer must be completely unaltered");
}
