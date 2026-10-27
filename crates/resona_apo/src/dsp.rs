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

pub struct RnNoiseChannel {
    state: Box<nnnoiseless::DenoiseState<'static>>,
    in_fifo: Vec<f32>,
    out_fifo: std::collections::VecDeque<f32>,
}

impl RnNoiseChannel {
    pub fn new() -> Self {
        let mut out_fifo = std::collections::VecDeque::with_capacity(1024);
        for _ in 0..480 {
            out_fifo.push_back(0.0);
        }
        Self {
            state: nnnoiseless::DenoiseState::new(),
            in_fifo: Vec::with_capacity(1024),
            out_fifo,
        }
    }

    pub fn reset(&mut self) {
        self.state = nnnoiseless::DenoiseState::new();
        self.in_fifo.clear();
        self.out_fifo.clear();
        for _ in 0..480 {
            self.out_fifo.push_back(0.0);
        }
    }
}

pub struct RnNoiseProcessor {
    pub enabled: bool,
    pub strength: f32,
    channels: Vec<RnNoiseChannel>,
    pub speech_prob: f32,
    scratch_in: [f32; 480],
    scratch_out: [f32; 480],
}

impl RnNoiseProcessor {
    pub fn new(num_channels: usize) -> Self {
        let count = num_channels.max(1);
        let mut chs = Vec::with_capacity(count);
        for _ in 0..count {
            chs.push(RnNoiseChannel::new());
        }
        Self {
            enabled: false,
            strength: 1.0,
            channels: chs,
            speech_prob: 0.0,
            scratch_in: [0.0; 480],
            scratch_out: [0.0; 480],
        }
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32], channels_count: usize) {
        if !self.enabled || samples.is_empty() {
            return;
        }

        let ch_cnt = channels_count.min(self.channels.len());
        if ch_cnt == 0 {
            return;
        }

        let total_frames = samples.len() / ch_cnt;
        let strength = self.strength.clamp(0.0, 1.0);

        for c in 0..ch_cnt {
            let ch_data = &mut self.channels[c];
            for frame in 0..total_frames {
                ch_data.in_fifo.push(samples[frame * ch_cnt + c]);
            }

            while ch_data.in_fifo.len() >= 480 {
                for i in 0..480 {
                    self.scratch_in[i] = (ch_data.in_fifo[i] * 32768.0).clamp(-32768.0, 32767.0);
                }

                let prob = ch_data.state.process_frame(&mut self.scratch_out, &self.scratch_in);
                if c == 0 {
                    self.speech_prob = prob;
                }

                for i in 0..480 {
                    let wet = (self.scratch_out[i] / 32768.0).clamp(-1.0, 1.0);
                    let dry = ch_data.in_fifo[i];
                    let mixed = dry * (1.0 - strength) + wet * strength;
                    ch_data.out_fifo.push_back(mixed);
                }

                ch_data.in_fifo.drain(0..480);
            }

            for frame in 0..total_frames {
                if let Some(val) = ch_data.out_fifo.pop_front() {
                    samples[frame * ch_cnt + c] = val;
                }
            }
        }
    }
}

use crate::vst_host::VstPluginInstance;

pub struct ResonaDspEngine {
    channels: Vec<ChannelDsp>,
    sample_rate: f32,
    last_synced_version: u32,
    pub vst_plugins: Vec<VstPluginInstance>,
    pub rnnoise: RnNoiseProcessor,
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
            rnnoise: RnNoiseProcessor::new(num_channels),
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

            if let Some((rn_enabled, rn_strength)) = rnnoise_opt {
                if !self.rnnoise.enabled && rn_enabled {
                    for ch in &mut self.rnnoise.channels {
                        ch.reset();
                    }
                }
                self.rnnoise.enabled = rn_enabled;
                self.rnnoise.strength = rn_strength;
            } else {
                self.rnnoise.enabled = false;
            }
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

        // Stage 0: AI Neural Noise Suppression (RNNoise)
        self.rnnoise.process_interleaved(samples, ch_cnt);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rnnoise_processor_execution() {
        let mut proc = RnNoiseProcessor::new(2);
        assert!(!proc.enabled);

        proc.enabled = true;
        proc.strength = 1.0;

        // Create 2 channels of 960 frames (20 ms at 48kHz, exactly two 480-sample RNNoise frames)
        let num_frames = 960;
        let num_channels = 2;
        let mut samples = vec![0.0f32; num_frames * num_channels];

        // Fill with low-level random/sine noise
        for (i, s) in samples.iter_mut().enumerate() {
            let f = (i / 2) as f32;
            *s = (f * 0.05).sin() * 0.05;
        }

        let orig_len = samples.len();
        proc.process_interleaved(&mut samples, num_channels);

        assert_eq!(samples.len(), orig_len);
        assert!(proc.speech_prob >= 0.0 && proc.speech_prob <= 1.0);
    }
}
