pub mod biquad;
pub mod config;
pub mod graphic_eq;
pub mod matrix;
pub mod pipeline;

pub use biquad::{BiquadCoefficients, BiquadFilter, BiquadParams, FilterType};
pub use config::{ChannelId, ConfigDirective, ConfigFile};
pub use graphic_eq::{GraphicEQ, GraphicEqPoint};
pub use matrix::{ChannelMatrix, ChannelRule, ChannelTerm};
pub use pipeline::{AudioPipeline, ChannelProcessingChain, FrequencyResponsePoint};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peaking_filter_response() {
        let sample_rate = 48000.0;
        let fc = 1000.0;
        let gain_db = 6.0;
        let q = 1.4142;

        let filter = BiquadFilter::new(BiquadParams {
            filter_type: FilterType::Peaking,
            frequency_hz: fc,
            gain_db,
            q_factor: q,
            sample_rate,
        });

        // At center frequency, magnitude response should match gain_db
        let (mag_at_fc, _) = filter.frequency_response(fc, sample_rate);
        assert!((mag_at_fc - gain_db).abs() < 0.05, "Expected ~6 dB, got {}", mag_at_fc);

        // Far away from center frequency, magnitude should approach 0 dB
        let (mag_far, _) = filter.frequency_response(50.0, sample_rate);
        assert!(mag_far.abs() < 0.2, "Expected ~0 dB at 50Hz, got {}", mag_far);
    }

    #[test]
    fn test_graphic_eq_interpolation() {
        let points = vec![
            GraphicEqPoint { freq_hz: 100.0, gain_db: 0.0 },
            GraphicEqPoint { freq_hz: 1000.0, gain_db: 6.0 },
            GraphicEqPoint { freq_hz: 10000.0, gain_db: 0.0 },
        ];
        let eq = GraphicEQ::new(points);

        let g_at_1k = eq.evaluate_gain_at(1000.0);
        assert!((g_at_1k - 6.0).abs() < 0.01);

        let g_between = eq.evaluate_gain_at(316.0); // approx geometric mean between 100 and 1000
        assert!(g_between > 1.0 && g_between < 5.5, "Expected smooth interpolation, got {}", g_between);
    }

    #[test]
    fn test_config_parsing_and_pipeline() {
        let config_text = r#"
            # Master studio EQ
            Preamp: -3.0 dB
            Channel: all
            Filter 1: ON PK Fc 100 Hz Gain 4.0 dB Q 1.0
            Filter 2: ON HS Fc 10000 Hz Gain 2.0 dB Q 0.7071
            Filter 3: OFF PK Fc 1000 Hz Gain -6.0 dB Q 2.0
            GraphicEQ: 50 -2.0; 200 3.0; 1000 0.0
            Copy: L=0.8*L+0.2*R R=0.2*L+0.8*R
        "#;

        let config = ConfigFile::parse(config_text, 48000.0).expect("Failed to parse config");
        assert_eq!(config.directives.len(), 7);

        let pipeline = AudioPipeline::from_config(&config, 48000.0);
        assert!(pipeline.matrix.is_some());
        let curve = pipeline.generate_response_curve(100, 20.0, 20000.0);
        assert_eq!(curve.len(), 100);
    }

    #[test]
    fn test_rnnoise_parsing() {
        let text = r#"
            # RNNoise test
            RNNoise: ON 85%
            # [Disabled] RNNoise: ON 100%
            AI_NOISE: OFF 50%
        "#;
        let config = ConfigFile::parse(text, 48000.0).expect("Failed to parse RNNoise config");
        assert_eq!(config.directives.len(), 4);
        match &config.directives[1] {
            ConfigDirective::RNNoise { is_enabled, strength, .. } => {
                assert_eq!(*is_enabled, true);
                assert!((*strength - 0.85).abs() < 1e-4);
            }
            _ => panic!("Expected RNNoise directive"),
        }
        match &config.directives[2] {
            ConfigDirective::RNNoise { is_enabled, strength, .. } => {
                assert_eq!(*is_enabled, false);
                assert!((*strength - 1.0).abs() < 1e-4);
            }
            _ => panic!("Expected disabled RNNoise directive"),
        }
        match &config.directives[3] {
            ConfigDirective::RNNoise { is_enabled, strength, .. } => {
                assert_eq!(*is_enabled, false);
                assert!((*strength - 0.5).abs() < 1e-4);
            }
            _ => panic!("Expected AI_NOISE directive"),
        }
    }
}
