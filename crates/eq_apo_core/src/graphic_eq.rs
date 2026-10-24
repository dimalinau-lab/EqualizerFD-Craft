//! Graphic Equalizer Engine with Linear and Monotone Spline Interpolation
//! Supports arbitrary frequency-gain pairs and standard ISO 15/31 band layouts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicEqPoint {
    pub freq_hz: f64,
    pub gain_db: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicEQ {
    pub points: Vec<GraphicEqPoint>,
    pub is_enabled: bool,
}

impl GraphicEQ {
    pub fn new(mut points: Vec<GraphicEqPoint>) -> Self {
        points.sort_by(|a, b| a.freq_hz.partial_cmp(&b.freq_hz).unwrap());
        Self {
            points,
            is_enabled: true,
        }
    }

    /// Evaluates gain in dB at an arbitrary frequency using monotonic cubic Hermite interpolation
    pub fn evaluate_gain_at(&self, freq_hz: f64) -> f64 {
        if !self.is_enabled || self.points.is_empty() {
            return 0.0;
        }

        if self.points.len() == 1 {
            return self.points[0].gain_db;
        }

        // Clamp to endpoints
        if freq_hz <= self.points[0].freq_hz {
            return self.points[0].gain_db;
        }
        let last_idx = self.points.len() - 1;
        if freq_hz >= self.points[last_idx].freq_hz {
            return self.points[last_idx].gain_db;
        }

        // Work in log10 frequency space for smooth acoustic octave scaling
        let log_f = freq_hz.log10();

        // Find surrounding interval [i, i+1]
        let mut idx = 0;
        for i in 0..last_idx {
            if freq_hz >= self.points[i].freq_hz && freq_hz <= self.points[i + 1].freq_hz {
                idx = i;
                break;
            }
        }

        let p0 = &self.points[idx];
        let p1 = &self.points[idx + 1];

        let x0 = p0.freq_hz.log10();
        let x1 = p1.freq_hz.log10();
        let y0 = p0.gain_db;
        let y1 = p1.gain_db;

        let dx = x1 - x0;
        if dx.abs() < 1e-9 {
            return y0;
        }

        let t = (log_f - x0) / dx;
        // Smooth Hermite blend (smoothstep s-curve)
        let h00 = (1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t);
        let h01 = t * t * (3.0 - 2.0 * t);

        h00 * y0 + h01 * y1
    }

    /// Returns standard ISO 31-band (1/3 octave) frequency grid
    pub fn standard_iso_31_frequencies() -> &'static [f64] {
        &[
            20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0,
            400.0, 500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0,
            5000.0, 6300.0, 8000.0, 10000.0, 12500.0, 16000.0, 20000.0,
        ]
    }
}
