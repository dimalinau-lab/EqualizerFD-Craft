//! Real-Time Audio DSP Engine for ResonaAPO
//! Ultra-low latency, zero allocation per buffer

use crate::shared_mem::ResonaSharedMemory;

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
            // Band Pass (constant 0 dB peak gain)
            6 => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
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
    filters: [BiquadState; 16],
    geq_filters: [BiquadState; 15],
    geq_active: bool,
    active_filters_count: usize,
    preamp_linear: f32,
    pub lowcut_filter: BiquadState,
    pub expander_env: f32,
    pub rc505_effects: Vec<(crate::boss_rc505::BossEffectKind, crate::boss_rc505::BossEffectState)>,
}

#[inline(always)]
fn soft_limit(out_s: f32) -> f32 {
    let abs_s = out_s.abs();
    if abs_s <= 0.75 {
        out_s
    } else {
        let sign = out_s.signum();
        let excess = abs_s - 0.75;
        sign * (0.75 + 0.23 * (excess / (excess + 0.23)))
    }
}

#[inline]
fn process_channel_stage3<const GEQ: bool, const BOSS: bool>(
    ch: &mut ChannelDsp,
    samples: &mut [f32],
    channel_offset: usize,
    stride: usize,
    sample_rate: f32,
) -> f32 {
    let preamp = ch.preamp_linear;
    let active_filters = &mut ch.filters[..ch.active_filters_count];
    let mut peak_sq = 0.0f32;

    for s in samples[channel_offset..].iter_mut().step_by(stride) {
        let mut out_s = *s * preamp;
        for flt in active_filters.iter_mut() {
            out_s = flt.process(out_s);
        }
        if GEQ {
            for g in ch.geq_filters.iter_mut() {
                out_s = g.process(out_s);
            }
        }
        if BOSS {
            for (kind, state) in &mut ch.rc505_effects {
                out_s = crate::boss_rc505::process_boss_effect(kind, state, out_s, sample_rate);
            }
        }
        let limited = soft_limit(out_s);
        *s = limited;
        peak_sq = peak_sq.max(limited * limited);
    }

    peak_sq
}

use crate::ai_denoise::NeuralDenoiser;
use crate::vst_host::VstPluginInstance;

pub const RTA_FREQUENCIES_32: [f32; 32] = [
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0,
    125.0, 160.0, 200.0, 250.0, 315.0, 400.0, 500.0, 630.0,
    800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0,
    5000.0, 6300.0, 8000.0, 10000.0, 12500.0, 16000.0, 18000.0, 20000.0,
];

pub struct ResonaDspEngine {
    channels: Vec<ChannelDsp>,
    sample_rate: f32,
    last_synced_version: u32,
    pub vst_plugins: Vec<VstPluginInstance>,
    pub rnnoise: NeuralDenoiser,
    pub spectrum_bins: [f32; 32],
    rta_cached_n_samples: usize,
    rta_coeffs: [f32; 32],
    rta_mono_buf: [f32; 256],
}

