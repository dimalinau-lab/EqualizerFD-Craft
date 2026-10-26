use resona_apo::dsp::ResonaDspEngine;

#[test]
fn test_all_50_boss_effects_execute_safely() {
    let effect_names = [
        // 1. Vocal & Harmony (8)
        "Vocoder", "PitchShift", "AutoPitch", "Robot", "Harmony", "Transpose", "GuitarToBass", "SlowGear",
        // 2. Rhythm & Beat (5)
        "Slicer", "Vinyl", "Scatter", "BeatRepeat", "BeatShift",
        // 3. Modulation (8)
        "RingMod", "Chorus", "Flanger", "Phaser", "Tremolo", "Vibrato", "Rotary", "AutoPan",
        // 4. Filters & DJ (8)
        "Isolator", "Lo-Fi", "AutoWah", "Telephone", "LPF", "BPF", "HPF", "Synth",
        // 5. Distortion & Preamps (7)
        "Distortion", "Fuzz", "Overdrive", "Metal", "OctFuzz", "Crunch", "Preamp",
        // 6. Dynamics (3)
        "Compressor", "Limiter", "NoiseSuppressor",
        // 7. Delay & Echo (6)
        "Delay", "PanningDelay", "TapeEcho", "WarpDelay", "TwistDelay", "GranularDelay",
        // 8. Reverb & Space (5)
        "Reverb", "RoomReverb", "HallReverb", "PlateReverb", "SpringReverb",
    ];

    assert_eq!(effect_names.len(), 50, "Must test exactly all 50 Boss RC-505 Mk2 effects");

    let mut engine = ResonaDspEngine::new(48000.0, 2);

    for (_idx, name) in effect_names.iter().enumerate() {
        let directive = format!(
            "BossRC505: Effect \"BOSS RC-505 Mk2: {}\" Params \"%7B%22mix%22%3A50%7D\"\n",
            name
        );
        engine.sync_from_config_text(&directive);

        let mut buf = vec![0.5f32; 1024]; // 512 stereo frames
        let (peak_l, peak_r) = engine.process_interleaved(&mut buf, 2);
        assert!(!peak_l.is_nan() && !peak_r.is_nan(), "Effect {} generated NaN peaks: ({}, {})", name, peak_l, peak_r);

        // Verify no NaNs or Infs generated
        for &s in buf.iter() {
            assert!(!s.is_nan(), "Effect {} generated NaN sample!", name);
            assert!(!s.is_infinite(), "Effect {} generated infinite sample!", name);
        }
    }
}
