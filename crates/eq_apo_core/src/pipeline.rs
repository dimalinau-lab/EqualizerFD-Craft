//! Audio Processing Pipeline
//! Connects channels, preamp stages, biquad filter chains, graphic EQ, and routing matrix.

use crate::biquad::BiquadFilter;
use crate::config::{ChannelId, ConfigDirective, ConfigFile};
use crate::graphic_eq::{GraphicEQ, GraphicEqPoint};
use crate::matrix::ChannelMatrix;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencyResponsePoint {
    pub freq_hz: f64,
    pub magnitude_db: f64,
    pub phase_deg: f64,
}

#[derive(Debug, Clone)]
pub struct ChannelProcessingChain {
    pub channel: ChannelId,
    pub preamp_gain_db: f64,
    pub filters: Vec<BiquadFilter>,
    pub graphic_eqs: Vec<GraphicEQ>,
    pub delay_samples: usize,
    delay_buffer: Vec<f64>,
    delay_write_pos: usize,
}

impl ChannelProcessingChain {
    pub fn new(channel: ChannelId) -> Self {
        Self {
            channel,
            preamp_gain_db: 0.0,
            filters: Vec::new(),
            graphic_eqs: Vec::new(),
            delay_samples: 0,
            delay_buffer: vec![0.0; 48000 * 2], // up to 2 seconds delay
            delay_write_pos: 0,
        }
    }

    pub fn set_preamp(&mut self, gain_db: f64) {
        self.preamp_gain_db = gain_db;
    }

    pub fn add_filter(&mut self, filter: BiquadFilter) {
        self.filters.push(filter);
    }

    pub fn add_graphic_eq(&mut self, eq: GraphicEQ) {
        self.graphic_eqs.push(eq);
    }

    pub fn set_delay_ms(&mut self, delay_ms: f64, sample_rate: f64) {
        let samples = ((delay_ms / 1000.0) * sample_rate).round() as usize;
        self.delay_samples = samples.min(self.delay_buffer.len() - 1);
    }

    #[inline]
    pub fn process_sample(&mut self, input: f64) -> f64 {
        // 1. Preamp
        let preamp_linear = 10.0f64.powf(self.preamp_gain_db / 20.0);
        let mut sample = input * preamp_linear;

        // 2. Biquad filter cascade
        for filter in &mut self.filters {
            sample = filter.process_sample(sample);
        }

        // 3. Optional delay line
        if self.delay_samples > 0 {
            let buffer_len = self.delay_buffer.len();
            self.delay_buffer[self.delay_write_pos] = sample;
            let read_pos = (self.delay_write_pos + buffer_len - self.delay_samples) % buffer_len;
            sample = self.delay_buffer[read_pos];
            self.delay_write_pos = (self.delay_write_pos + 1) % buffer_len;
        }

        sample
    }

    /// Computes cumulative frequency response at a specific frequency
    pub fn frequency_response(&self, freq_hz: f64, sample_rate: f64) -> (f64, f64) {
        let mut total_mag_db = self.preamp_gain_db;
        let mut total_phase_rad = 0.0;

        for filter in &self.filters {
            let (m, p) = filter.frequency_response(freq_hz, sample_rate);
            total_mag_db += m;
            total_phase_rad += p;
        }

        for eq in &self.graphic_eqs {
            total_mag_db += eq.evaluate_gain_at(freq_hz);
        }

        (total_mag_db, total_phase_rad)
    }
}

pub struct AudioPipeline {
    pub sample_rate: f64,
    pub left_chain: ChannelProcessingChain,
    pub right_chain: ChannelProcessingChain,
    pub matrix: Option<ChannelMatrix>,
}

