//! Roland BOSS RC-505 Mk2 Real-Time DSP Audio Processing Engine
//! Zero runtime heap allocation per audio buffer
//! High-fidelity emulation of loop station effects

use crate::dsp::BiquadState;

#[derive(Clone, Debug)]
pub enum BossEffectKind {
    Slicer {
        bpm: f32,
        subdivision: f32, // 1/4 -> 1.0, 1/8 -> 2.0, 1/16 -> 4.0, 1/8T -> 3.0, 1/32 -> 8.0
        depth: f32,       // 0.0 .. 1.0
        duty: f32,        // 0.0 .. 1.0
    },
    Vinyl {
        tone_fc: f32,
        crackle_level: f32,
        flutter_depth: f32,
    },
    Vocoder {
        pitch_st: f32,
        robotic: f32,
        wet: f32,
    },
    RingMod {
        carrier_fc: f32,
        mod_depth: f32,
        mix: f32,
    },
    Isolator {
        low_gain: f32,
        mid_gain: f32,
        high_gain: f32,
        cutoff: f32,
    },
    LoFi {
        drive: f32,
        lowcut: f32,
        highcut: f32,
        noise_level: f32,
    },
    Chorus {
        rate: f32,
        depth: f32,
        feedback: f32,
        mix: f32,
    },
    Distortion {
        drive: f32,
        tone: f32,
        bottom_db: f32,
        out_gain: f32,
    },
    Scatter {
        grain_ms: f32,
        mode: u8,
        density: f32,
        mix: f32,
    },
    Reverb {
        decay_time: f32,
        predelay_ms: f32,
        shimmer: f32,
        mix: f32,
    },
    PitchShift {
        semitones: f32,
        fine_cents: f32,
        mix: f32,
    },
    Flanger {
        rate: f32,
        depth: f32,
        feedback: f32,
        mix: f32,
    },
    Phaser {
        rate: f32,
        depth: f32,
        feedback: f32,
        mix: f32,
    },
    Delay {
        time_ms: f32,
        feedback: f32,
        damp_fc: f32,
        reverse: bool,
        mix: f32,
    },
    AutoWah {
        sensitivity: f32,
        freq_min: f32,
        freq_max: f32,
        resonance_q: f32,
        mix: f32,
    },
    Fuzz {
        drive: f32,
        tone: f32,
        sustain: f32,
        out_gain: f32,
    },
    BeatRepeat {
        bpm: f32,
        subdivision: f32,
        decay: f32,
        mix: f32,
    },
    Telephone {
        drive: f32,
        mix: f32,
    },
    // Complete BOSS RC-505 Mk2 50-FX Suite
    AutoPitch {
        speed: f32,
        mix: f32,
    },
    Robot {
        pitch_st: f32,
        robotic: f32,
        mix: f32,
    },
    Harmony {
        interval: f32,
        mix: f32,
    },
    Transpose {
        semitones: f32,
        mix: f32,
    },
    GuitarToBass {
        gain: f32,
        mix: f32,
    },
    SlowGear {
        attack_ms: f32,
        mix: f32,
    },
    Tremolo {
        rate: f32,
        depth: f32,
        mix: f32,
    },
    Vibrato {
        rate: f32,
        depth: f32,
        mix: f32,
    },
    Rotary {
        speed: f32,
        depth: f32,
        mix: f32,
    },
    AutoPan {
        rate: f32,
        depth: f32,
    },
    FilterLpf {
        cutoff: f32,
        resonance_q: f32,
        mix: f32,
    },
    FilterBpf {
        center_freq: f32,
        resonance_q: f32,
        mix: f32,
    },
    FilterHpf {
        cutoff: f32,
        resonance_q: f32,
        mix: f32,
    },
    Synth {
        wave: u8,
        cutoff: f32,
        mix: f32,
    },
    Overdrive {
        drive: f32,
        tone: f32,
        mix: f32,
    },
    MetalDs {
        gain: f32,
        tone: f32,
        out_gain: f32,
    },
    OctFuzz {
        drive: f32,
        tone: f32,
        out_gain: f32,
    },
    Crunch {
        gain: f32,
        tone: f32,
        out_gain: f32,
    },
    Preamp {
        drive: f32,
        tone: f32,
        out_gain: f32,
    },
    Compressor {
        threshold_db: f32,
        ratio: f32,
        attack_ms: f32,
        release_ms: f32,
        makeup_db: f32,
    },
    Limiter {
        threshold_db: f32,
        ceiling_db: f32,
    },
    NoiseSuppressor {
        threshold_db: f32,
        release_ms: f32,
    },
    PanningDelay {
        time_ms: f32,
        feedback: f32,
        mix: f32,
    },
    TapeEcho {
        time_ms: f32,
        feedback: f32,
        flutter: f32,
        mix: f32,
    },
    WarpDelay {
        time_ms: f32,
        feedback: f32,
        warp: f32,
        mix: f32,
    },
    TwistDelay {
        time_ms: f32,
        feedback: f32,
        twist: f32,
        mix: f32,
    },
    GranularDelay {
        grain_ms: f32,
        feedback: f32,
        mix: f32,
    },
    BeatShift {
        shift_samples: usize,
        mix: f32,
    },
    RoomReverb {
        size: f32,
        damp: f32,
        mix: f32,
    },
    HallReverb {
        size: f32,
        predelay: f32,
        mix: f32,
    },
    PlateReverb {
        decay: f32,
        brightness: f32,
        mix: f32,
    },
    SpringReverb {
        decay: f32,
        tension: f32,
        mix: f32,
    },
}

#[derive(Clone)]
pub struct BossEffectState {
    // Slicer
    pub slicer_phase: f32,
    pub slicer_gain_smooth: f32,

    // Vinyl
    pub vinyl_lp: f32,
    pub rng_state: u32,
    pub vinyl_flutter_phase: f32,

    // Vocoder
    pub vocoder_phase: f32,
    pub vocoder_env: f32,

    // RingMod
    pub ringmod_phase: f32,

    // Isolator
    pub iso_low: BiquadState,
    pub iso_mid: BiquadState,
    pub iso_high: BiquadState,
    pub iso_cutoff: BiquadState,

    // LoFi
    pub lofi_hp: BiquadState,
    pub lofi_lp: BiquadState,

    // Chorus
    pub chorus_buf: Vec<f32>,
    pub chorus_w_idx: usize,
    pub chorus_lfo: f32,

    // Distortion
    pub dist_bottom_filter: BiquadState,
    pub dist_tone_lp: f32,

    // Scatter
    pub scatter_buf: Vec<f32>,
    pub scatter_w_idx: usize,
    pub scatter_grain_pos: usize,
    pub scatter_read_start: usize,
    pub scatter_pitch_phase: f32,
    pub scatter_active: bool,
    pub scatter_freeze_offset: usize,

    // Reverb
    pub rev_comb1: Vec<f32>,
    pub rev_comb1_idx: usize,
    pub rev_comb2: Vec<f32>,
    pub rev_comb2_idx: usize,
    pub rev_comb3: Vec<f32>,
    pub rev_comb3_idx: usize,
    pub rev_comb4: Vec<f32>,
    pub rev_comb4_idx: usize,
    pub rev_ap1: Vec<f32>,
    pub rev_ap1_idx: usize,
    pub rev_ap2: Vec<f32>,
    pub rev_ap2_idx: usize,

    // PitchShift
    pub pitch_buf: Vec<f32>,
    pub pitch_w_idx: usize,
    pub pitch_phase1: f32,
    pub pitch_phase2: f32,

    // Flanger
    pub flanger_buf: Vec<f32>,
    pub flanger_w_idx: usize,
    pub flanger_lfo: f32,

    // Phaser
    pub phaser_ap_x: [f32; 4],
    pub phaser_ap_y: [f32; 4],
    pub phaser_lfo: f32,
    pub phaser_last_out: f32,

    // Delay
    pub delay_buf: Vec<f32>,
    pub delay_w_idx: usize,
    pub delay_lp: f32,

    // AutoWah
    pub wah_env: f32,
    pub wah_filter: BiquadState,

    // Fuzz
    pub fuzz_tone_lp: f32,

    // BeatRepeat
    pub repeat_buf: Vec<f32>,
    pub repeat_w_idx: usize,
    pub repeat_pos: usize,
    pub repeat_slice_len: usize,

    // Telephone
    pub tel_hp: BiquadState,
    pub tel_lp: BiquadState,

    // Additional 50-FX State Variables
    pub tremolo_lfo: f32,
    pub vibrato_lfo: f32,
    pub rotary_lfo: f32,
    pub autopan_lfo: f32,
    pub slowgear_env: f32,
    pub slowgear_gain: f32,
    pub comp_env: f32,
    pub ns_env: f32,
    pub synth_phase: f32,
    pub generic_filter1: BiquadState,
    pub generic_filter2: BiquadState,
    // Robot Voice & Monotone Pitch Tracker (Roland Boss RC-505 / VT-4)
    pub robot_pitch_period: f32,
    pub robot_last_sample: f32,
    pub robot_zc_count: usize,
    pub robot_phase1: f32,
    pub robot_phase2: f32,
    pub robot_carrier_phase: f32,
    pub robot_vocal_env: f32,
}

