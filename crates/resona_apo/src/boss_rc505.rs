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

    if eff_lower.contains("slicer") || eff_lower.contains("repeat") || eff_lower.contains("нарез") {
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
    } else if eff_lower.contains("vocoder") || eff_lower.contains("robot") || eff_lower.contains("вокод") {
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
    } else if eff_lower.contains("chorus") || eff_lower.contains("flanger") || eff_lower.contains("хорус") {
        let rate = json.get("rate").and_then(|v| v.as_f64()).unwrap_or(1.2) as f32;
        let depth = (json.get("depth").and_then(|v| v.as_f64()).unwrap_or(65.0) / 100.0) as f32;
        let feedback = (json.get("feedback").and_then(|v| v.as_f64()).unwrap_or(15.0) / 100.0) as f32;
        let mix = (json.get("mix").and_then(|v| v.as_f64()).unwrap_or(45.0) / 100.0) as f32;

        Some((BossEffectKind::Chorus { rate, depth, feedback, mix }, state))
    } else if eff_lower.contains("distortion") || eff_lower.contains("overdrive") || eff_lower.contains("перегруз") {
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
    match kind {
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
    }
}
