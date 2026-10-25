use resona_apo::dsp::ResonaDspEngine;

#[test]
fn test_live_vst_chain() {
    let cfg = std::fs::read_to_string(r"d:\testik\config.txt").expect("read config.txt");
    println!("Config text:\n{}", cfg);

    let mut engine = ResonaDspEngine::new(48000.0, 2);
    engine.sync_from_config_text(&cfg);

    println!("Loaded VST count: {}", engine.vst_plugins.len());
    for (i, p) in engine.vst_plugins.iter().enumerate() {
        println!("  VST #{}: path='{}', id='{}', enabled={}", i, p.library_path, p.vst_id, p.enabled);
    }

    assert!(!engine.vst_plugins.is_empty(), "VST plugins should not be empty!");

    // Generate test audio (quiet noise and louder voice)
    let frames = 512;
    let mut buffer = vec![0.0f32; frames * 2];
    for f in 0..frames {
        // -50 dB noise (amplitude ~0.003)
        let s = 0.003 * ((f % 17) as f32 / 17.0 - 0.5);
        buffer[f * 2] = s;
        buffer[f * 2 + 1] = s;
    }

    let (in_pk, out_pk) = engine.process_interleaved(&mut buffer, 2);
    println!("Quiet block: in_pk = {:.2} dB, out_pk = {:.2} dB", in_pk, out_pk);

    // With ReaGate threshold at -38 dB, this -50 dB noise should be gated to -96 dB!
    println!("Max sample after gate: {:.8}", buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max));
}
