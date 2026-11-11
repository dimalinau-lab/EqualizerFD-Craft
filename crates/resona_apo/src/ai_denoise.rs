//! Next-Gen Neural AI Noise Suppression Engine (DeepFilterNet3 Low-Latency + RNNoise Fallback)
//!
//! Combines `DeepFilterNet3_ll` (48 kHz full-band, 10 ms hop, 0-frame lookahead) via pure-Rust
//! `tract` with a multi-criterion Balanced Neural Guard (Local SNR + vocal formant balance +
//! transient anti-click onset + smooth hysteresis) for Krisp-level noise immunity at 10-20 ms latency.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, OnceLock};

use df::tract::{DfParams, DfTract, ReduceMask, RuntimeParams};
use ndarray::Array2;

pub const AI_FRAME_SIZE: usize = 480; // 10 ms @ 48 kHz
pub const AI_ENGINE_RNNOISE: u32 = 0;
pub const AI_ENGINE_DEEPFILTER3_LL: u32 = 1;

static CACHED_DF_PARAMS: OnceLock<Option<Arc<DfParams>>> = OnceLock::new();

/// Locates and unpacks `DeepFilterNet3_ll_onnx.tar.gz` once per process.
fn find_model_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(8);

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("models").join("DeepFilterNet3_ll_onnx.tar.gz"));
            candidates.push(dir.join("DeepFilterNet3_ll_onnx.tar.gz"));
        }
    }

    candidates.push(PathBuf::from(r"d:\testik\models\DeepFilterNet3_ll_onnx.tar.gz"));
    candidates.push(PathBuf::from(
        r"C:\Program Files\EqualizerAPO\models\DeepFilterNet3_ll_onnx.tar.gz",
    ));
    candidates.push(PathBuf::from("models/DeepFilterNet3_ll_onnx.tar.gz"));

    candidates.into_iter().find(|p| p.is_file())
}

fn get_cached_df_params() -> Option<Arc<DfParams>> {
    CACHED_DF_PARAMS
        .get_or_init(|| {
            let path = find_model_path()?;
            match DfParams::new(path.clone()) {
                Ok(params) => {
                    println!(
                        "[ResonaDSP AI] Loaded DeepFilterNet3_ll model package from '{}'",
                        path.display()
                    );
                    Some(Arc::new(params))
                }
                Err(err) => {
                    eprintln!(
                        "[ResonaDSP AI] Failed to parse model '{}': {:?}",
                        path.display(),
                        err
                    );
                    None
                }
            }
        })
        .clone()
}

#[inline]
fn compute_tuning(strength: f32) -> (f32, f32, f32) {
    let s = strength.clamp(0.0, 1.0);
    let atten_lim_db = if s >= 0.85 {
        100.0
    } else {
        15.0 + s * 70.0
    };
    let post_filter_beta = 0.015 + s * 0.025; // ~0.036-0.040 at 85-100%
    // Keep DfTract's hard-zero cutoff strictly below lsnr_min (-15 dB) so DeepFilterNet always
    // applies smooth ERB masks without punching zero holes mid-word; SpeechGuard handles clean pause muting.
    let min_db_thresh = -16.0;
    (atten_lim_db, post_filter_beta, min_db_thresh)
}

fn build_runtime_params(strength: f32) -> RuntimeParams {
    let (atten_lim_db, post_filter_beta, min_db_thresh) = compute_tuning(strength);

    RuntimeParams::new(
        1,
        post_filter_beta,
        atten_lim_db,
        min_db_thresh,
        35.0,
        24.0,
        ReduceMask::MEAN,
    )
}

fn load_df_tract_for_strength(strength: f32) -> Option<DfTract> {
    let params = get_cached_df_params()?;
    let rp = build_runtime_params(strength);
    match DfTract::new((*params).clone(), &rp) {
        Ok(m) => Some(m),
        Err(err) => {
            eprintln!("[ResonaDSP AI] Failed to init DfTract runtime: {:?}", err);
            None
        }
    }
}

/// Simple zero-allocation 1-pole filter for vocal vs transient band probes.
#[derive(Clone, Copy, Default)]
struct OnePoleProbe {
    lp_prev: f32,
    hp_prev_in: f32,
    hp_prev_out: f32,
}