impl ResonaDspEngine {
    pub fn new(sample_rate: f32, num_channels: usize) -> Self {
        let mut ch_vec = Vec::with_capacity(num_channels);
        for _ in 0..num_channels {
            let mut geq = [BiquadState::default(); 15];
            for (i, &f) in ISO_FREQUENCIES_15.iter().enumerate() {
                geq[i].calculate(0, f, 0.0, 1.4142, sample_rate);
            }
            let mut lowcut = BiquadState::default();
            lowcut.calculate(4, 80.0, 0.0, 0.7071, sample_rate); // High Pass 80 Hz Low-Cut
            ch_vec.push(ChannelDsp {
                filters: [BiquadState::default(); 16],
                geq_filters: geq,
                geq_active: false,
                active_filters_count: 0,
                preamp_linear: 1.0,
                lowcut_filter: lowcut,
                expander_env: 0.0,
                rc505_effects: Vec::new(),
            });
        }

        Self {
            channels: ch_vec,
            sample_rate,
            last_synced_version: 0,
            vst_plugins: Vec::new(),
            rnnoise: NeuralDenoiser::new(sample_rate, num_channels),
            spectrum_bins: [0.0; 32],
            rta_cached_n_samples: 0,
            rta_coeffs: [0.0; 32],
            rta_mono_buf: [0.0; 256],
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
        let geq_active = sm.geq_bands.iter().any(|&g| g.abs() > 1e-4);

        for ch in &mut self.channels {
            ch.preamp_linear = preamp_linear;

            let mut packed_cnt = 0usize;
            for i in 0..filter_cnt {
                let sf = &sm.filters[i];
                if sf.enabled != 0 {
                    ch.filters[packed_cnt].calculate(
                        sf.filter_type,
                        sf.fc,
                        sf.gain_db,
                        sf.q,
                        self.sample_rate,
                    );
                    packed_cnt += 1;
                }
            }
            ch.active_filters_count = packed_cnt;
            ch.geq_active = geq_active;

            if geq_active {
                for i in 0..15 {
                    ch.geq_filters[i].calculate(
                        0,
                        ISO_FREQUENCIES_15[i],
                        sm.geq_bands[i],
                        1.4142,
                        self.sample_rate,
                    );
                }
            }
        }
    }

    pub fn sync_from_config_text(&mut self, text: &str) {
        let expanded = eq_apo_core::ConfigFile::expand_includes(text, 0);
        if let Ok(config) = eq_apo_core::ConfigFile::parse(&expanded, self.sample_rate as f64) {
            let mut preamp_db = 0.0f32;
            let mut filter_slots = Vec::new();
            let mut geq_bands = [0.0f32; 15];

            let mut boss_effects = Vec::new();
            let mut vst_configs = Vec::new();
            let mut rnnoise_opt: Option<(bool, f32)> = None;

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
                    eq_apo_core::ConfigDirective::BossRc505 { effect_name, is_enabled, .. } => {
                        if *is_enabled {
                            if let Some((kind, state)) = crate::boss_rc505::parse_boss_directive(effect_name, self.sample_rate) {
                                boss_effects.push((kind, state));
                            }
                        }
                    }
                    eq_apo_core::ConfigDirective::VSTPlugin { library, vst_id, params_json, is_enabled, .. } => {
                        vst_configs.push((library.clone(), vst_id.clone(), params_json.clone(), *is_enabled));
                    }
                    eq_apo_core::ConfigDirective::RNNoise { is_enabled, strength, .. } => {
                        rnnoise_opt = Some((*is_enabled, *strength));
                    }
                    _ => {}
                }
            }

            let preamp_linear = 10.0f32.powf(preamp_db / 20.0);
            let geq_active = geq_bands.iter().any(|&g| g.abs() > 1e-4);
            for ch in &mut self.channels {
                ch.preamp_linear = preamp_linear;

                let mut packed_cnt = 0usize;
                for &(enabled, f_type, fc, gain_db, q) in &filter_slots {
                    if enabled {
                        ch.filters[packed_cnt].calculate(f_type, fc, gain_db, q, self.sample_rate);
                        packed_cnt += 1;
                    }
                }
                ch.active_filters_count = packed_cnt;
                ch.geq_active = geq_active;

                if geq_active {
                    for i in 0..15 {
                        ch.geq_filters[i].calculate(0, ISO_FREQUENCIES_15[i], geq_bands[i], 1.4142, self.sample_rate);
                    }
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

            if let Some((rn_enabled, rn_strength)) = rnnoise_opt {
                if !self.rnnoise.enabled && rn_enabled {
                    self.rnnoise.reset();
                }
                self.rnnoise.enabled = rn_enabled;
                self.rnnoise.strength = rn_strength;
                if rn_enabled {
                    self.rnnoise.ensure_loader_started();
                }
            } else {
                self.rnnoise.enabled = false;
            }
        }
    }

    #[inline]
    fn run_stage0_lowcut(&mut self, samples: &mut [f32], ch_cnt: usize) -> f32 {
        let mut in_peak_sq = 0.0f32;
        if ch_cnt == 2 {
            let mut l_lowcut = self.channels[0].lowcut_filter;
            let mut r_lowcut = self.channels[1].lowcut_filter;
            for frame in samples.chunks_exact_mut(2) {
                let raw_l = frame[0];
                let raw_r = frame[1];
                in_peak_sq = in_peak_sq.max(raw_l * raw_l).max(raw_r * raw_r);
                frame[0] = l_lowcut.process(raw_l);
                frame[1] = r_lowcut.process(raw_r);
            }
            self.channels[0].lowcut_filter = l_lowcut;
            self.channels[1].lowcut_filter = r_lowcut;
        } else {
            for (c, ch) in self.channels[..ch_cnt].iter_mut().enumerate() {
                let mut lowcut = ch.lowcut_filter;
                for s in samples[c..].iter_mut().step_by(ch_cnt) {
                    let raw_in = *s;
                    in_peak_sq = in_peak_sq.max(raw_in * raw_in);
                    *s = lowcut.process(raw_in);
                }
                ch.lowcut_filter = lowcut;
            }
        }
        in_peak_sq
    }

    fn run_post_stages_and_rta(&mut self, samples: &mut [f32], ch_cnt: usize, in_peak_sq: f32) -> (f32, f32) {
        let in_peak_db = if in_peak_sq > 1e-9 { 10.0 * in_peak_sq.log10() } else { -96.0 };
        let total_frames = samples.len() / ch_cnt;
        if total_frames == 0 {
            return (in_peak_db, -96.0);
        }

        let mut out_peak_sq = 0.0f32;

        // Stage 3: User Preamp, active Parametric EQ, optional Graphic EQ (bypassed when flat), Boss RC-505, Soft Limiter
        for (c, ch) in self.channels[..ch_cnt].iter_mut().enumerate() {
            let has_boss = !ch.rc505_effects.is_empty();
            let ch_peak = match (ch.geq_active, has_boss) {
                (false, false) => process_channel_stage3::<false, false>(ch, samples, c, ch_cnt, self.sample_rate),
                (true, false) => process_channel_stage3::<true, false>(ch, samples, c, ch_cnt, self.sample_rate),
                (false, true) => process_channel_stage3::<false, true>(ch, samples, c, ch_cnt, self.sample_rate),
                (true, true) => process_channel_stage3::<true, true>(ch, samples, c, ch_cnt, self.sample_rate),
            };
            out_peak_sq = out_peak_sq.max(ch_peak);
        }

        // Stage 2: VST 2.4 Plugins Chain (ReaGate, ReaComp, ReaDelay, etc.)
        let mut any_vst_active = false;
        for vst in &mut self.vst_plugins {
            if vst.enabled {
                any_vst_active = true;
                vst.process_interleaved(samples, ch_cnt, total_frames);
            }
        }
        if any_vst_active {
            out_peak_sq = 0.0;
            for &s in samples.iter() {
                out_peak_sq = out_peak_sq.max(s * s);
            }
        }

        let out_peak_db = if out_peak_sq > 1e-9 { 10.0 * out_peak_sq.log10() } else { -96.0 };

        // 32-Band Logarithmic Audio Spectrum Energy (Goertzel with cached cosines and single mono extraction)
        let step = (total_frames / 256).max(1);
        let n_samples = (total_frames / step).min(256);
        if n_samples >= 16 {
            let n_f32 = n_samples as f32;
            if self.rta_cached_n_samples != n_samples {
                for (i, &f_hz) in RTA_FREQUENCIES_32.iter().enumerate() {
                    let k = (n_f32 * f_hz / self.sample_rate).round();
                    let omega = 2.0 * std::f32::consts::PI * k / n_f32;
                    self.rta_coeffs[i] = 2.0 * omega.cos();
                }
                self.rta_cached_n_samples = n_samples;
            }

            if ch_cnt == 1 {
                for (dst, frame_idx) in self.rta_mono_buf[..n_samples]
                    .iter_mut()
                    .zip((0..total_frames).step_by(step))
                {
                    *dst = samples[frame_idx];
                }
            } else {
                for (dst, frame_idx) in self.rta_mono_buf[..n_samples]
                    .iter_mut()
                    .zip((0..total_frames).step_by(step))
                {
                    let base = frame_idx * ch_cnt;
                    *dst = (samples[base] + samples[base + 1]) * 0.5;
                }
            }

            let norm_factor = 4.0 / (n_f32 * n_f32);
            let mono = &self.rta_mono_buf[..n_samples];
            for (i, &f_hz) in RTA_FREQUENCIES_32.iter().enumerate() {
                if f_hz >= self.sample_rate * 0.49 {
                    self.spectrum_bins[i] = 0.0;
                    continue;
                }
                let coeff = self.rta_coeffs[i];

                let mut s1 = 0.0f32;
                let mut s2 = 0.0f32;
                for &mono_sample in mono {
                    let s0 = mono_sample + coeff * s1 - s2;
                    s2 = s1;
                    s1 = s0;
                }

                let power = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0);
                let rms_sq = (power * norm_factor).max(1e-10);
                let db = 10.0 * rms_sq.log10();
                // Map -70 dB..0 dB to 0.0..1.0
                let target_norm = ((db + 70.0) / 70.0).clamp(0.0, 1.0);

                if target_norm > self.spectrum_bins[i] {
                    self.spectrum_bins[i] = self.spectrum_bins[i] * 0.35 + target_norm * 0.65;
                } else {
                    self.spectrum_bins[i] = self.spectrum_bins[i] * 0.88 + target_norm * 0.12;
                }
            }
        }

        (in_peak_db, out_peak_db)
    }

