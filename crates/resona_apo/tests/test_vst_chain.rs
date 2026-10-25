use resona_apo::dsp::ResonaDspEngine;

#[test]
fn test_live_vst_chain() {
    let cfg = r#"
# [VSTPlugin: reagate] Params: {"thresh":-38,"attack":2,"hold":60,"release":150,"preopen":2}
VSTPlugin: Library "D:\testik\vst_plugins\reagate-standalone.dll"
"#;

    let mut engine = ResonaDspEngine::new(48000.0, 2);
    engine.sync_from_config_text(cfg);

    assert_eq!(engine.vst_plugins.len(), 1, "Should have 1 active VST plugin");

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

    // With ReaGate threshold at -38 dB, this -50 dB noise should be gated to silence!
    let max_amp = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(max_amp < 1e-6, "ReaGate should have gated quiet noise to complete silence (got max_amp = {})", max_amp);
}