impl OnePoleProbe {
    #[inline]
    fn highpass(&mut self, x: f32, alpha: f32) -> f32 {
        let y = alpha * (self.hp_prev_out + x - self.hp_prev_in);
        self.hp_prev_in = x;
        self.hp_prev_out = y;
        y
    }

    #[inline]
    fn lowpass(&mut self, x: f32, beta: f32) -> f32 {
        self.lp_prev += beta * (x - self.lp_prev);
        self.lp_prev
    }
}

/// Balanced Neural Speech Guard (Discord Krisp-style hysteresis + adaptive floor + anti-click gate).
pub struct SpeechGuard {
    hp_voice: OnePoleProbe,
    lp_voice: OnePoleProbe,
    hp_high: OnePoleProbe,
    noise_floor_rms: f32,
    hold_frames_remaining: usize,
    confirm_streak: usize,
    gate_gain_smooth: f32,
    alpha_voice_hp: f32,
    beta_voice_lp: f32,
    alpha_hf_hp: f32,
    att_coef: f32,
    rel_coef: f32,
}

impl SpeechGuard {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8000.0);
        let dt = 1.0 / sr;
        let rc_coef = |fc: f32| -> f32 {
            let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
            rc / (rc + dt)
        };
        let lp_coef = |fc: f32| -> f32 {
            let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
            dt / (rc + dt)
        };

        Self {
            hp_voice: OnePoleProbe::default(),
            lp_voice: OnePoleProbe::default(),
            hp_high: OnePoleProbe::default(),
            noise_floor_rms: 0.002,
            hold_frames_remaining: 0,
            confirm_streak: 0,
            gate_gain_smooth: 0.0001,
            alpha_voice_hp: rc_coef(180.0),
            beta_voice_lp: lp_coef(3600.0),
            alpha_hf_hp: rc_coef(5000.0),
            att_coef: (-dt / 0.0015).exp(), // 1.5 ms fast onset
            rel_coef: (-dt / 0.065).exp(),  // 65 ms organic fadeout
        }
    }

    pub fn reset(&mut self) {
        self.hp_voice = OnePoleProbe::default();
        self.lp_voice = OnePoleProbe::default();
        self.hp_high = OnePoleProbe::default();
        self.noise_floor_rms = 0.002;
        self.hold_frames_remaining = 0;
        self.confirm_streak = 0;
        self.gate_gain_smooth = 0.0001;
    }

    /// Evaluates one 10 ms frame and returns `(speech_prob_0_to_1, target_gate_gain)`.
    pub fn evaluate_frame(
        &mut self,
        enhanced_mono: &[f32],
        lsnr_db: f32,
        vad_prob: f32,
        is_fallback_mode: bool,
        strength: f32,
    ) -> (f32, f32) {
        if enhanced_mono.is_empty() {
            return (0.0, 1.0);
        }

        let mut sum_sq = 0.0f32;
        let mut voice_sq = 0.0f32;
        let mut hf_sq = 0.0f32;
        let mut peak_abs = 0.0f32;

        for &s in enhanced_mono {
            let abs_s = s.abs();
            if abs_s > peak_abs {
                peak_abs = abs_s;
            }
            sum_sq += s * s;

            let v_hp = self.hp_voice.highpass(s, self.alpha_voice_hp);
            let v_bp = self.lp_voice.lowpass(v_hp, self.beta_voice_lp);
            voice_sq += v_bp * v_bp;

            let hf = self.hp_high.highpass(s, self.alpha_hf_hp);
            hf_sq += hf * hf;
        }

        let n = enhanced_mono.len() as f32;
        let total_rms = (sum_sq / n).sqrt();
        let voice_rms = (voice_sq / n).sqrt();
        let hf_rms = (hf_sq / n).sqrt();
        let crest = peak_abs / total_rms.max(1e-7);
        let voice_ratio = voice_rms / total_rms.max(1e-7);

        // Adaptive residual noise floor tracker (tracks quiet background fast down, very slowly up)
        if vad_prob < 0.35 && voice_rms < 0.015 {
            let alpha = if voice_rms < self.noise_floor_rms { 0.12 } else { 0.025 };
            self.noise_floor_rms += alpha * (voice_rms - self.noise_floor_rms);
            self.noise_floor_rms = self.noise_floor_rms.clamp(0.0002, 0.012);
        }

        let snr_prob = ((lsnr_db + 9.0) / 15.0).clamp(0.0, 1.0);
        // Combined probability merges neural speech VAD + DeepFilterNet local SNR
        let prob = if is_fallback_mode {
            vad_prob
        } else {
            (vad_prob * 0.72 + snr_prob * 0.28).clamp(0.0, 1.0)
        };

        // Mechanical clicks, keyboard taps & mouse snaps: high crest factor or HF dominated, OR low neural VAD
        let is_transient_click = (crest > 4.2 && (hf_rms > voice_rms * 0.55 || voice_ratio < 0.50))
            || (hf_rms > voice_rms * 1.15 && self.hold_frames_remaining == 0)
            || vad_prob < 0.22;

        // Require vocal energy genuinely above adaptive residual noise floor + neural speech VAD
        let min_voice_energy = (self.noise_floor_rms * 2.2).max(0.0035);

        // Instant zero-clipping open requires dominant vocal-band energy + neural VAD + non-impulse shape
        let definite_speech = !is_transient_click
            && vad_prob >= 0.48
            && voice_rms >= min_voice_energy
            && voice_ratio >= 0.48
            && lsnr_db >= -5.0;

        // Once gate is open, preserve softer consonants/sibilants/tails smoothly (balanced 4B)
        let soft_tail_speech = (vad_prob >= 0.30 || lsnr_db >= 1.0)
            && voice_rms >= (min_voice_energy * 0.40)
            && !is_transient_click;

        // Hangover ~180 ms (18 x 10ms hops) balanced for natural word endings
        let hold_frames = 18;

        if self.hold_frames_remaining > 0 {
            if definite_speech || soft_tail_speech {
                self.hold_frames_remaining = hold_frames;
            } else {
                self.hold_frames_remaining -= 1;
            }
            self.confirm_streak = 0;
        } else if definite_speech {
            // Immediate zero-clipping onset for genuine voice
            self.hold_frames_remaining = hold_frames;
            self.confirm_streak = 0;
        } else if vad_prob >= 0.40 && voice_rms >= (min_voice_energy * 0.8) && !is_transient_click {
            // Borderline soft voice onset persists >= 20 ms to ignore sub-frame transients
            self.confirm_streak += 1;
            if self.confirm_streak >= 2 {
                self.hold_frames_remaining = hold_frames;
                self.confirm_streak = 0;
            }
        } else {
            self.confirm_streak = 0;
        }

        let s = strength.clamp(0.0, 1.0);
        let closed_floor = if s >= 0.80 {
            0.0001 // -80 dB studio silence
        } else {
            10.0f32.powf(-(24.0 + (s / 0.80) * 56.0) / 20.0)
        };

        let (target_gain, ui_prob) = if self.hold_frames_remaining > 0 {
            (1.0, prob.max(0.45))
        } else {
            (closed_floor, (prob * 0.35).min(0.28))
        };

        (ui_prob, target_gain)
    }

    pub fn is_gate_open(&self) -> bool {
        self.hold_frames_remaining > 0
    }

    #[inline]
    pub fn smooth_sample(&mut self, target_gain: f32) -> f32 {
        let coef = if target_gain > self.gate_gain_smooth {
            self.att_coef
        } else {
            self.rel_coef
        };
        self.gate_gain_smooth = target_gain + coef * (self.gate_gain_smooth - target_gain);
        self.gate_gain_smooth
    }
}