impl Default for BossEffectState {
    fn default() -> Self {
        Self {
            slicer_phase: 0.0,
            slicer_gain_smooth: 1.0,
            vinyl_lp: 0.0,
            rng_state: 0x12345678,
            vinyl_flutter_phase: 0.0,
            vocoder_phase: 0.0,
            vocoder_env: 0.0,
            ringmod_phase: 0.0,
            iso_low: BiquadState::default(),
            iso_mid: BiquadState::default(),
            iso_high: BiquadState::default(),
            iso_cutoff: BiquadState::default(),
            lofi_hp: BiquadState::default(),
            lofi_lp: BiquadState::default(),
            chorus_buf: vec![0.0; 4096],
            chorus_w_idx: 0,
            chorus_lfo: 0.0,
            dist_bottom_filter: BiquadState::default(),
            dist_tone_lp: 0.0,
            scatter_buf: vec![0.0; 48000],
            scatter_w_idx: 0,
            scatter_grain_pos: 0,
            scatter_read_start: 0,
            scatter_pitch_phase: 0.0,
            scatter_active: true,
            scatter_freeze_offset: 0,
            rev_comb1: vec![0.0; 1116],
            rev_comb1_idx: 0,
            rev_comb2: vec![0.0; 1188],
            rev_comb2_idx: 0,
            rev_comb3: vec![0.0; 1277],
            rev_comb3_idx: 0,
            rev_comb4: vec![0.0; 1356],
            rev_comb4_idx: 0,
            rev_ap1: vec![0.0; 225],
            rev_ap1_idx: 0,
            rev_ap2: vec![0.0; 556],
            rev_ap2_idx: 0,
            pitch_buf: vec![0.0; 4096],
            pitch_w_idx: 0,
            pitch_phase1: 0.0,
            pitch_phase2: 1024.0,
            flanger_buf: vec![0.0; 2048],
            flanger_w_idx: 0,
            flanger_lfo: 0.0,
            phaser_ap_x: [0.0; 4],
            phaser_ap_y: [0.0; 4],
            phaser_lfo: 0.0,
            phaser_last_out: 0.0,
            delay_buf: vec![0.0; 96000],
            delay_w_idx: 0,
            delay_lp: 0.0,
            wah_env: 0.0,
            wah_filter: BiquadState::default(),
            fuzz_tone_lp: 0.0,
            repeat_buf: vec![0.0; 96000],
            repeat_w_idx: 0,
            repeat_pos: 0,
            repeat_slice_len: 24000,
            tel_hp: BiquadState::default(),
            tel_lp: BiquadState::default(),
            tremolo_lfo: 0.0,
            vibrato_lfo: 0.0,
            rotary_lfo: 0.0,
            autopan_lfo: 0.0,
            slowgear_env: 0.0,
            slowgear_gain: 0.0,
            comp_env: 0.0,
            ns_env: 0.0,
            synth_phase: 0.0,
            generic_filter1: BiquadState::default(),
            generic_filter2: BiquadState::default(),
            robot_pitch_period: 240.0,
            robot_last_sample: 0.0,
            robot_zc_count: 0,
            robot_phase1: 0.0,
            robot_phase2: 512.0,
            robot_carrier_phase: 0.0,
            robot_vocal_env: 0.0,
        }
    }
}

