//! Real-Time Audio DSP Engine for ResonaAPO
//! Ultra-low latency, zero allocation per buffer

use crate::shared_mem::{ResonaSharedMemory, SharedFilter};

const ISO_FREQUENCIES_15: [f32; 15] = [
    25.0, 40.0, 63.0, 100.0, 160.0, 250.0, 400.0, 630.0,
    1000.0, 1600.0, 2500.0, 4000.0, 6300.0, 10000.0, 16000.0,
];

#[derive(Clone, Copy, Default)]
pub struct BiquadState {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
    pub s1: f32,
    pub s2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.s1;
        self.s1 = self.b1 * input - self.a1 * output + self.s2;
        self.s2 = self.b2 * input - self.a2 * output;
        output
    }

    pub fn calculate(&mut self, filter_type: u32, fc: f32, gain_db: f32, q: f32, sample_rate: f32) {
        let f0 = (fc / sample_rate).clamp(0.0001, 0.499);
        let omega = 2.0 * std::f32::consts::PI * f0;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha = sn / (2.0 * q.max(0.01));
        let a = 10.0f32.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            // Peaking EQ
            0 => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cs;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            // Low Shelf
            1 => {
                let a_p1 = a + 1.0;
                let a_m1 = a - 1.0;
                let sqrt_a_2_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * (a_p1 - a_m1 * cs + sqrt_a_2_alpha);
                let b1 = 2.0 * a * (a_m1 - a_p1 * cs);
                let b2 = a * (a_p1 - a_m1 * cs - sqrt_a_2_alpha);
                let a0 = a_p1 + a_m1 * cs + sqrt_a_2_alpha;
                let a1 = -2.0 * (a_m1 + a_p1 * cs);
                let a2 = a_p1 + a_m1 * cs - sqrt_a_2_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            // High Shelf
            2 => {
                let a_p1 = a + 1.0;
                let a_m1 = a - 1.0;
                let sqrt_a_2_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * (a_p1 + a_m1 * cs + sqrt_a_2_alpha);
                let b1 = -2.0 * a * (a_m1 + a_p1 * cs);
                let b2 = a * (a_p1 + a_m1 * cs - sqrt_a_2_alpha);
                let a0 = a_p1 - a_m1 * cs + sqrt_a_2_alpha;
                let a1 = 2.0 * (a_m1 - a_p1 * cs);
                let a2 = a_p1 - a_m1 * cs - sqrt_a_2_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            // Low Pass
            3 => {
                let b0 = (1.0 - cs) / 2.0;
                let b1 = 1.0 - cs;
                let b2 = (1.0 - cs) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            // High Pass
            4 => {
                let b0 = (1.0 + cs) / 2.0;
                let b1 = -(1.0 + cs);
                let b2 = (1.0 + cs) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            // Notch
            5 => {
                let b0 = 1.0;
                let b1 = -2.0 * cs;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cs;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            _ => (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        };

        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }
}

pub struct ChannelDsp {
    filters: [BiquadFilterSlot; 16],
    geq_filters: [BiquadState; 15],
    active_filters_count: usize,
    preamp_linear: f32,
    pub rc505_effects: Vec<(crate::boss_rc505::BossEffectKind, crate::boss_rc505::BossEffectState)>,
}

#[derive(Clone, Copy, Default)]
struct BiquadFilterSlot {
    state: BiquadState,
    enabled: bool,
}

use crate::vst_host::VstPluginInstance;

pub struct ResonaDspEngine {
    channels: Vec<ChannelDsp>,
    sample_rate: f32,
    last_synced_version: u32,
    pub vst_plugins: Vec<VstPluginInstance>,
}

impl ResonaDspEngine {
    pub fn new(sample_rate: f32, num_channels: usize) -> Self {
        let mut ch_vec = Vec::with_capacity(num_channels);
        for _ in 0..num_channels {
            let mut geq = [BiquadState::default(); 15];
            for (i, &f) in ISO_FREQUENCIES_15.iter().enumerate() {
                geq[i].calculate(0, f, 0.0, 1.4142, sample_rate);
            }
            ch_vec.push(ChannelDsp {
                filters: [BiquadFilterSlot::default(); 16],
                geq_filters: geq,
                active_filters_count: 0,
                preamp_linear: 1.0,
                rc505_effects: Vec::new(),
            });
        }

        Self {
            channels: ch_vec,
            sample_rate,
            last_synced_version: 0,
            vst_plugins: Vec::new(),
        }
    }

    pub fn sync_from_shared_mem(&mut self, sm: &ResonaSharedMemory) {
        if self.last_synced_version == sm.version && self.last_synced_version > 0 {
            return;
        }

        self.last_synced_version = sm.version;

        if sm.config_len > 0 && (sm.config_len as usize) <= sm.config_bytes.len() {
            if let Ok(cfg_str) = std::str::from_utf8(&sm.config_bytes[..sm.config_len as usize]) {
                self.sync_from_config_text(cfg_str);
                return;
            }
        }

        let preamp_linear = 10.0f32.powf(sm.preamp_gain_db / 20.0);
        let filter_cnt = (sm.filter_count as usize).min(16);

        for ch in &mut self.channels {
            ch.preamp_linear = preamp_linear;
            ch.active_filters_count = filter_cnt;

            for i in 0..filter_cnt {
                let sf = &sm.filters[i];
                ch.filters[i].enabled = sf.enabled != 0;
                if ch.filters[i].enabled {
                    ch.filters[i].state.calculate(
                        sf.filter_type,
                        sf.fc,
                        sf.gain_db,
                        sf.q,
                        self.sample_rate,
                    );
                }
            }

            for i in 0..15 {
                let gain = sm.geq_bands[i];
                ch.geq_filters[i].calculate(
                    0,
                    ISO_FREQUENCIES_15[i],
                    gain,
                    1.4142,
                    self.sample_rate,
                );
            }
        }
    }

    pub fn sync_from_config_text(&mut self, text: &str) {
        if let Ok(config) = eq_apo_core::ConfigFile::parse(text, self.sample_rate as f64) {
            let mut preamp_db = 0.0f32;
            let mut filter_slots = Vec::new();
            let mut geq_bands = [0.0f32; 15];

            let mut boss_effects = Vec::new();
            let mut vst_configs = Vec::new();

            for directive in &config.directives {
                match directive {
                    eq_apo_core::ConfigDirective::Preamp { gain_db, .. } => {
                        preamp_db += *gain_db as f32;
                    }
                    eq_apo_core::ConfigDirective::Filter { is_enabled, params, .. } => {
                        if filter_slots.len() < 16 {
                            let f_type = match params.filter_type {
                                eq_apo_core::FilterType::Peaking => 0,
                                eq_apo_core::FilterType::LowShelf => 1,
                                eq_apo_core::FilterType::HighShelf => 2,
                                eq_apo_core::FilterType::LowPass => 3,
                                eq_apo_core::FilterType::HighPass => 4,
                                eq_apo_core::FilterType::Notch => 5,
                                _ => 0,
                            };
                            filter_slots.push((*is_enabled, f_type, params.frequency_hz as f32, params.gain_db as f32, params.q_factor as f32));
                        }
                    }
                    eq_apo_core::ConfigDirective::GraphicEQ { points, .. } => {
                        let geq_pts: Vec<eq_apo_core::GraphicEqPoint> = points
                            .iter()
                            .map(|&(freq, gain)| eq_apo_core::GraphicEqPoint { freq_hz: freq, gain_db: gain })
                            .collect();
                        let geq = eq_apo_core::GraphicEQ::new(geq_pts);
                        for (i, &f) in ISO_FREQUENCIES_15.iter().enumerate() {
                            geq_bands[i] = geq.evaluate_gain_at(f as f64) as f32;
                        }
                    }
                    eq_apo_core::ConfigDirective::BossRc505 { effect_name, .. } => {
                        if let Some((kind, state)) = crate::boss_rc505::parse_boss_directive(effect_name, self.sample_rate) {
                            boss_effects.push((kind, state));
                        }
                    }
                    eq_apo_core::ConfigDirective::VSTPlugin { library, vst_id, params_json, is_enabled, .. } => {
                        vst_configs.push((library.clone(), vst_id.clone(), params_json.clone(), *is_enabled));
                    }
                    _ => {}
                }
            }

            let preamp_linear = 10.0f32.powf(preamp_db / 20.0);
            for ch in &mut self.channels {
                ch.preamp_linear = preamp_linear;
                ch.active_filters_count = filter_slots.len();

                for (i, &(enabled, f_type, fc, gain_db, q)) in filter_slots.iter().enumerate() {
                    ch.filters[i].enabled = enabled;
                    if enabled {
                        ch.filters[i].state.calculate(f_type, fc, gain_db, q, self.sample_rate);
                    }
                }

                for i in 0..15 {
                    ch.geq_filters[i].calculate(0, ISO_FREQUENCIES_15[i], geq_bands[i], 1.4142, self.sample_rate);
                }

                ch.rc505_effects = boss_effects.clone();
            }

            // Sync and retain loaded VST plugin instances
            let mut updated_vst = Vec::with_capacity(vst_configs.len());
            for (lib_path, vst_id_opt, params_json_opt, enabled) in vst_configs {
                let resolved_path = resolve_vst_path(&lib_path);
                let id_str = vst_id_opt.unwrap_or_else(|| {
                    std::path::Path::new(&resolved_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("plugin")
                        .to_string()
                });

                let existing_idx = self.vst_plugins.iter().position(|p| {
                    p.library_path.eq_ignore_ascii_case(&resolved_path)
                });

                let mut instance = if let Some(idx) = existing_idx {
                    self.vst_plugins.swap_remove(idx)
                } else {
                    match VstPluginInstance::load(&resolved_path, self.sample_rate, 4096) {
                        Ok(inst) => {
                            println!("[ResonaDSP] Loaded VST plugin: '{}' ({})", resolved_path, id_str);
                            inst
                        }
                        Err(e) => {
                            eprintln!("[ResonaDSP] Failed to load VST '{}': {}", resolved_path, e);
                            continue;
                        }
                    }
                };

                instance.enabled = enabled;
                if let Some(ref pj) = params_json_opt {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(pj) {
                        instance.apply_json_params(&id_str, &val);
                    }
                }
                updated_vst.push(instance);
            }
            self.vst_plugins = updated_vst;
        }
    }

    /// Process in-place interleaved float audio buffer
    /// samples: slice of (frames * channels) floats
    pub fn process_interleaved(
        &mut self,
        samples: &mut [f32],
        channels_count: usize,
    ) -> (f32, f32) {
        let ch_cnt = channels_count.min(self.channels.len());
        if ch_cnt == 0 || samples.is_empty() {
            return (-96.0, -96.0);
        }

        let mut in_peak_sq = 0.0f32;
        let mut out_peak_sq = 0.0f32;

        let total_frames = samples.len() / ch_cnt;

        // Stage 1: Preamp, Biquad Filters, Graphic EQ, Boss RC-505
        for frame in 0..total_frames {
            for c in 0..ch_cnt {
                let idx = frame * ch_cnt + c;
                let input = samples[idx];
                in_peak_sq = in_peak_sq.max(input * input);

                let ch = &mut self.channels[c];

                // 1. Preamp
                let mut s = input * ch.preamp_linear;

                // 2. Parametric Filters
                for f_idx in 0..ch.active_filters_count {
                    if ch.filters[f_idx].enabled {
                        s = ch.filters[f_idx].state.process(s);
                    }
                }

                // 3. Graphic EQ
                for g_idx in 0..15 {
                    s = ch.geq_filters[g_idx].process(s);
                }

                // 4. Boss RC-505 Mk2 Real-Time Effects
                for (kind, state) in &mut ch.rc505_effects {
                    s = crate::boss_rc505::process_boss_effect(kind, state, s, self.sample_rate);
                }

                // 5. Soft Limiter (prevent harsh digital wraps)
                s = s.clamp(-1.0, 1.0);

                samples[idx] = s;
            }
        }

        // Stage 2: VST 2.4 Plugins Chain (ReaGate, ReaComp, ReaDelay, etc.)
        for vst in &mut self.vst_plugins {
            if vst.enabled {
                vst.process_interleaved(samples, ch_cnt, total_frames);
            }
        }

        // Measure final output peak across all processed samples
        for &s in samples.iter() {
            out_peak_sq = out_peak_sq.max(s * s);
        }

        let in_peak_db = if in_peak_sq > 1e-9 { 10.0 * in_peak_sq.log10() } else { -96.0 };
        let out_peak_db = if out_peak_sq > 1e-9 { 10.0 * out_peak_sq.log10() } else { -96.0 };

        (in_peak_db, out_peak_db)
    }
}

fn resolve_vst_path(lib: &str) -> String {
    let clean = lib.trim().trim_matches('"').replace(r"\\", r"\");
    let p = std::path::Path::new(&clean);
    if p.exists() {
        return clean;
    }
    let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or(&clean);
    let candidates = [
        format!(r"D:\testik\vst_plugins\{}", file_name),
        format!(r"C:\Program Files\EqualizerAPO\vstplugins\{}", file_name),
        format!(r"vst_plugins\{}", file_name),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.clone();
        }
    }
    clean
}