struct FallbackRnNoise {
    state: Box<nnnoiseless::DenoiseState<'static>>,
    scratch_in: [f32; AI_FRAME_SIZE],
    scratch_out: [f32; AI_FRAME_SIZE],
}

impl FallbackRnNoise {
    fn new() -> Self {
        Self {
            state: nnnoiseless::DenoiseState::new(),
            scratch_in: [0.0; AI_FRAME_SIZE],
            scratch_out: [0.0; AI_FRAME_SIZE],
        }
    }

    fn reset(&mut self) {
        self.state = nnnoiseless::DenoiseState::new();
    }

    fn process_hop(&mut self, input: &[f32], output: &mut [f32], strength: f32) -> f32 {
        for i in 0..AI_FRAME_SIZE {
            self.scratch_in[i] = (input[i] * 32768.0).clamp(-32768.0, 32767.0);
        }
        let prob = self.state.process_frame(&mut self.scratch_out, &self.scratch_in);
        for i in 0..AI_FRAME_SIZE {
            let wet = (self.scratch_out[i] / 32768.0).clamp(-1.0, 1.0);
            output[i] = input[i] * (1.0 - strength) + wet * strength;
        }
        prob
    }

    /// Computes speech probability only (0..1) without modifying caller's audio buffer.
    fn vad_only_prob(&mut self, clean_input: &[f32]) -> f32 {
        for i in 0..AI_FRAME_SIZE {
            self.scratch_in[i] = (clean_input[i] * 32768.0).clamp(-32768.0, 32767.0);
        }
        self.state.process_frame(&mut self.scratch_out, &self.scratch_in)
    }
}