impl AudioPipeline {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            left_chain: ChannelProcessingChain::new(ChannelId::Left),
            right_chain: ChannelProcessingChain::new(ChannelId::Right),
            matrix: None,
        }
    }

    pub fn from_config(config: &ConfigFile, sample_rate: f64) -> Self {
        Self::from_config_internal(config, sample_rate, None)
    }

    pub fn from_config_with_includes(config: &ConfigFile, sample_rate: f64, base_dir: &Path) -> Self {
        Self::from_config_internal(config, sample_rate, Some(base_dir))
    }

    fn from_config_internal(config: &ConfigFile, sample_rate: f64, base_dir: Option<&Path>) -> Self {
        let mut pipeline = Self::new(sample_rate);

        for directive in &config.directives {
            match directive {
                ConfigDirective::Preamp { gain_db, channels } => {
                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Left) {
                        pipeline.left_chain.set_preamp(*gain_db);
                    }
                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Right) {
                        pipeline.right_chain.set_preamp(*gain_db);
                    }
                }
                ConfigDirective::Filter { is_enabled, params, channels, .. } => {
                    let mut p = params.clone();
                    p.sample_rate = sample_rate;
                    let mut filter = BiquadFilter::new(p);
                    filter.is_enabled = *is_enabled;

                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Left) {
                        pipeline.left_chain.add_filter(filter.clone());
                    }
                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Right) {
                        pipeline.right_chain.add_filter(filter);
                    }
                }
                ConfigDirective::GraphicEQ { points, channels } => {
                    let pts: Vec<GraphicEqPoint> = points
                        .iter()
                        .map(|(f, g)| GraphicEqPoint { freq_hz: *f, gain_db: *g })
                        .collect();
                    let eq = GraphicEQ::new(pts);

                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Left) {
                        pipeline.left_chain.add_graphic_eq(eq.clone());
                    }
                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Right) {
                        pipeline.right_chain.add_graphic_eq(eq);
                    }
                }
                ConfigDirective::Delay { delay_ms, channels } => {
                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Left) {
                        pipeline.left_chain.set_delay_ms(*delay_ms, sample_rate);
                    }
                    if channels.contains(&ChannelId::All) || channels.contains(&ChannelId::Right) {
                        pipeline.right_chain.set_delay_ms(*delay_ms, sample_rate);
                    }
                }
                ConfigDirective::Copy { assignments } => {
                    let mut expr_str = String::new();
                    for (t, e) in assignments {
                        expr_str.push_str(&format!("{}={} ", t, e));
                    }
                    pipeline.matrix = Some(ChannelMatrix::parse(&expr_str));
                }
                ConfigDirective::Include { path } => {
                    if let Some(base) = base_dir {
                        let full_path = base.join(path);
                        if let Ok(content) = fs::read_to_string(&full_path) {
                            if let Ok(sub_config) = ConfigFile::parse(&content, sample_rate) {
                                let sub_pipe = Self::from_config_internal(&sub_config, sample_rate, Some(base));
                                // Append filters from included sub-config
                                for f in sub_pipe.left_chain.filters {
                                    pipeline.left_chain.add_filter(f);
                                }
                                for f in sub_pipe.right_chain.filters {
                                    pipeline.right_chain.add_filter(f);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        pipeline
    }

    /// Process a stereo interleaved buffer in-place
    pub fn process_interleaved_stereo(&mut self, buffer: &mut [f64]) {
        for chunk in buffer.chunks_exact_mut(2) {
            let left_in = chunk[0];
            let right_in = chunk[1];

            let left_out = self.left_chain.process_sample(left_in);
            let right_out = self.right_chain.process_sample(right_in);

            if let Some(matrix) = &self.matrix {
                let mut map = std::collections::HashMap::new();
                map.insert("L".to_string(), left_out);
                map.insert("R".to_string(), right_out);
                let processed_map = matrix.process_channels(&map);
                chunk[0] = processed_map.get("L").copied().unwrap_or(left_out);
                chunk[1] = processed_map.get("R").copied().unwrap_or(right_out);
            } else {
                chunk[0] = left_out;
                chunk[1] = right_out;
            }
        }
    }

    /// Generates logarithmic frequency response points (e.g. 500 points from 10Hz to 20kHz)
    pub fn generate_response_curve(
        &self,
        points_count: usize,
        min_freq: f64,
        max_freq: f64,
    ) -> Vec<FrequencyResponsePoint> {
        let mut curve = Vec::with_capacity(points_count);
        let log_min = min_freq.log10();
        let log_max = max_freq.log10();
        let step = (log_max - log_min) / (points_count.max(2) - 1) as f64;

        for i in 0..points_count {
            let freq = 10.0f64.powf(log_min + (i as f64) * step);
            let (mag_db, phase_rad) = self.left_chain.frequency_response(freq, self.sample_rate);
            curve.push(FrequencyResponsePoint {
                freq_hz: (freq * 10.0).round() / 10.0,
                magnitude_db: (mag_db * 100.0).round() / 100.0,
                phase_deg: ((phase_rad * 180.0 / std::f64::consts::PI) * 10.0).round() / 10.0,
            });
        }

        curve
    }
}