    /// Process in-place fixed-length interleaved float audio buffer (COM sAPO path).
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

        let in_peak_sq = self.run_stage0_lowcut(samples, ch_cnt);
        self.rnnoise.process_interleaved(samples, ch_cnt);
        self.run_post_stages_and_rta(samples, ch_cnt, in_peak_sq)
    }

    /// Variable-length streaming processor for the WASAPI bridge: avoids zero-padding
    /// gaps when capture packet sizes do not align to 480-frame AI hops.
    pub fn process_interleaved_vec(
        &mut self,
        samples: &mut Vec<f32>,
        channels_count: usize,
    ) -> (f32, f32) {
        let ch_cnt = channels_count.min(self.channels.len());
        if ch_cnt == 0 || samples.is_empty() {
            return (-96.0, -96.0);
        }

        let in_peak_sq = self.run_stage0_lowcut(samples, ch_cnt);
        self.rnnoise.process_interleaved_vec(samples, ch_cnt);
        self.run_post_stages_and_rta(samples, ch_cnt, in_peak_sq)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_neural_denoiser_fallback_and_df3_ll() {
        let mut proc = NeuralDenoiser::new(48000.0, 2);
        assert!(!proc.enabled);

        proc.enabled = true;
        proc.strength = 0.85;

        // 1. Test odd/sub-hop buffer sizes and fallback RNNoise cleanly
        let mut odd_buf = vec![0.01f32; 256 * 2];
        proc.process_interleaved(&mut odd_buf, 2);
        assert_eq!(odd_buf.len(), 512);
        assert!(proc.speech_prob >= 0.0 && proc.speech_prob <= 1.0);

        // 2. Load DeepFilterNet3_ll model synchronously, measure exact algorithmic frame delay & speed
        let loaded = proc.load_sync_for_test();
        assert!(loaded, "DeepFilterNet3_ll_onnx.tar.gz must load from d:\\testik\\models\\");
        assert_eq!(proc.active_engine, crate::ai_denoise::AI_ENGINE_DEEPFILTER3_LL);

        proc.reset();

        // Feed 300 ms (30 hops x 480 frames) of room noise (55 Hz AC hum + broadband room hiss)
        // plus a loud sharp 5 ms mechanical keyboard tap at step 12.
        // Using ResonaDspEngine (includes 80 Hz High-Pass Stage 0 just like production!)
        let mut engine = ResonaDspEngine::new(48000.0, 2);
        assert!(engine.rnnoise.load_sync_for_test());
        engine.rnnoise.enabled = true;
        engine.rnnoise.strength = 0.85;

        let hop = 480;
        let num_channels = 2;
        let mut click_buf = vec![0.0f32; hop * num_channels];
        let mut max_click_out = 0.0f32;

        let t0 = Instant::now();
        for step in 0..30 {
            for f in 0..hop {
                let t = ((step * hop + f) as f32) / 48000.0;
                // 55 Hz mains AC hum + broadband room/fan hiss (typical noisy mic floor ~-38 dBFS)
                let hum = (2.0 * std::f32::consts::PI * 55.0 * t).sin() * 0.010;
                let hash = (((step * hop + f).wrapping_mul(1664525).wrapping_add(1013904223)) >> 16) as f32 / 65536.0 - 0.5;
                let room_noise = hum + hash * 0.015;
                // Sharp 5 ms mechanical keyboard tap around step 12 (peak 0.25 = -12 dBFS!)
                let click = if step == 12 && f < 240 {
                    let decay = (-((f as f32) / 50.0)).exp();
                    let hf = (2.0 * std::f32::consts::PI * 5200.0 * t).sin() * 0.7
                        + (2.0 * std::f32::consts::PI * 2400.0 * t).sin() * 0.3;
                    hf * decay * 0.25
                } else {
                    0.0
                };
                let sample = room_noise + click;
                click_buf[f * 2] = sample;
                click_buf[f * 2 + 1] = sample;
            }
            engine.process_interleaved(&mut click_buf, 2);
            if step >= 10 {
                for &s in &click_buf {
                    max_click_out = max_click_out.max(s.abs());
                }
            }
        }
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let avg_hop_ms = elapsed_ms / 30.0;
        println!(
            "[DeepFilterNet3_ll Bench] 300ms audio processed in {:.2} ms ({:.3} ms per 10ms hop, RTF={:.3}), max click leak={:.6}",
            elapsed_ms,
            avg_hop_ms,
            avg_hop_ms / 10.0,
            max_click_out
        );

        // Mechanical click + room hum must stay aggressively attenuated (< -40 dB)
        assert!(
            max_click_out < 0.01,
            "Room noise + keyboard click leaked too strongly: max_click_out = {}",
            max_click_out
        );
        // Must beat real-time comfortably
        assert!(avg_hop_ms < 8.5, "Inference too slow for 10ms real-time audio: {:.3} ms", avg_hop_ms);

        // 3. Verify genuine modulated vocal signal + background noise opens gate promptly and preserves speech clarity
        engine.rnnoise.reset();
        let mut speech_buf = vec![0.0f32; hop * num_channels];
        let mut out_speech_rms_acc = 0.0f64;
        let mut measured_hops = 0usize;
        let mut first_voice_out_step = None;

        for step in 0..35 {
            for f in 0..hop {
                let t = ((step * hop + f) as f32) / 48000.0;
                // Silence+hum for steps 0..4, dynamic speech onset starting at step 5
                let voice = if step >= 5 {
                    let syllabic_env = 0.60 + 0.40 * (2.0 * std::f32::consts::PI * 5.2 * (t - 0.05)).sin();
                    let f0 = 145.0 + 35.0 * (2.0 * std::f32::consts::PI * 3.8 * t).sin();
                    let f1 = 480.0 + 180.0 * (2.0 * std::f32::consts::PI * 2.7 * t).sin();
                    let f2 = 1350.0 + 320.0 * (2.0 * std::f32::consts::PI * 2.1 * t).sin();
                    let glottal = 0.08 * (2.0 * std::f32::consts::PI * f0 * t).sin()
                        + 0.06 * (2.0 * std::f32::consts::PI * f1 * t).sin()
                        + 0.04 * (2.0 * std::f32::consts::PI * f2 * t).sin()
                        + 0.02 * (2.0 * std::f32::consts::PI * 2450.0 * t).sin();
                    glottal * syllabic_env
                } else {
                    0.0
                };
                let fan_noise = (2.0 * std::f32::consts::PI * 55.0 * t).sin() * 0.008;
                let sample = voice + fan_noise;
                speech_buf[f * 2] = sample;
                speech_buf[f * 2 + 1] = sample;
            }
            engine.process_interleaved(&mut speech_buf, 2);
            let step_max = speech_buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            if first_voice_out_step.is_none() && step >= 5 && step_max > 0.01 {
                first_voice_out_step = Some(step);
            }
            if step >= 6 && step <= 14 {
                let sq: f64 = speech_buf.iter().step_by(2).map(|&s| (s as f64) * (s as f64)).sum();
                out_speech_rms_acc += sq / (hop as f64);
                measured_hops += 1;
            }
        }
        let out_speech_rms = (out_speech_rms_acc / (measured_hops as f64)).sqrt();
        let onset_lag_ms = first_voice_out_step.map(|s| (s - 5) * 10);
        println!(
            "[DeepFilterNet3_ll Bench] Voiced onset lag={:?}ms (from step 5), active_out_rms={:.4}, speech_prob={:.2}, lsnr={:.1} dB",
            onset_lag_ms, out_speech_rms, engine.rnnoise.speech_prob, engine.rnnoise.lsnr_db
        );
        assert!(
            onset_lag_ms.map_or(false, |ms| ms <= 20),
            "Speech onset lag exceeded 20 ms target: {:?}",
            onset_lag_ms
        );
        assert!(
            out_speech_rms > 0.02,
            "Genuine vocal speech was overly attenuated: out_speech_rms = {}",
            out_speech_rms
        );
    }

    #[test]
    fn test_sync_from_config_text_expands_include() {
        let mut engine = ResonaDspEngine::new(48000.0, 2);
        let cfg = "Include: d:\\testik\\preset_2.txt\n";
        engine.sync_from_config_text(cfg);
        assert!(
            engine.rnnoise.enabled,
            "Include directive should expand preset_2.txt and enable AI denoising"
        );
    }

    #[test]
    fn test_streaming_odd_packet_sizes_zero_gaps() {
        let mut engine = ResonaDspEngine::new(48000.0, 2);
        assert!(engine.rnnoise.load_sync_for_test());
        engine.rnnoise.enabled = true;
        engine.rnnoise.strength = 0.85;

        // Simulate realistic WASAPI/resampled capture packet sizes that do not divide 480
        let packet_sizes = [448usize, 487, 481, 479, 512, 448, 488, 480, 477, 485, 480, 480, 480, 480, 480];
        let mut total_in_frames = 0usize;
        let mut streamed_left: Vec<f32> = Vec::new();
        let mut phase = 0usize;

        for &pkt_frames in &packet_sizes {
            total_in_frames += pkt_frames;
            let mut pkt = vec![0.0f32; pkt_frames * 2];
            for f in 0..pkt_frames {
                let t = (phase as f32) / 48000.0;
                // Continuous multi-formant vocal signal
                let s = 0.08 * (2.0 * std::f32::consts::PI * 165.0 * t).sin()
                    + 0.05 * (2.0 * std::f32::consts::PI * 520.0 * t).sin()
                    + 0.03 * (2.0 * std::f32::consts::PI * 1400.0 * t).sin();
                pkt[f * 2] = s;
                pkt[f * 2 + 1] = s;
                phase += 1;
            }
            engine.process_interleaved_vec(&mut pkt, 2);
            assert_eq!(pkt.len() % (480 * 2), 0, "Output must align strictly to completed hops");
            for &l in pkt.iter().step_by(2) {
                streamed_left.push(l);
            }
        }

        let expected_frames = (total_in_frames / 480) * 480;
        assert_eq!(streamed_left.len(), expected_frames);

        // Once gate has opened (after initial 40ms warmup), verify no mid-speech zero-hole runs (>= 4 consecutive zeros)
        let mut zero_run = 0usize;
        let mut max_zero_run_after_warmup = 0usize;
        for &s in streamed_left.iter().skip(1920) {
            if s.abs() < 1e-6 {
                zero_run += 1;
                max_zero_run_after_warmup = max_zero_run_after_warmup.max(zero_run);
            } else {
                zero_run = 0;
            }
        }
        assert!(
            max_zero_run_after_warmup < 4,
            "Detected zero-hole silence gap in streamed speech: {} samples",
            max_zero_run_after_warmup
        );
    }
}