pub struct NeuralDenoiser {
    pub enabled: bool,
    pub strength: f32,
    pub speech_prob: f32,
    pub lsnr_db: f32,
    pub active_engine: u32,

    sample_rate: f32,
    channels_count: usize,

    df_primary: Option<DfTract>,
    df_secondary: Option<DfTract>,
    df_loader_rx: Option<Receiver<Arc<DfParams>>>,
    df_loader_spawned: bool,
    last_applied_strength: f32,

    rn_primary: FallbackRnNoise,
    rn_secondary: FallbackRnNoise,

    in_fifo_l: Vec<f32>,
    in_fifo_r: Vec<f32>,
    out_fifo_l: VecDeque<f32>,
    out_fifo_r: VecDeque<f32>,

    hop_in_arr: Array2<f32>,
    hop_out_arr: Array2<f32>,
    mono_out_hop: [f32; AI_FRAME_SIZE],
    stereo_out_hop_r: [f32; AI_FRAME_SIZE],
    gain_fifo: VecDeque<f32>,
    current_target_gain: f32,
    guard: SpeechGuard,
    true_stereo_streak: usize,
}

impl NeuralDenoiser {
    pub fn new(sample_rate: f32, num_channels: usize) -> Self {
        Self {
            enabled: false,
            strength: 1.0,
            speech_prob: 0.0,
            lsnr_db: -15.0,
            active_engine: AI_ENGINE_RNNOISE,
            sample_rate,
            channels_count: num_channels.max(1),
            df_primary: None,
            df_secondary: None,
            df_loader_rx: None,
            df_loader_spawned: false,
            last_applied_strength: -1.0,
            rn_primary: FallbackRnNoise::new(),
            rn_secondary: FallbackRnNoise::new(),
            in_fifo_l: Vec::with_capacity(2048),
            in_fifo_r: Vec::with_capacity(2048),
            out_fifo_l: VecDeque::with_capacity(2048),
            out_fifo_r: VecDeque::with_capacity(2048),
            hop_in_arr: Array2::zeros((1, AI_FRAME_SIZE)),
            hop_out_arr: Array2::zeros((1, AI_FRAME_SIZE)),
            mono_out_hop: [0.0; AI_FRAME_SIZE],
            stereo_out_hop_r: [0.0; AI_FRAME_SIZE],
            gain_fifo: VecDeque::with_capacity(2048),
            current_target_gain: 0.0001,
            guard: SpeechGuard::new(sample_rate),
            true_stereo_streak: 0,
        }
    }

    /// Triggers background tar.gz extraction of `DfParams` without blocking audio callbacks.
    pub fn ensure_loader_started(&mut self) {
        if self.df_primary.is_some() || self.df_loader_spawned {
            return;
        }
        if (self.sample_rate - 48000.0).abs() > 150.0 {
            return;
        }
        if CACHED_DF_PARAMS.get().and_then(|o| o.as_ref()).is_some() {
            let _ = self.preload_sync();
            return;
        }
        if find_model_path().is_none() {
            return;
        }

        self.df_loader_spawned = true;
        let (tx, rx) = mpsc::sync_channel(1);
        self.df_loader_rx = Some(rx);

        std::thread::Builder::new()
            .name("resona_df3_loader".to_string())
            .spawn(move || {
                if let Some(params) = get_cached_df_params() {
                    let _ = tx.send(params);
                }
            })
            .ok();
    }

    /// Synchronously initializes `DeepFilterNet3_ll` on the current thread (ideal before capture starts or in tests).
    pub fn preload_sync(&mut self) -> bool {
        if self.df_primary.is_some() {
            return true;
        }
        if (self.sample_rate - 48000.0).abs() > 150.0 {
            return false;
        }
        if let Some(model) = load_df_tract_for_strength(self.strength) {
            self.df_primary = Some(model);
            self.active_engine = AI_ENGINE_DEEPFILTER3_LL;
            self.last_applied_strength = self.strength;
            self.df_loader_rx = None;
            println!("[ResonaDSP AI] DeepFilterNet3_ll (48 kHz, 10ms Zero-Lookahead) ACTIVE!");
            return true;
        }
        false
    }