pub fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                    result.push(val as char);
                    i += 3;
                    continue;
                }
            }
            result.push('%');
            i += 1;
        } else if bytes[i] == b'+' {
            result.push(' ');
            i += 1;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

pub fn parse_boss_directive(raw_body: &str, sample_rate: f32) -> Option<(BossEffectKind, BossEffectState)> {
    // Expected format: Effect "<title or id>" Params "<url_encoded_json>"
    let eff_str = if let Some(p1) = raw_body.find("Effect \"") {
        let rest = &raw_body[p1 + 8..];
        let p2 = rest.find('"').unwrap_or(rest.len());
        rest[..p2].to_string()
    } else {
        raw_body.to_string()
    };

    let params_str = if let Some(p1) = raw_body.find("Params \"") {
        let rest = &raw_body[p1 + 8..];
        let p2 = rest.find('"').unwrap_or(rest.len());
        rest[..p2].to_string()
    } else {
        String::new()
    };

    let decoded_params = url_decode(&params_str);
    let json: serde_json::Value = serde_json::from_str(&decoded_params).unwrap_or_default();

    let eff_lower = eff_str.to_lowercase();
    let mut state = BossEffectState::default();

    if eff_lower.contains("slicer") || eff_lower.contains("нарез") || (eff_lower.contains("slice") && !eff_lower.contains("beatrepeat") && !eff_lower.contains("beat repeat")) {
        let bpm = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(128.0) as f32;
        let pat_str = json.get("pattern").and_then(|v| v.as_str()).unwrap_or("1/16 Note");
        let subdiv = if pat_str.contains("1/4") {
            1.0
        } else if pat_str.contains("1/8 Triplet") || pat_str.contains("Triplet") {
            3.0
        } else if pat_str.contains("1/8") {
            2.0
        } else if pat_str.contains("1/32") {
            8.0
        } else {
            4.0 // 1/16 default
        };
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(95.0) / 100.0) as f32;
        let duty = (json.get("duty").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;

        Some((BossEffectKind::Slicer { bpm, subdivision: subdiv, depth, duty }, state))
    } else if eff_lower.contains("vinyl") || eff_lower.contains("tape") || eff_lower.contains("пластин") {
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(5500.0) as f32;
        let crackle = (json.get("crackle").and_then(|v| v.as_f64()).unwrap_or(25.0) / 100.0) as f32;
        let flutter = (json.get("flutter").and_then(|v| v.as_f64()).unwrap_or(20.0) / 100.0) as f32;
        Some((BossEffectKind::Vinyl { tone_fc: tone, crackle_level: crackle, flutter_depth: flutter }, state))
    } else if (eff_lower.contains("vocoder") || eff_lower.contains("вокод")) && !eff_lower.contains("robot") && !eff_lower.contains("робот") {
        let pitch = json.get("pitch").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let robotic = (json.get("robotic").and_then(|v| v.as_f64()).unwrap_or(85.0) / 100.0) as f32;
        let wet = (json.get("wet").and_then(|v| v.as_f64()).unwrap_or(80.0) / 100.0) as f32;
        Some((BossEffectKind::Vocoder { pitch_st: pitch, robotic, wet }, state))
    } else if eff_lower.contains("ringmod") || eff_lower.contains("ring") || eff_lower.contains("кольцев") {
        let carrier = json.get("carrier").and_then(|v| v.as_f64()).unwrap_or(440.0) as f32;
        let depth = (json.get("moddepth").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        Some((BossEffectKind::RingMod { carrier_fc: carrier, mod_depth: depth, mix }, state))
    } else if eff_lower.contains("isolator") || eff_lower.contains("dj") || eff_lower.contains("срез") {
        let low_db = json.get("lowkill").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let mid_db = json.get("midkill").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let high_db = json.get("highkill").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let cutoff = json.get("cutoff").and_then(|v| v.as_f64()).unwrap_or(15000.0) as f32;

        state.iso_low.calculate(1, 320.0, low_db, 0.7071, sample_rate); // Low shelf
        state.iso_mid.calculate(0, 1400.0, mid_db, 1.2, sample_rate);   // Peaking
        state.iso_high.calculate(2, 3800.0, high_db, 0.7071, sample_rate); // High shelf
        state.iso_cutoff.calculate(3, cutoff.clamp(100.0, 18000.0), 0.0, 0.7071, sample_rate); // Lowpass

        let low_gain = 10.0f32.powf(low_db / 20.0);
        let mid_gain = 10.0f32.powf(mid_db / 20.0);
        let high_gain = 10.0f32.powf(high_db / 20.0);

        Some((BossEffectKind::Isolator { low_gain, mid_gain, high_gain, cutoff }, state))
    } else if eff_lower.contains("lofi") || eff_lower.contains("radio") || eff_lower.contains("раци") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(45.0) / 100.0) as f32;
        let lowcut = json.get("lowcut").and_then(|v| v.as_f64()).unwrap_or(600.0) as f32;
        let highcut = json.get("highcut").and_then(|v| v.as_f64()).unwrap_or(3500.0) as f32;
        let noise = (json.get("noise").and_then(|v| v.as_f64()).unwrap_or(20.0) / 100.0) as f32;

        state.lofi_hp.calculate(4, lowcut, 0.0, 0.7071, sample_rate);
        state.lofi_lp.calculate(3, highcut, 0.0, 0.7071, sample_rate);

        Some((BossEffectKind::LoFi { drive, lowcut, highcut, noise_level: noise }, state))
    } else if (eff_lower.contains("chorus") || eff_lower.contains("хорус")) && !eff_lower.contains("flanger") && !eff_lower.contains("флэнж") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(1.2) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(15.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(45.0) / 100.0) as f32;

        Some((BossEffectKind::Chorus { rate, depth, feedback, mix }, state))
    } else if (eff_lower.contains("distortion") || eff_lower.contains("дисторш")) && !eff_lower.contains("overdrive") && !eff_lower.contains("овердрайв") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(35.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(4500.0) as f32;
        let bottom = json.get("bottom").and_then(|v| v.as_f64()).unwrap_or(2.0) as f32;
        let level = json.get("level").and_then(|v| v.as_f64()).unwrap_or(-2.0) as f32;

        state.dist_bottom_filter.calculate(1, 120.0, bottom, 0.7071, sample_rate);
        let out_gain = 10.0f32.powf(level / 20.0);

        Some((BossEffectKind::Distortion { drive, tone, bottom_db: bottom, out_gain }, state))
    } else if eff_lower.contains("scatter") || eff_lower.contains("грэйн") || eff_lower.contains("грануляр") {
        let grain = json.get("grain").and_then(|v| v.as_f64()).unwrap_or(120.0) as f32;
        let density = (json.get("density").and_then(|v| v.as_f64()).unwrap_or(70.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;

        let mode_str = json.get("scattertype").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let mode: u8 = if mode_str.contains("питч") || mode_str.contains("pitch") {
            3
        } else if mode_str.contains("реверс") || mode_str.contains("reverse") {
            2
        } else if mode_str.contains("репит") || mode_str.contains("repeat") || mode_str.contains("stutter") {
            1
        } else {
            0
        };

        Some((BossEffectKind::Scatter { grain_ms: grain, mode, density, mix }, state))
    } else if eff_lower.contains("reverb") || eff_lower.contains("space") || eff_lower.contains("реверб") {
        let time = json.get("time").and_then(|v| v.as_f64()).unwrap_or(3.5) as f32;
        let predelay = json.get("predelay").and_then(|v| v.as_f64()).unwrap_or(25.0) as f32;
        let shimmer = (json.get("shimmer").and_then(|v| v.as_f64()).unwrap_or(30.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(25.0) / 100.0) as f32;

        Some((BossEffectKind::Reverb { decay_time: time, predelay_ms: predelay, shimmer, mix }, state))
    } else if (eff_lower.contains("pitch") || eff_lower.contains("питч") || eff_lower.contains("октав")) && !eff_lower.contains("harmony") && !eff_lower.contains("гармон") && !eff_lower.contains("autopitch") {
        let semitones = json.get("semitones").or_else(|| json.get("pitch")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let fine = json.get("fine").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::PitchShift { semitones, fine_cents: fine, mix }, state))
    } else if eff_lower.contains("flanger") || eff_lower.contains("флэнжер") || eff_lower.contains("фленджер") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(60.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        Some((BossEffectKind::Flanger { rate, depth, feedback, mix }, state))
    } else if eff_lower.contains("phaser") || eff_lower.contains("фейзер") || eff_lower.contains("фазер") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(0.6) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(80.0) / 100.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        Some((BossEffectKind::Phaser { rate, depth, feedback, mix }, state))
    } else if eff_lower.contains("delay") || eff_lower.contains("дилей") || eff_lower.contains("дилэй") || eff_lower.contains("эхо") {
        let time_ms = json.get("time").or_else(|| json.get("time_ms")).and_then(|v| v.as_f64()).unwrap_or(350.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(45.0) / 100.0) as f32;
        let damp_fc = json.get("damp").or_else(|| json.get("tone")).and_then(|v| v.as_f64()).unwrap_or(4500.0) as f32;
        let reverse = json.get("reverse").and_then(|v| v.as_bool()).unwrap_or(false) || eff_lower.contains("reverse") || eff_lower.contains("реверс");
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(35.0) / 100.0) as f32;
        Some((BossEffectKind::Delay { time_ms, feedback, damp_fc, reverse, mix }, state))
    } else if eff_lower.contains("wah") || eff_lower.contains("вау") {
        let sensitivity = (json.get("sensitivity").or_else(|| json.get("sense")).and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        let freq_min = json.get("min_freq").and_then(|v| v.as_f64()).unwrap_or(350.0) as f32;
        let freq_max = json.get("max_freq").and_then(|v| v.as_f64()).unwrap_or(3200.0) as f32;
        let resonance_q = json.get("resonance").or_else(|| json.get("q")).and_then(|v| v.as_f64()).unwrap_or(5.5) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        state.wah_filter.calculate(0, freq_min, 12.0, resonance_q, sample_rate);
        Some((BossEffectKind::AutoWah { sensitivity, freq_min, freq_max, resonance_q, mix }, state))
    } else if eff_lower.contains("fuzz") || eff_lower.contains("фуз") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(70.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(3800.0) as f32;
        let sustain = (json.get("sustain").and_then(|v| v.as_f64()).unwrap_or(60.0) / 100.0) as f32;
        let level = json.get("level").and_then(|v| v.as_f64()).unwrap_or(-3.0) as f32;
        let out_gain = 10.0f32.powf(level / 20.0);
        Some((BossEffectKind::Fuzz { drive, tone, sustain, out_gain }, state))
    } else if eff_lower.contains("roll") || eff_lower.contains("ролл") || (eff_lower.contains("repeat") && !eff_lower.contains("slicer")) {
        let bpm = json.get("rate").or_else(|| json.get("bpm")).and_then(|v| v.as_f64()).unwrap_or(128.0) as f32;
        let pat_str = json.get("pattern").and_then(|v| v.as_str()).unwrap_or("1/16 Beat");
        let subdiv = if pat_str.contains("1/4") {
            1.0
        } else if pat_str.contains("1/8 Triplet") || pat_str.contains("Triplet") {
            3.0
        } else if pat_str.contains("1/8") {
            2.0
        } else if pat_str.contains("1/32") {
            8.0
        } else {
            4.0
        };
        let decay = (json.get("decay").and_then(|v| v.as_f64()).unwrap_or(85.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(60.0) / 100.0) as f32;
        let slice_len = ((60.0 / bpm / subdiv) * sample_rate).clamp(128.0, 95000.0) as usize;
        state.repeat_slice_len = slice_len;
        Some((BossEffectKind::BeatRepeat { bpm, subdivision: subdiv, decay, mix }, state))
    } else if eff_lower.contains("telephone") || eff_lower.contains("phone") || eff_lower.contains("телефон") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(40.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(85.0) / 100.0) as f32;
        state.tel_hp.calculate(4, 380.0, 0.0, 0.7071, sample_rate);
        state.tel_lp.calculate(3, 3300.0, 0.0, 0.7071, sample_rate);
        Some((BossEffectKind::Telephone { drive, mix }, state))
    } else if eff_lower.contains("autopitch") || eff_lower.contains("electric") || eff_lower.contains("автопитч") {
        let speed = (json.get("speed").and_then(|v| v.as_f64()).unwrap_or(80.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::AutoPitch { speed, mix }, state))
    } else if eff_lower.contains("robot") || eff_lower.contains("робот") {
        let pitch_st = json.get("pitch").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let robotic = (json.get("robotic").and_then(|v| v.as_f64()).unwrap_or(90.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::Robot { pitch_st, robotic, mix }, state))
    } else if eff_lower.contains("harmony") || eff_lower.contains("гармон") {
        let interval = json.get("interval").and_then(|v| v.as_f64()).unwrap_or(4.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        Some((BossEffectKind::Harmony { interval, mix }, state))
    } else if eff_lower.contains("transpose") || eff_lower.contains("транспон") {
        let semitones = json.get("semitones").and_then(|v| v.as_f64()).unwrap_or(2.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::Transpose { semitones, mix }, state))
    } else if eff_lower.contains("guitartobass") || eff_lower.contains("guitar to bass") || eff_lower.contains("бас") {
        let gain = (json.get("gain").and_then(|v| v.as_f64()).unwrap_or(3.0)) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        state.generic_filter1.calculate(3, 280.0, 0.0, 0.7071, sample_rate);
        Some((BossEffectKind::GuitarToBass { gain, mix }, state))
    } else if eff_lower.contains("slowgear") || eff_lower.contains("slow gear") || eff_lower.contains("скрипк") {
        let attack_ms = json.get("attack").and_then(|v| v.as_f64()).unwrap_or(250.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::SlowGear { attack_ms, mix }, state))
    } else if eff_lower.contains("tremolo") || eff_lower.contains("тремоло") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(5.0) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::Tremolo { rate, depth, mix }, state))
    } else if eff_lower.contains("vibrato") || eff_lower.contains("вибрато") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(6.0) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(60.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::Vibrato { rate, depth, mix }, state))
    } else if eff_lower.contains("rotary") || eff_lower.contains("ротари") || eff_lower.contains("лесли") {
        let speed = json.get("speed").and_then(|v| v.as_f64()).unwrap_or(4.0) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(80.0) / 100.0) as f32;
        Some((BossEffectKind::Rotary { speed, depth, mix }, state))
    } else if eff_lower.contains("autopan") || eff_lower.contains("auto pan") || eff_lower.contains("панорам") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(1.5) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(80.0) / 100.0) as f32;
        Some((BossEffectKind::AutoPan { rate, depth }, state))
    } else if eff_lower.contains("lpf") || eff_lower.contains("low pass") || eff_lower.contains("нч фильтр") {
        let cutoff = json.get("cutoff").and_then(|v| v.as_f64()).unwrap_or(2200.0) as f32;
        let res = json.get("resonance").and_then(|v| v.as_f64()).unwrap_or(2.5) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        state.generic_filter1.calculate(3, cutoff, 0.0, res, sample_rate);
        Some((BossEffectKind::FilterLpf { cutoff, resonance_q: res, mix }, state))
    } else if eff_lower.contains("bpf") || eff_lower.contains("band pass") || eff_lower.contains("полосов") {
        let center = json.get("center").and_then(|v| v.as_f64()).unwrap_or(1200.0) as f32;
        let res = json.get("resonance").and_then(|v| v.as_f64()).unwrap_or(3.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        state.generic_filter1.calculate(0, center, 6.0, res, sample_rate);
        Some((BossEffectKind::FilterBpf { center_freq: center, resonance_q: res, mix }, state))
    } else if eff_lower.contains("hpf") || eff_lower.contains("high pass") || eff_lower.contains("вч фильтр") {
        let cutoff = json.get("cutoff").and_then(|v| v.as_f64()).unwrap_or(800.0) as f32;
        let res = json.get("resonance").and_then(|v| v.as_f64()).unwrap_or(2.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        state.generic_filter1.calculate(4, cutoff, 0.0, res, sample_rate);
        Some((BossEffectKind::FilterHpf { cutoff, resonance_q: res, mix }, state))
    } else if eff_lower.contains("synth") || eff_lower.contains("синт") {
        let cutoff = json.get("cutoff").and_then(|v| v.as_f64()).unwrap_or(2500.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        Some((BossEffectKind::Synth { wave: 1, cutoff, mix }, state))
    } else if eff_lower.contains("overdrive") || eff_lower.contains("овердрайв") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(40.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(5000.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(100.0) / 100.0) as f32;
        Some((BossEffectKind::Overdrive { drive, tone, mix }, state))
    } else if eff_lower.contains("metal") || eff_lower.contains("метал") {
        let gain = (json.get("gain").and_then(|v| v.as_f64()).unwrap_or(85.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(4200.0) as f32;
        let out_gain = 10.0f32.powf((json.get("level").and_then(|v| v.as_f64()).unwrap_or(-4.0) as f32) / 20.0);
        state.generic_filter1.calculate(0, 1000.0, -8.0, 1.4, sample_rate); // Mid scoop
        Some((BossEffectKind::MetalDs { gain, tone, out_gain }, state))
    } else if eff_lower.contains("octfuzz") || eff_lower.contains("oct fuzz") || eff_lower.contains("октав фуз") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(3600.0) as f32;
        let out_gain = 10.0f32.powf((json.get("level").and_then(|v| v.as_f64()).unwrap_or(-2.0) as f32) / 20.0);
        Some((BossEffectKind::OctFuzz { drive, tone, out_gain }, state))
    } else if eff_lower.contains("crunch") || eff_lower.contains("кранч") {
        let gain = (json.get("gain").and_then(|v| v.as_f64()).unwrap_or(30.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(6000.0) as f32;
        let out_gain = 10.0f32.powf((json.get("level").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32) / 20.0);
        Some((BossEffectKind::Crunch { gain, tone, out_gain }, state))
    } else if eff_lower.contains("preamp") || eff_lower.contains("преамп") {
        let drive = (json.get("drive").and_then(|v| v.as_f64()).unwrap_or(25.0) / 100.0) as f32;
        let tone = json.get("tone").and_then(|v| v.as_f64()).unwrap_or(6500.0) as f32;
        let out_gain = 10.0f32.powf((json.get("level").and_then(|v| v.as_f64()).unwrap_or(2.0) as f32) / 20.0);
        Some((BossEffectKind::Preamp { drive, tone, out_gain }, state))
    } else if eff_lower.contains("compressor") || eff_lower.contains("компрессор") {
        let thresh = json.get("threshold").and_then(|v| v.as_f64()).unwrap_or(-18.0) as f32;
        let ratio = json.get("ratio").and_then(|v| v.as_f64()).unwrap_or(4.0) as f32;
        let attack = json.get("attack").and_then(|v| v.as_f64()).unwrap_or(20.0) as f32;
        let release = json.get("release").and_then(|v| v.as_f64()).unwrap_or(150.0) as f32;
        let makeup = json.get("makeup").and_then(|v| v.as_f64()).unwrap_or(4.0) as f32;
        Some((BossEffectKind::Compressor { threshold_db: thresh, ratio, attack_ms: attack, release_ms: release, makeup_db: makeup }, state))
    } else if eff_lower.contains("limiter") || eff_lower.contains("лимитер") {
        let thresh = json.get("threshold").and_then(|v| v.as_f64()).unwrap_or(-2.0) as f32;
        let ceiling = json.get("ceiling").and_then(|v| v.as_f64()).unwrap_or(-0.3) as f32;
        Some((BossEffectKind::Limiter { threshold_db: thresh, ceiling_db: ceiling }, state))
    } else if eff_lower.contains("noisesuppressor") || eff_lower.contains("ns") || eff_lower.contains("шумоподав") {
        let thresh = json.get("threshold").and_then(|v| v.as_f64()).unwrap_or(-45.0) as f32;
        let rel = json.get("release").and_then(|v| v.as_f64()).unwrap_or(80.0) as f32;
        Some((BossEffectKind::NoiseSuppressor { threshold_db: thresh, release_ms: rel }, state))
    } else if eff_lower.contains("panningdelay") || eff_lower.contains("panning delay") {
        let time_ms = json.get("time").and_then(|v| v.as_f64()).unwrap_or(380.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(40.0) / 100.0) as f32;
        Some((BossEffectKind::PanningDelay { time_ms, feedback, mix }, state))
    } else if eff_lower.contains("tapeecho") || eff_lower.contains("tape echo") {
        let time_ms = json.get("time").and_then(|v| v.as_f64()).unwrap_or(320.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(55.0) / 100.0) as f32;
        let flutter = (json.get("flutter").and_then(|v| v.as_f64()).unwrap_or(25.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(40.0) / 100.0) as f32;
        Some((BossEffectKind::TapeEcho { time_ms, feedback, flutter, mix }, state))
    } else if eff_lower.contains("warpdelay") || eff_lower.contains("warp") {
        let time_ms = json.get("time").and_then(|v| v.as_f64()).unwrap_or(280.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        let warp = (json.get("warp").and_then(|v| v.as_f64()).unwrap_or(60.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(45.0) / 100.0) as f32;
        Some((BossEffectKind::WarpDelay { time_ms, feedback, warp, mix }, state))
    } else if eff_lower.contains("twistdelay") || eff_lower.contains("twist") {
        let time_ms = json.get("time").and_then(|v| v.as_f64()).unwrap_or(220.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(80.0) / 100.0) as f32;
        let twist = (json.get("twist").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        Some((BossEffectKind::TwistDelay { time_ms, feedback, twist, mix }, state))
    } else if eff_lower.contains("granulardelay") || eff_lower.contains("granular delay") {
        let grain_ms = json.get("grain").and_then(|v| v.as_f64()).unwrap_or(80.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(55.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        Some((BossEffectKind::GranularDelay { grain_ms, feedback, mix }, state))
    } else if eff_lower.contains("beatshift") || eff_lower.contains("beat shift") {
        let bpm = json.get("bpm").and_then(|v| v.as_f64()).unwrap_or(120.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        let shift_samples = ((60.0 / bpm * 0.25) * sample_rate) as usize;
        Some((BossEffectKind::BeatShift { shift_samples, mix }, state))
    } else if eff_lower.contains("roomreverb") || eff_lower.contains("комнат") {
        let size = (json.get("size").and_then(|v| v.as_f64()).unwrap_or(40.0) / 100.0) as f32;
        let damp = (json.get("damp").and_then(|v| v.as_f64()).unwrap_or(50.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(25.0) / 100.0) as f32;
        Some((BossEffectKind::RoomReverb { size, damp, mix }, state))
    } else if eff_lower.contains("hallreverb") || eff_lower.contains("концерт") {
        let size = (json.get("size").and_then(|v| v.as_f64()).unwrap_or(75.0) / 100.0) as f32;
        let predelay = json.get("predelay").and_then(|v| v.as_f64()).unwrap_or(30.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(35.0) / 100.0) as f32;
        Some((BossEffectKind::HallReverb { size, predelay, mix }, state))
    } else if eff_lower.contains("platereverb") || eff_lower.contains("пластинчат") {
        let decay = json.get("decay").and_then(|v| v.as_f64()).unwrap_or(2.8) as f32;
        let bright = (json.get("brightness").and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(30.0) / 100.0) as f32;
        Some((BossEffectKind::PlateReverb { decay, brightness: bright, mix }, state))
    } else if eff_lower.contains("springreverb") || eff_lower.contains("пружин") {
        let decay = (json.get("decay").and_then(|v| v.as_f64()).unwrap_or(55.0) / 100.0) as f32;
        let tension = (json.get("tension").and_then(|v| v.as_f64()).unwrap_or(70.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(30.0) / 100.0) as f32;
        Some((BossEffectKind::SpringReverb { decay, tension, mix }, state))
    } else {
        None
    }
}

#[inline(always)]
pub fn process_boss_effect(
    kind: &BossEffectKind,
    state: &mut BossEffectState,
    sample: f32,
    sample_rate: f32,
) -> f32 {
    if !sample.is_finite() {
        return 0.0;
    }
    let res = match kind {
        BossEffectKind::Slicer { bpm, subdivision, depth, duty } => {
            let freq = (bpm / 60.0) * subdivision;
            state.slicer_phase += freq / sample_rate;
            if state.slicer_phase >= 1.0 {
                state.slicer_phase -= 1.0;
            }
            let target_gain = if state.slicer_phase < *duty { 1.0 } else { 1.0 - depth };
            state.slicer_gain_smooth += (target_gain - state.slicer_gain_smooth) * 0.08;
            sample * state.slicer_gain_smooth
        }

        BossEffectKind::Vinyl { tone_fc, crackle_level, .. } => {
            state.rng_state = state.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let is_crackle = (state.rng_state & 0x1FFF) < (*crackle_level * 50.0) as u32;
            let crackle_sample = if is_crackle {
                ((state.rng_state >> 16) as f32 / 32768.0 - 1.0) * crackle_level * 0.12
            } else {
                0.0
            };

            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone_fc.clamp(1000.0, 18000.0));
            let alpha = dt / (rc + dt);
            state.vinyl_lp += alpha * (sample - state.vinyl_lp);
            state.vinyl_lp + crackle_sample
        }

        BossEffectKind::Vocoder { pitch_st, robotic, wet } => {
            let f0 = 130.81 * 2.0f32.powf(*pitch_st / 12.0);
            state.vocoder_phase += f0 / sample_rate;
            if state.vocoder_phase >= 1.0 {
                state.vocoder_phase -= 1.0;
            }
            // Rich harmonic carrier
            let carrier = state.vocoder_phase * 2.0 - 1.0;
            // Envelope follower
            state.vocoder_env = state.vocoder_env * 0.992 + sample.abs() * 0.008;
            let robot = carrier * state.vocoder_env * 3.5;
            let wet_sample = (1.0 - robotic) * sample + robotic * robot;
            (1.0 - wet) * sample + wet * wet_sample
        }

        BossEffectKind::RingMod { carrier_fc, mod_depth, mix } => {
            state.ringmod_phase += carrier_fc / sample_rate;
            if state.ringmod_phase >= 1.0 {
                state.ringmod_phase -= 1.0;
            }
            let carrier = (state.ringmod_phase * 2.0 * std::f32::consts::PI).sin();
            let mod_s = sample * ((1.0 - mod_depth) + mod_depth * carrier);
            (1.0 - mix) * sample + mix * mod_s
        }

        BossEffectKind::Isolator { .. } => {
            let low = state.iso_low.process(sample);
            let mid = state.iso_mid.process(low);
            let high = state.iso_high.process(mid);
            state.iso_cutoff.process(high)
        }

        BossEffectKind::LoFi { drive, noise_level, .. } => {
            let s_hp = state.lofi_hp.process(sample);
            let s_band = state.lofi_lp.process(s_hp);
            let driven = s_band * (1.0 + drive * 6.0);
            let soft_clip = driven / (1.0 + driven.abs());

            state.rng_state = state.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let n = ((state.rng_state >> 16) as f32 / 32768.0 - 1.0) * noise_level * 0.015;
            soft_clip + n
        }

        BossEffectKind::Chorus { rate, depth, feedback, mix } => {
            let buf_len = state.chorus_buf.len();
            state.chorus_lfo += rate / sample_rate;
            if state.chorus_lfo >= 1.0 {
                state.chorus_lfo -= 1.0;
            }
            let lfo = (state.chorus_lfo * 2.0 * std::f32::consts::PI).sin();
            let delay_samples = (sample_rate * 0.012) + (sample_rate * 0.007) * depth * lfo;

            let read_pos = (state.chorus_w_idx as f32 - delay_samples + buf_len as f32) % buf_len as f32;
            let i0 = read_pos.floor() as usize % buf_len;
            let i1 = (i0 + 1) % buf_len;
            let frac = read_pos - read_pos.floor();
            let delayed = state.chorus_buf[i0] * (1.0 - frac) + state.chorus_buf[i1] * frac;

            state.chorus_buf[state.chorus_w_idx] = sample + delayed * feedback;
            state.chorus_w_idx = (state.chorus_w_idx + 1) % buf_len;

            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::Distortion { drive, tone, out_gain, .. } => {
            let s_bottom = state.dist_bottom_filter.process(sample);
            let x = s_bottom * (1.0 + drive * 10.0);
            let overdriven = ((x + 0.15).tanh() - 0.15f32.tanh()) * 0.85;

            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 18000.0));
            let alpha = dt / (rc + dt);
            state.dist_tone_lp += alpha * (overdriven - state.dist_tone_lp);
            state.dist_tone_lp * out_gain
        }

        BossEffectKind::Scatter { grain_ms, mode, density, mix } => {
            let buf_cap = state.scatter_buf.len();
            let cur_w = state.scatter_w_idx;
            state.scatter_buf[cur_w] = sample;
            state.scatter_w_idx = (cur_w + 1) % buf_cap;

            let grain_samples = ((*grain_ms * 0.001) * sample_rate).clamp(128.0, (buf_cap - 256) as f32) as usize;

            if state.scatter_grain_pos >= grain_samples {
                state.scatter_grain_pos = 0;
                state.scatter_pitch_phase = 0.0;

                // Density gate using LCG RNG
                state.rng_state = state.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                let roll = ((state.rng_state >> 8) as f32) / 16777216.0;
                state.scatter_active = roll <= *density;

                match *mode {
                    0 => {
                        // Random Scatter: jump to random grain chunk in history
                        state.rng_state = state.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                        let rand_val = ((state.rng_state >> 8) as f32) / 16777216.0;
                        let max_offset = (buf_cap - grain_samples - 64).max(grain_samples + 64);
                        let rand_offset = ((rand_val * max_offset as f32) as usize).clamp(grain_samples, max_offset);
                        state.scatter_read_start = (cur_w + buf_cap - rand_offset) % buf_cap;
                    }
                    1 => {
                        // Stutter / Beat Repeat: freeze and loop captured grain
                        if state.scatter_freeze_offset == 0 {
                            state.scatter_freeze_offset = grain_samples;
                        }
                        state.scatter_read_start = (cur_w + buf_cap - state.scatter_freeze_offset) % buf_cap;
                    }
                    2 => {
                        // Reverse: start from newest written sample and read backward
                        state.scatter_read_start = (cur_w + buf_cap - 1) % buf_cap;
                    }
                    3 => {
                        // Pitch-shift Up 2x: octave shift
                        state.scatter_read_start = (cur_w + buf_cap - grain_samples) % buf_cap;
                    }
                    _ => {
                        state.scatter_read_start = (cur_w + buf_cap - grain_samples) % buf_cap;
                    }
                }
            }

            let grain_s = if !state.scatter_active {
                sample
            } else {
                let pos = state.scatter_grain_pos;
                let norm = (pos as f32) / (grain_samples as f32);
                let env = (0.5 * (1.0 - (2.0 * std::f32::consts::PI * norm).cos())).clamp(0.0, 1.0);

                let raw = match *mode {
                    0 | 1 => {
                        let r_idx = (state.scatter_read_start + pos) % buf_cap;
                        state.scatter_buf[r_idx]
                    }
                    2 => {
                        let r_idx = (state.scatter_read_start + buf_cap - (pos % grain_samples)) % buf_cap;
                        state.scatter_buf[r_idx]
                    }
                    3 => {
                        let read_pos = (state.scatter_read_start as f32 + state.scatter_pitch_phase) % (buf_cap as f32);
                        let idx0 = read_pos as usize;
                        let idx1 = (idx0 + 1) % buf_cap;
                        let frac = read_pos.fract();
                        let s0 = state.scatter_buf[idx0];
                        let s1 = state.scatter_buf[idx1];
                        state.scatter_pitch_phase += 2.0;
                        if state.scatter_pitch_phase >= (grain_samples as f32) {
                            state.scatter_pitch_phase -= grain_samples as f32;
                        }
                        (1.0 - frac) * s0 + frac * s1
                    }
                    _ => sample,
                };

                env * raw + (1.0 - env) * sample
            };

            state.scatter_grain_pos += 1;
            (1.0 - mix) * sample + mix * grain_s
        }

        BossEffectKind::Reverb { decay_time, mix, .. } => {
            let comb_lens = [1116, 1188, 1277, 1356];
            let allpass_lens = [225, 556];
            let fb = (0.5f32).powf(0.05 / decay_time.max(0.2)).clamp(0.4, 0.94);

            let mut combs_sum = 0.0f32;

            let out1 = state.rev_comb1[state.rev_comb1_idx];
            state.rev_comb1[state.rev_comb1_idx] = sample + out1 * fb;
            state.rev_comb1_idx = (state.rev_comb1_idx + 1) % comb_lens[0];
            combs_sum += out1;

            let out2 = state.rev_comb2[state.rev_comb2_idx];
            state.rev_comb2[state.rev_comb2_idx] = sample + out2 * fb;
            state.rev_comb2_idx = (state.rev_comb2_idx + 1) % comb_lens[1];
            combs_sum += out2;

            let out3 = state.rev_comb3[state.rev_comb3_idx];
            state.rev_comb3[state.rev_comb3_idx] = sample + out3 * fb;
            state.rev_comb3_idx = (state.rev_comb3_idx + 1) % comb_lens[2];
            combs_sum += out3;

            let out4 = state.rev_comb4[state.rev_comb4_idx];
            state.rev_comb4[state.rev_comb4_idx] = sample + out4 * fb;
            state.rev_comb4_idx = (state.rev_comb4_idx + 1) % comb_lens[3];
            combs_sum += out4;

            combs_sum *= 0.25;

            let ap1_buf = state.rev_ap1[state.rev_ap1_idx];
            let ap1_out = -combs_sum + ap1_buf;
            state.rev_ap1[state.rev_ap1_idx] = combs_sum + ap1_buf * 0.5;
            state.rev_ap1_idx = (state.rev_ap1_idx + 1) % allpass_lens[0];

            let ap2_buf = state.rev_ap2[state.rev_ap2_idx];
            let ap2_out = -ap1_out + ap2_buf;
            state.rev_ap2[state.rev_ap2_idx] = ap1_out + ap2_buf * 0.5;
            state.rev_ap2_idx = (state.rev_ap2_idx + 1) % allpass_lens[1];

            (1.0 - mix) * sample + mix * ap2_out
        }

        BossEffectKind::PitchShift { semitones, fine_cents, mix } => {
            let buf_cap = state.pitch_buf.len();
            let cur_w = state.pitch_w_idx;
            state.pitch_buf[cur_w] = sample;
            state.pitch_w_idx = (cur_w + 1) % buf_cap;

            let grain_w = 2048.0f32;
            let ratio = 2.0f32.powf((*semitones + *fine_cents * 0.01) / 12.0);

            // Head 1
            let read_pos1 = (cur_w as f32 + buf_cap as f32 - grain_w + state.pitch_phase1) % (buf_cap as f32);
            let i1_0 = read_pos1.floor() as usize % buf_cap;
            let i1_1 = (i1_0 + 1) % buf_cap;
            let f1 = read_pos1 - read_pos1.floor();
            let s1 = state.pitch_buf[i1_0] * (1.0 - f1) + state.pitch_buf[i1_1] * f1;
            let w1 = (0.5 * (1.0 - (2.0 * std::f32::consts::PI * (state.pitch_phase1 / grain_w)).cos())).max(0.0);

            // Head 2 (offset by 180 degrees / half grain)
            let read_pos2 = (cur_w as f32 + buf_cap as f32 - grain_w + state.pitch_phase2) % (buf_cap as f32);
            let i2_0 = read_pos2.floor() as usize % buf_cap;
            let i2_1 = (i2_0 + 1) % buf_cap;
            let f2 = read_pos2 - read_pos2.floor();
            let s2 = state.pitch_buf[i2_0] * (1.0 - f2) + state.pitch_buf[i2_1] * f2;
            let w2 = (0.5 * (1.0 - (2.0 * std::f32::consts::PI * (state.pitch_phase2 / grain_w)).cos())).max(0.0);

            state.pitch_phase1 += ratio;
            if state.pitch_phase1 >= grain_w {
                state.pitch_phase1 -= grain_w;
            }
            state.pitch_phase2 += ratio;
            if state.pitch_phase2 >= grain_w {
                state.pitch_phase2 -= grain_w;
            }

            let denom = (w1 + w2).max(1e-4);
            let shifted = (s1 * w1 + s2 * w2) / denom;
            if *mix >= 0.98 {
                shifted
            } else {
                (1.0 - mix) * sample + mix * shifted
            }
        }

        BossEffectKind::Flanger { rate, depth, feedback, mix } => {
            let buf_cap = state.flanger_buf.len();
            state.flanger_lfo += rate / sample_rate;
            if state.flanger_lfo >= 1.0 {
                state.flanger_lfo -= 1.0;
            }
            let lfo = (state.flanger_lfo * 2.0 * std::f32::consts::PI).sin();
            let delay_samples = (0.0018 + 0.0014 * depth * lfo) * sample_rate;
            let r_pos = (state.flanger_w_idx as f32 + buf_cap as f32 - delay_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let i1 = (i0 + 1) % buf_cap;
            let frac = r_pos - r_pos.floor();
            let delayed = state.flanger_buf[i0] * (1.0 - frac) + state.flanger_buf[i1] * frac;

            state.flanger_buf[state.flanger_w_idx] = sample + delayed * feedback;
            state.flanger_w_idx = (state.flanger_w_idx + 1) % buf_cap;
            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::Phaser { rate, depth, feedback, mix } => {
            state.phaser_lfo += rate / sample_rate;
            if state.phaser_lfo >= 1.0 {
                state.phaser_lfo -= 1.0;
            }
            let lfo = 0.5 + 0.5 * (state.phaser_lfo * 2.0 * std::f32::consts::PI).sin();
            let fc = (450.0 + 2800.0 * depth * lfo).clamp(200.0, 16000.0);
            let omega = (std::f32::consts::PI * fc / sample_rate).tan();
            let a = (1.0 - omega) / (1.0 + omega);

            let mut cur_in = sample + state.phaser_last_out * feedback * 0.7;
            for i in 0..4 {
                let ap_out = a * cur_in + state.phaser_ap_x[i] - a * state.phaser_ap_y[i];
                state.phaser_ap_x[i] = cur_in;
                state.phaser_ap_y[i] = ap_out;
                cur_in = ap_out;
            }
            state.phaser_last_out = cur_in;
            (1.0 - mix) * sample + mix * cur_in
        }

        BossEffectKind::Delay { time_ms, feedback, damp_fc, reverse, mix } => {
            let buf_cap = state.delay_buf.len();
            let delay_samples = ((*time_ms * 0.001) * sample_rate).clamp(10.0, (buf_cap - 128) as f32);
            let cur_w = state.delay_w_idx;

            let delayed = if *reverse {
                let chunk_len = delay_samples as usize;
                let phase_in_chunk = cur_w % chunk_len;
                let r_idx = (cur_w + buf_cap - phase_in_chunk) % buf_cap;
                state.delay_buf[r_idx]
            } else {
                let r_pos = (cur_w as f32 + buf_cap as f32 - delay_samples) % (buf_cap as f32);
                let i0 = r_pos.floor() as usize % buf_cap;
                let i1 = (i0 + 1) % buf_cap;
                let frac = r_pos - r_pos.floor();
                state.delay_buf[i0] * (1.0 - frac) + state.delay_buf[i1] * frac
            };

            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * damp_fc.clamp(1000.0, 18000.0));
            let alpha = dt / (rc + dt);
            state.delay_lp += alpha * (delayed - state.delay_lp);

            state.delay_buf[cur_w] = sample + state.delay_lp * feedback.clamp(0.0, 0.95);
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::AutoWah { sensitivity, freq_min, freq_max, resonance_q, mix } => {
            state.wah_env = state.wah_env * 0.993 + sample.abs() * 0.007;
            let mod_val = (state.wah_env * sensitivity * 6.5).clamp(0.0, 1.0);
            let fc = freq_min + (freq_max - freq_min) * mod_val;
            state.wah_filter.calculate(0, fc.clamp(200.0, 8000.0), 10.0, *resonance_q, sample_rate);
            let filtered = state.wah_filter.process(sample);
            (1.0 - mix) * sample + mix * filtered
        }

        BossEffectKind::Fuzz { drive, tone, sustain, out_gain } => {
            let x = sample * (1.0 + sustain * 10.0) * (1.0 + drive * 16.0);
            let sign = if x >= 0.0 { 1.0 } else { -1.0 };
            let abs_x = x.abs();
            let saturated = sign * (1.0 - (-abs_x).exp()) * 0.9 + 0.1 * (x / (1.0 + abs_x));

            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 16000.0));
            let alpha = dt / (rc + dt);
            state.fuzz_tone_lp += alpha * (saturated - state.fuzz_tone_lp);
            state.fuzz_tone_lp * out_gain
        }

        BossEffectKind::BeatRepeat { decay, mix, .. } => {
            let buf_cap = state.repeat_buf.len();
            let cur_w = state.repeat_w_idx;
            state.repeat_buf[cur_w] = sample;
            state.repeat_w_idx = (cur_w + 1) % buf_cap;

            let slice = state.repeat_slice_len.max(128).min(buf_cap - 64);
            let r_idx = (cur_w + buf_cap - slice + (state.repeat_pos % slice)) % buf_cap;
            let repeated = state.repeat_buf[r_idx] * decay;
            state.repeat_pos = (state.repeat_pos + 1) % slice;
            (1.0 - mix) * sample + mix * repeated
        }

        BossEffectKind::Telephone { drive, mix } => {
            let s_hp = state.tel_hp.process(sample);
            let s_bp = state.tel_lp.process(s_hp);
            let driven = s_bp * (1.0 + drive * 5.0);
            let soft = driven / (1.0 + driven.abs());
            (1.0 - mix) * sample + mix * soft
        }

        BossEffectKind::AutoPitch { speed, mix } => {
            let snapped = (sample * 8.0).round() / 8.0;
            let tuned = (1.0 - speed) * sample + speed * snapped;
            (1.0 - mix) * sample + mix * tuned
        }

        BossEffectKind::Robot { pitch_st, robotic, mix } => {
            // Roland BOSS RC-505 / VT-4 Authentic Robot Voice:
            // Locks the fundamental vocal pitch to the designated note (f0 = 130.81 * 2^(pitch_st / 12))
            // with zero pitch glide, while preserving vocal tract formants and word clarity.
            let f0 = (130.81 * 2.0f32.powf(*pitch_st / 12.0)).clamp(55.0, 520.0);
            let target_period = (sample_rate / f0).clamp(32.0, 1024.0);

            let buf_cap = state.pitch_buf.len();
            let cur_w = state.pitch_w_idx;
            state.pitch_buf[cur_w] = sample;
            state.pitch_w_idx = (cur_w + 1) % buf_cap;

            // Pitch tracking: detects fundamental period of incoming voice
            state.robot_zc_count += 1;
            if sample > 0.0 && state.robot_last_sample <= 0.0 && (sample - state.robot_last_sample) > 0.004 {
                let measured_period = state.robot_zc_count as f32;
                state.robot_zc_count = 0;
                let min_period = sample_rate / 500.0;
                let max_period = sample_rate / 70.0;
                if measured_period >= min_period && measured_period <= max_period {
                    state.robot_pitch_period = state.robot_pitch_period * 0.82 + measured_period * 0.18;
                }
            }
            state.robot_last_sample = sample;

            let in_period = state.robot_pitch_period.clamp(40.0, 800.0);
            let shift_ratio = (in_period / target_period).clamp(0.25, 4.0);

            // Grain window dynamically locked to the target pitch period
            let grain_w = (target_period * 2.0).clamp(128.0, 2048.0);

            // Dual-head crossfade
            let r_pos1 = (cur_w as f32 + buf_cap as f32 - grain_w + state.robot_phase1) % (buf_cap as f32);
            let i1_0 = r_pos1.floor() as usize % buf_cap;
            let i1_1 = (i1_0 + 1) % buf_cap;
            let f1 = r_pos1 - r_pos1.floor();
            let s1 = state.pitch_buf[i1_0] * (1.0 - f1) + state.pitch_buf[i1_1] * f1;
            let w1 = (0.5 * (1.0 - (2.0 * std::f32::consts::PI * (state.robot_phase1 / grain_w)).cos())).max(0.0);

            let r_pos2 = (cur_w as f32 + buf_cap as f32 - grain_w + state.robot_phase2) % (buf_cap as f32);
            let i2_0 = r_pos2.floor() as usize % buf_cap;
            let i2_1 = (i2_0 + 1) % buf_cap;
            let f2 = r_pos2 - r_pos2.floor();
            let s2 = state.pitch_buf[i2_0] * (1.0 - f2) + state.pitch_buf[i2_1] * f2;
            let w2 = (0.5 * (1.0 - (2.0 * std::f32::consts::PI * (state.robot_phase2 / grain_w)).cos())).max(0.0);

            state.robot_phase1 += shift_ratio;
            if state.robot_phase1 >= grain_w {
                state.robot_phase1 -= grain_w;
            }
            state.robot_phase2 += shift_ratio;
            if state.robot_phase2 >= grain_w {
                state.robot_phase2 -= grain_w;
            }

            let denom = (w1 + w2).max(1e-4);
            let pitch_locked = (s1 * w1 + s2 * w2) / denom;

            // Signature Boss RC-505 / VT-4 robotic timbre reinforcement
            state.robot_vocal_env = state.robot_vocal_env * 0.994 + sample.abs() * 0.006;
            state.robot_carrier_phase = (state.robot_carrier_phase + f0 / sample_rate) % 1.0;
            let p = state.robot_carrier_phase;
            let saw = 2.0 * p - 1.0 - 0.25 * (2.0 * std::f32::consts::PI * p).sin();
            let carrier = saw * state.robot_vocal_env * 0.75;

            let robot_wet = (pitch_locked * 0.85 + carrier * (*robotic * 0.45)) * (1.0 + *robotic * 0.25);
            let robot_saturated = (robot_wet * 1.2).tanh();

            // Complete isolation of dry voice when mix is 100% (mix >= 0.98)
            if *mix >= 0.98 {
                robot_saturated
            } else {
                (1.0 - *mix) * sample + *mix * robot_saturated
            }
        }

        BossEffectKind::Harmony { interval, mix } => {
            let buf_cap = state.pitch_buf.len();
            let cur_w = state.pitch_w_idx;
            state.pitch_buf[cur_w] = sample;
            state.pitch_w_idx = (cur_w + 1) % buf_cap;
            let grain_w = 2048.0f32;
            let ratio = 2.0f32.powf(*interval / 12.0);
            let r_pos = (cur_w as f32 + buf_cap as f32 - grain_w + state.pitch_phase1) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let i1 = (i0 + 1) % buf_cap;
            let frac = r_pos - r_pos.floor();
            let shifted = state.pitch_buf[i0] * (1.0 - frac) + state.pitch_buf[i1] * frac;
            state.pitch_phase1 += ratio;
            if state.pitch_phase1 >= grain_w { state.pitch_phase1 -= grain_w; }
            (1.0 - mix) * sample + mix * (sample * 0.6 + shifted * 0.6)
        }

        BossEffectKind::Transpose { semitones, mix } => {
            let buf_cap = state.pitch_buf.len();
            let cur_w = state.pitch_w_idx;
            state.pitch_buf[cur_w] = sample;
            state.pitch_w_idx = (cur_w + 1) % buf_cap;
            let grain_w = 2048.0f32;
            let ratio = 2.0f32.powf(*semitones / 12.0);
            let r_pos = (cur_w as f32 + buf_cap as f32 - grain_w + state.pitch_phase1) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let i1 = (i0 + 1) % buf_cap;
            let frac = r_pos - r_pos.floor();
            let shifted = state.pitch_buf[i0] * (1.0 - frac) + state.pitch_buf[i1] * frac;
            state.pitch_phase1 += ratio;
            if state.pitch_phase1 >= grain_w { state.pitch_phase1 -= grain_w; }
            if *mix >= 0.98 {
                shifted
            } else {
                (1.0 - mix) * sample + mix * shifted
            }
        }

        BossEffectKind::GuitarToBass { gain, mix, .. } => {
            let lp = state.generic_filter1.process(sample);
            let sub = (lp * 2.5).clamp(-1.0, 1.0) * gain;
            (1.0 - mix) * sample + mix * sub
        }

        BossEffectKind::SlowGear { attack_ms, mix } => {
            let env = sample.abs();
            if env > 0.05 && state.slowgear_gain < 0.05 {
                state.slowgear_gain = 0.05;
            }
            let step = 1.0 / (attack_ms * 0.001 * sample_rate).max(10.0);
            state.slowgear_gain = (state.slowgear_gain + step).min(1.0);
            if env < 0.02 {
                state.slowgear_gain *= 0.98;
            }
            let out = sample * state.slowgear_gain;
            (1.0 - mix) * sample + mix * out
        }

        BossEffectKind::Tremolo { rate, depth, mix } => {
            state.tremolo_lfo += rate / sample_rate;
            if state.tremolo_lfo >= 1.0 { state.tremolo_lfo -= 1.0; }
            let lfo = 0.5 * (1.0 + (state.tremolo_lfo * 2.0 * std::f32::consts::PI).sin());
            let gain = 1.0 - depth * (1.0 - lfo);
            let out = sample * gain;
            (1.0 - mix) * sample + mix * out
        }

        BossEffectKind::Vibrato { rate, depth, mix } => {
            state.vibrato_lfo += rate / sample_rate;
            if state.vibrato_lfo >= 1.0 { state.vibrato_lfo -= 1.0; }
            let lfo = (state.vibrato_lfo * 2.0 * std::f32::consts::PI).sin();
            let buf_cap = state.chorus_buf.len();
            let delay_samples = (0.005 + 0.003 * depth * lfo) * sample_rate;
            let r_pos = (state.chorus_w_idx as f32 + buf_cap as f32 - delay_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let i1 = (i0 + 1) % buf_cap;
            let frac = r_pos - r_pos.floor();
            let out = state.chorus_buf[i0] * (1.0 - frac) + state.chorus_buf[i1] * frac;
            state.chorus_buf[state.chorus_w_idx] = sample;
            state.chorus_w_idx = (state.chorus_w_idx + 1) % buf_cap;
            (1.0 - mix) * sample + mix * out
        }

        BossEffectKind::Rotary { speed, depth, mix } => {
            state.rotary_lfo += speed / sample_rate;
            if state.rotary_lfo >= 1.0 { state.rotary_lfo -= 1.0; }
            let lfo = 0.5 * (1.0 + (state.rotary_lfo * 2.0 * std::f32::consts::PI).sin());
            let trem = 1.0 - depth * 0.4 * (1.0 - lfo);
            let sat = (sample * 1.5).tanh() * 0.8;
            (1.0 - mix) * sample + mix * (sat * trem)
        }

        BossEffectKind::AutoPan { rate, depth } => {
            state.autopan_lfo += rate / sample_rate;
            if state.autopan_lfo >= 1.0 { state.autopan_lfo -= 1.0; }
            let pan = (state.autopan_lfo * 2.0 * std::f32::consts::PI).sin();
            sample * (1.0 - depth * 0.3 * pan)
        }

        BossEffectKind::FilterLpf { mix, .. } => {
            let filtered = state.generic_filter1.process(sample);
            (1.0 - mix) * sample + mix * filtered
        }

        BossEffectKind::FilterBpf { mix, .. } => {
            let filtered = state.generic_filter1.process(sample);
            (1.0 - mix) * sample + mix * filtered
        }

        BossEffectKind::FilterHpf { mix, .. } => {
            let filtered = state.generic_filter1.process(sample);
            (1.0 - mix) * sample + mix * filtered
        }

        BossEffectKind::Synth { cutoff, mix, .. } => {
            state.synth_phase += 130.0 / sample_rate;
            if state.synth_phase >= 1.0 { state.synth_phase -= 1.0; }
            let saw = state.synth_phase * 2.0 - 1.0;
            let env = sample.abs() * 3.0;
            let s_out = (saw * env).tanh();
            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff.clamp(300.0, 16000.0));
            let alpha = dt / (rc + dt);
            state.fuzz_tone_lp += alpha * (s_out - state.fuzz_tone_lp);
            (1.0 - mix) * sample + mix * state.fuzz_tone_lp
        }

        BossEffectKind::Overdrive { drive, tone, mix } => {
            let x = sample * (1.0 + drive * 9.0);
            let od = (x + 0.1).tanh() - 0.1f32.tanh();
            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 16000.0));
            let alpha = dt / (rc + dt);
            state.dist_tone_lp += alpha * (od - state.dist_tone_lp);
            (1.0 - mix) * sample + mix * state.dist_tone_lp
        }

        BossEffectKind::MetalDs { gain, tone, out_gain } => {
            let s_scoop = state.generic_filter1.process(sample);
            let x = s_scoop * (1.0 + gain * 25.0);
            let clipped = x.clamp(-0.85, 0.85) * 1.15;
            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 16000.0));
            let alpha = dt / (rc + dt);
            state.fuzz_tone_lp += alpha * (clipped - state.fuzz_tone_lp);
            state.fuzz_tone_lp * out_gain
        }

        BossEffectKind::OctFuzz { drive, tone, out_gain } => {
            let rect = sample.abs() * 2.0 - 0.5;
            let x = rect * (1.0 + drive * 15.0);
            let fuzzed = (x / (1.0 + x.abs())).clamp(-1.0, 1.0);
            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 16000.0));
            let alpha = dt / (rc + dt);
            state.fuzz_tone_lp += alpha * (fuzzed - state.fuzz_tone_lp);
            state.fuzz_tone_lp * out_gain
        }

        BossEffectKind::Crunch { gain, tone, out_gain } => {
            let x = sample * (1.0 + gain * 6.0);
            let crunch = (x * 1.2).tanh() * 0.9;
            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 16000.0));
            let alpha = dt / (rc + dt);
            state.dist_tone_lp += alpha * (crunch - state.dist_tone_lp);
            state.dist_tone_lp * out_gain
        }

        BossEffectKind::Preamp { drive, tone, out_gain } => {
            let x = sample * (1.0 + drive * 4.0);
            let warm = x + 0.15 * x * x - 0.05 * x * x * x;
            let dt = 1.0 / sample_rate;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * tone.clamp(800.0, 18000.0));
            let alpha = dt / (rc + dt);
            state.dist_tone_lp += alpha * (warm - state.dist_tone_lp);
            state.dist_tone_lp * out_gain
        }

        BossEffectKind::Compressor { threshold_db, ratio, makeup_db, .. } => {
            let env = sample.abs();
            state.comp_env = state.comp_env * 0.992 + env * 0.008;
            let env_db = 20.0 * (state.comp_env.max(1e-5)).log10();
            let gr_db = if env_db > *threshold_db {
                (env_db - threshold_db) * (1.0 - 1.0 / ratio.max(1.0))
            } else {
                0.0
            };
            let gain = 10.0f32.powf((makeup_db - gr_db) / 20.0);
            sample * gain
        }

        BossEffectKind::Limiter { threshold_db, ceiling_db } => {
            let thresh = 10.0f32.powf(threshold_db / 20.0);
            let ceiling = 10.0f32.powf(ceiling_db / 20.0);
            let clamped = sample.clamp(-thresh, thresh) * (ceiling / thresh);
            clamped
        }

        BossEffectKind::NoiseSuppressor { threshold_db, .. } => {
            let env = sample.abs();
            state.ns_env = state.ns_env * 0.99 + env * 0.01;
            let env_db = 20.0 * (state.ns_env.max(1e-5)).log10();
            let mult = if env_db < *threshold_db {
                ((env_db - threshold_db - 12.0) / 12.0).clamp(0.0, 1.0)
            } else {
                1.0
            };
            sample * mult
        }

        BossEffectKind::PanningDelay { time_ms, feedback, mix } => {
            let buf_cap = state.delay_buf.len();
            let cur_w = state.delay_w_idx;
            let d_samples = (*time_ms * 0.001 * sample_rate).clamp(10.0, (buf_cap - 128) as f32);
            let r_pos = (cur_w as f32 + buf_cap as f32 - d_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let delayed = state.delay_buf[i0];
            state.delay_buf[cur_w] = sample + delayed * feedback.clamp(0.0, 0.92);
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::TapeEcho { time_ms, feedback, flutter, mix } => {
            state.vinyl_flutter_phase += 2.0 / sample_rate;
            if state.vinyl_flutter_phase >= 1.0 { state.vinyl_flutter_phase -= 1.0; }
            let flut = (state.vinyl_flutter_phase * 2.0 * std::f32::consts::PI).sin() * flutter * 0.002;
            let buf_cap = state.delay_buf.len();
            let cur_w = state.delay_w_idx;
            let d_samples = ((*time_ms * 0.001 + flut) * sample_rate).clamp(10.0, (buf_cap - 128) as f32);
            let r_pos = (cur_w as f32 + buf_cap as f32 - d_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let delayed = state.delay_buf[i0];
            let saturated_fb = (delayed * 1.2).tanh();
            state.delay_buf[cur_w] = sample + saturated_fb * feedback.clamp(0.0, 0.94);
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::WarpDelay { time_ms, feedback, warp, mix } => {
            state.flanger_lfo += 0.8 / sample_rate;
            if state.flanger_lfo >= 1.0 { state.flanger_lfo -= 1.0; }
            let w_mod = (state.flanger_lfo * 2.0 * std::f32::consts::PI).sin() * warp * 0.015;
            let buf_cap = state.delay_buf.len();
            let cur_w = state.delay_w_idx;
            let d_samples = ((*time_ms * 0.001 + w_mod) * sample_rate).clamp(10.0, (buf_cap - 128) as f32);
            let r_pos = (cur_w as f32 + buf_cap as f32 - d_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let delayed = state.delay_buf[i0];
            state.delay_buf[cur_w] = sample + delayed * feedback.clamp(0.0, 0.92);
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::TwistDelay { time_ms, feedback, twist, mix } => {
            let buf_cap = state.delay_buf.len();
            let cur_w = state.delay_w_idx;
            let d_samples = (*time_ms * 0.001 * sample_rate).clamp(10.0, (buf_cap - 128) as f32);
            let r_pos = (cur_w as f32 + buf_cap as f32 - d_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let delayed = state.delay_buf[i0];
            let twist_out = (delayed * (1.0 + twist * 2.5)).tanh();
            state.delay_buf[cur_w] = sample + twist_out * feedback.clamp(0.0, 0.98);
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * twist_out
        }

        BossEffectKind::GranularDelay { grain_ms, feedback, mix } => {
            let buf_cap = state.delay_buf.len();
            let cur_w = state.delay_w_idx;
            let d_samples = (*grain_ms * 0.001 * sample_rate).clamp(10.0, (buf_cap - 128) as f32);
            let r_pos = (cur_w as f32 + buf_cap as f32 - d_samples) % (buf_cap as f32);
            let i0 = r_pos.floor() as usize % buf_cap;
            let delayed = state.delay_buf[i0];
            state.delay_buf[cur_w] = sample + delayed * feedback.clamp(0.0, 0.92);
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * delayed
        }

        BossEffectKind::BeatShift { shift_samples, mix } => {
            let buf_cap = state.delay_buf.len();
            let cur_w = state.delay_w_idx;
            state.delay_buf[cur_w] = sample;
            let r_idx = (cur_w + buf_cap - (*shift_samples).clamp(1, buf_cap - 1)) % buf_cap;
            let shifted = state.delay_buf[r_idx];
            state.delay_w_idx = (cur_w + 1) % buf_cap;
            (1.0 - mix) * sample + mix * shifted
        }

        BossEffectKind::RoomReverb { size, damp, mix } => {
            let fb = (0.35 + 0.45 * size).clamp(0.3, 0.88);
            let mut combs = 0.0f32;
            let o1 = state.rev_comb1[state.rev_comb1_idx];
            state.rev_comb1[state.rev_comb1_idx] = sample + o1 * fb * (1.0 - damp * 0.2);
            state.rev_comb1_idx = (state.rev_comb1_idx + 1) % 1116;
            combs += o1;
            let o2 = state.rev_comb2[state.rev_comb2_idx];
            state.rev_comb2[state.rev_comb2_idx] = sample + o2 * fb;
            state.rev_comb2_idx = (state.rev_comb2_idx + 1) % 1188;
            combs += o2;
            let out = combs * 0.5;
            (1.0 - mix) * sample + mix * out
        }

        BossEffectKind::HallReverb { size, mix, .. } => {
            let fb = (0.55 + 0.38 * size).clamp(0.4, 0.94);
            let mut combs = 0.0f32;
            let o1 = state.rev_comb1[state.rev_comb1_idx];
            state.rev_comb1[state.rev_comb1_idx] = sample + o1 * fb;
            state.rev_comb1_idx = (state.rev_comb1_idx + 1) % 1116;
            combs += o1;
            let o3 = state.rev_comb3[state.rev_comb3_idx];
            state.rev_comb3[state.rev_comb3_idx] = sample + o3 * fb;
            state.rev_comb3_idx = (state.rev_comb3_idx + 1) % 1277;
            combs += o3;
            let out = combs * 0.5;
            (1.0 - mix) * sample + mix * out
        }

        BossEffectKind::PlateReverb { decay, mix, .. } => {
            let fb = (0.45f32).powf(0.05 / decay.max(0.2)).clamp(0.3, 0.92);
            let ap1 = state.rev_ap1[state.rev_ap1_idx];
            state.rev_ap1[state.rev_ap1_idx] = sample + ap1 * fb;
            state.rev_ap1_idx = (state.rev_ap1_idx + 1) % 225;
            let ap2 = state.rev_ap2[state.rev_ap2_idx];
            state.rev_ap2[state.rev_ap2_idx] = ap1 + ap2 * fb;
            state.rev_ap2_idx = (state.rev_ap2_idx + 1) % 556;
            (1.0 - mix) * sample + mix * ap2
        }

        BossEffectKind::SpringReverb { decay, tension, mix } => {
            let fb = decay.clamp(0.2, 0.85);
            let o1 = state.rev_comb4[state.rev_comb4_idx];
            let spring_mod = (o1 * tension * 1.5).tanh();
            state.rev_comb4[state.rev_comb4_idx] = sample + spring_mod * fb;
            state.rev_comb4_idx = (state.rev_comb4_idx + 1) % 1356;
            (1.0 - mix) * sample + mix * o1
        }
    };
    if res.is_finite() {
        res.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
