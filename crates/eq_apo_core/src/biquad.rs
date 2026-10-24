//! Biquad IIR Filter Implementation
//! Clean-room implementation based on Robert Bristow-Johnson's Audio EQ Cookbook formulas.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterType {
    Peaking,   // PK
    LowShelf,  // LS
    HighShelf, // HS
    LowPass,   // LP
    HighPass,  // HP
    BandPass,  // BP
    Notch,     // NO
    AllPass,   // AP
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiquadParams {
    pub filter_type: FilterType,
    pub frequency_hz: f64,
    pub gain_db: f64,
    pub q_factor: f64,
    pub sample_rate: f64,
}

#[derive(Debug, Clone)]
pub struct BiquadCoefficients {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl BiquadCoefficients {
    pub fn compute(params: &BiquadParams) -> Self {
        let sr = params.sample_rate.max(1.0);
        let fc = params.frequency_hz.clamp(1.0, (sr / 2.0) - 1.0);
        let q = params.q_factor.max(0.001);
        let gain_db = params.gain_db;

        let w0 = 2.0 * PI * fc / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let a_linear = 10.0f64.powf(gain_db / 40.0); // sqrt(10^(gain/20))

        let (b0, b1, b2, a0, a1, a2) = match params.filter_type {
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a_linear;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a_linear;
                let a0 = 1.0 + alpha / a_linear;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a_linear;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a_linear.sqrt() * alpha;
                let b0 = a_linear * ((a_linear + 1.0) - (a_linear - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = 2.0 * a_linear * ((a_linear - 1.0) - (a_linear + 1.0) * cos_w0);
                let b2 = a_linear * ((a_linear + 1.0) - (a_linear - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a_linear + 1.0) + (a_linear - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a_linear - 1.0) + (a_linear + 1.0) * cos_w0);
                let a2 = (a_linear + 1.0) + (a_linear - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a_linear.sqrt() * alpha;
                let b0 = a_linear * ((a_linear + 1.0) + (a_linear - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = -2.0 * a_linear * ((a_linear - 1.0) + (a_linear + 1.0) * cos_w0);
                let b2 = a_linear * ((a_linear + 1.0) + (a_linear - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a_linear + 1.0) - (a_linear - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a_linear - 1.0) - (a_linear + 1.0) * cos_w0);
                let a2 = (a_linear + 1.0) - (a_linear - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::AllPass => {
                let b0 = 1.0 - alpha;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 + alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Normalize coefficients by a0
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Evaluates the complex transfer function H(e^(j*w)) at a given frequency in Hz
    /// Returns (magnitude_dB, phase_radians)
    pub fn frequency_response(&self, freq_hz: f64, sample_rate: f64) -> (f64, f64) {
        let w = 2.0 * PI * freq_hz / sample_rate;
        let cos_w = w.cos();
        let sin_w = w.sin();
        let cos_2w = (2.0 * w).cos();
        let sin_2w = (2.0 * w).sin();

        // Numerator: B(e^(jw)) = b0 + b1*e^(-jw) + b2*e^(-j2w)
        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -self.b1 * sin_w - self.b2 * sin_2w;

        // Denominator: A(e^(jw)) = 1 + a1*e^(-jw) + a2*e^(-j2w)
        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -self.a1 * sin_w - self.a2 * sin_2w;

        let num_mag_sq = num_re * num_re + num_im * num_im;
        let den_mag_sq = den_re * den_re + den_im * den_im;

        let mag_linear = (num_mag_sq / den_mag_sq.max(1e-15)).sqrt();
        let mag_db = 20.0 * mag_linear.max(1e-7).log10();

        let num_phase = num_im.atan2(num_re);
        let den_phase = den_im.atan2(den_re);
        let phase = num_phase - den_phase;

        (mag_db, phase)
    }
}

/// Real-time Biquad Filter with Direct Form II Transposed state
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    params: BiquadParams,
    coeffs: BiquadCoefficients,
    s1: f64,
    s2: f64,
    pub is_enabled: bool,
}

impl BiquadFilter {
    pub fn new(params: BiquadParams) -> Self {
        let coeffs = BiquadCoefficients::compute(&params);
        Self {
            params,
            coeffs,
            s1: 0.0,
            s2: 0.0,
            is_enabled: true,
        }
    }

    pub fn update_params(&mut self, params: BiquadParams) {
        self.params = params;
        self.coeffs = BiquadCoefficients::compute(&self.params);
    }

    pub fn reset_state(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    #[inline(always)]
    pub fn process_sample(&mut self, input: f64) -> f64 {
        if !self.is_enabled {
            return input;
        }
        // Direct Form II Transposed
        let output = self.coeffs.b0 * input + self.s1;
        self.s1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.s2;
        self.s2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
        output
    }

    pub fn frequency_response(&self, freq_hz: f64, sample_rate: f64) -> (f64, f64) {
        if !self.is_enabled {
            (0.0, 0.0)
        } else {
            self.coeffs.frequency_response(freq_hz, sample_rate)
        }
    }
}