    /// Backwards-compatible alias for synchronous test initialization.
    pub fn load_sync_for_test(&mut self) -> bool {
        self.preload_sync()
    }

    fn poll_loaded_model(&mut self) {
        if let Some(ref rx) = self.df_loader_rx {
            match rx.try_recv() {
                Ok(params) => {
                    let rp = build_runtime_params(self.strength);
                    if let Ok(model) = DfTract::new((*params).clone(), &rp) {
                        self.df_primary = Some(model);
                        self.active_engine = AI_ENGINE_DEEPFILTER3_LL;
                        self.last_applied_strength = -1.0; // force parameter sync
                        println!("[ResonaDSP AI] DeepFilterNet3_ll (48 kHz, 10ms Zero-Lookahead) ACTIVE!");
                    }
                    self.df_loader_rx = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.df_loader_rx = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    fn sync_model_strength(&mut self) {
        if (self.strength - self.last_applied_strength).abs() < 0.01 {
            return;
        }
        let s = self.strength.clamp(0.0, 1.0);
        self.last_applied_strength = s;

        let (atten_lim_db, pf_beta, min_db) = compute_tuning(s);

        if let Some(ref mut df) = self.df_primary {
            df.set_atten_lim(atten_lim_db);
            df.set_pf_beta(pf_beta);
            df.min_db_thresh = min_db;
        }
        if let Some(ref mut df2) = self.df_secondary {
            df2.set_atten_lim(atten_lim_db);
            df2.set_pf_beta(pf_beta);
            df2.min_db_thresh = min_db;
        }
    }

    pub fn reset(&mut self) {
        self.in_fifo_l.clear();
        self.in_fifo_r.clear();
        self.out_fifo_l.clear();
        self.out_fifo_r.clear();
        self.gain_fifo.clear();
        self.rn_primary.reset();
        self.rn_secondary.reset();
        if self.df_primary.is_some() {
            // Re-instantiate cleanly from cached Arc<DfParams> (upstream DfTract::init() leaks rolling_spec_buf_x)
            self.df_primary = load_df_tract_for_strength(self.strength);
            self.last_applied_strength = self.strength;
        }
        self.df_secondary = None;
        self.guard.reset();
        self.speech_prob = 0.0;
        self.lsnr_db = -15.0;
    }

    /// Processes interleaved float samples `[L0, R0, L1, R1, ...]` through AI denoising
    /// AND applies the Balanced Neural Guard gate smooth envelope cleanly per sample.
    pub fn process_interleaved(&mut self, samples: &mut [f32], channels_count: usize) {
        if !self.enabled || samples.is_empty() {
            return;
        }

        let ch_cnt = channels_count.min(self.channels_count).max(1);
        let total_frames = samples.len() / ch_cnt;
        if total_frames == 0 {
            return;
        }

        self.ensure_loader_started();
        self.poll_loaded_model();
        self.sync_model_strength();

        // Detect if stereo buffer is actually mono/dual-mono (common for USB mics & WASAPI bridge)
        let mut max_lr_diff = 0.0f32;
        if ch_cnt >= 2 {
            for f in 0..total_frames {
                let d = (samples[f * ch_cnt] - samples[f * ch_cnt + 1]).abs();
                if d > max_lr_diff {
                    max_lr_diff = d;
                }
            }
        }

        let frame_is_true_stereo = ch_cnt >= 2 && max_lr_diff > 1e-4;
        if frame_is_true_stereo {
            self.true_stereo_streak = (self.true_stereo_streak + 1).min(100);
        } else {
            self.true_stereo_streak = self.true_stereo_streak.saturating_sub(1);
        }
        let use_dual_channel = self.true_stereo_streak >= 8 && ch_cnt >= 2;

        // Push capture samples into analysis FIFOs
        for f in 0..total_frames {
            let l = samples[f * ch_cnt];
            let r = if ch_cnt >= 2 { samples[f * ch_cnt + 1] } else { l };
            if use_dual_channel {
                self.in_fifo_l.push(l);
                self.in_fifo_r.push(r);
            } else {
                // Average L+R cleanly if near-mono or duplicated
                self.in_fifo_l.push(if ch_cnt >= 2 { 0.5 * (l + r) } else { l });
                self.in_fifo_r.push(if ch_cnt >= 2 { 0.5 * (l + r) } else { l });
            }
        }

        let strength = self.strength.clamp(0.0, 1.0);

        while self.in_fifo_l.len() >= AI_FRAME_SIZE {
            let (lsnr_db, vad_prob, is_fallback_mode) = if let Some(ref mut df) = self.df_primary {
                self.active_engine = AI_ENGINE_DEEPFILTER3_LL;
                // Adaptive threshold: aggressive CPU saving during silence (-8.5 dB), zero mid-speech clipping while talking (-16.0 dB)
                df.min_db_thresh = if self.guard.is_gate_open() { -16.0 } else { -8.5 };
                if let Some(slice_in) = self.hop_in_arr.as_slice_mut() {
                    slice_in.copy_from_slice(&self.in_fifo_l[..AI_FRAME_SIZE]);
                }
                let lsnr = match df.process(self.hop_in_arr.view(), self.hop_out_arr.view_mut()) {
                    Ok(v) => v,
                    Err(_) => -15.0,
                };
                if let Some(slice_out) = self.hop_out_arr.as_slice() {
                    self.mono_out_hop.copy_from_slice(slice_out);
                }

                if use_dual_channel {
                    if self.df_secondary.is_none() {
                        if let Some(df2) = load_df_tract_for_strength(strength) {
                            self.df_secondary = Some(df2);
                        }
                    }
                    if let Some(ref mut df2) = self.df_secondary {
                        if let Some(slice_in) = self.hop_in_arr.as_slice_mut() {
                            slice_in.copy_from_slice(&self.in_fifo_r[..AI_FRAME_SIZE]);
                        }
                        let _ = df2.process(self.hop_in_arr.view(), self.hop_out_arr.view_mut());
                        if let Some(slice_out) = self.hop_out_arr.as_slice() {
                            self.stereo_out_hop_r.copy_from_slice(slice_out);
                        }
                    } else {
                        self.stereo_out_hop_r.copy_from_slice(&self.mono_out_hop);
                    }
                }
                let clean_vad = self.rn_primary.vad_only_prob(&self.mono_out_hop);
                (lsnr, clean_vad, false)
            } else {
                self.active_engine = AI_ENGINE_RNNOISE;
                let prob = self.rn_primary.process_hop(
                    &self.in_fifo_l[..AI_FRAME_SIZE],
                    &mut self.mono_out_hop,
                    strength,
                );
                if use_dual_channel {
                    self.rn_secondary.process_hop(
                        &self.in_fifo_r[..AI_FRAME_SIZE],
                        &mut self.stereo_out_hop_r,
                        strength,
                    );
                }
                let pseudo_lsnr = -12.0 + prob * 32.0;
                (pseudo_lsnr, prob, true)
            };

            let (prob, target_gain) = self.guard.evaluate_frame(
                &self.mono_out_hop,
                lsnr_db,
                vad_prob,
                is_fallback_mode,
                strength,
            );

            self.lsnr_db = lsnr_db;
            self.speech_prob = prob;
            self.current_target_gain = target_gain;

            for i in 0..AI_FRAME_SIZE {
                let g = self.guard.smooth_sample(target_gain);
                let out_l = self.mono_out_hop[i] * g;
                let out_r = if use_dual_channel {
                    self.stereo_out_hop_r[i] * g
                } else {
                    out_l
                };
                self.out_fifo_l.push_back(out_l);
                self.out_fifo_r.push_back(out_r);
            }

            self.in_fifo_l.drain(0..AI_FRAME_SIZE);
            self.in_fifo_r.drain(0..AI_FRAME_SIZE);
        }

        // Write processed frames back without injecting permanent idle FIFO zeros when host feeds >= 480 frames
        for f in 0..total_frames {
            let out_l = self.out_fifo_l.pop_front().unwrap_or(0.0);
            let out_r = self.out_fifo_r.pop_front().unwrap_or(out_l);

            samples[f * ch_cnt] = out_l;
            if ch_cnt >= 2 {
                samples[f * ch_cnt + 1] = out_r;
                for c in 2..ch_cnt {
                    samples[f * ch_cnt + c] = out_l;
                }
            }
        }
    }
}

