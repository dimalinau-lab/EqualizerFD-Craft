//! Channel Matrix and Arithmetic Expression Engine
//! Evaluates directives like `Copy: L=0.7*L+0.3*R R=0.7*R+0.3*L`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelTerm {
    pub coefficient: f64,
    pub source_channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRule {
    pub target_channel: String,
    pub terms: Vec<ChannelTerm>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ChannelMatrix {
    pub rules: Vec<ChannelRule>,
}

impl ChannelMatrix {
    pub fn parse(directive_body: &str) -> Self {
        let mut rules = Vec::new();

        // Tokens are assignments separated by space e.g. "L=0.7*L+0.3*R"
        for token in directive_body.split_whitespace() {
            if let Some((target, expr)) = token.split_once('=') {
                let target = target.trim().to_uppercase();
                let terms = parse_expression(expr);
                rules.push(ChannelRule {
                    target_channel: target,
                    terms,
                });
            }
        }

        Self { rules }
    }

    /// Applies the matrix routing to a set of input channel samples
    pub fn process_channels(&self, channel_values: &HashMap<String, f64>) -> HashMap<String, f64> {
        let mut output = channel_values.clone();

        for rule in &self.rules {
            let mut sum = 0.0;
            for term in &rule.terms {
                let val = channel_values.get(&term.source_channel).copied().unwrap_or(0.0);
                sum += term.coefficient * val;
            }
            output.insert(rule.target_channel.clone(), sum);
        }

        output
    }
}

fn parse_expression(expr: &str) -> Vec<ChannelTerm> {
    let mut terms = Vec::new();
    let clean = expr.trim().replace('-', "+-");

    for part in clean.split('+') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }

        if let Some((coeff_str, chan_str)) = p.split_once('*') {
            let coeff = coeff_str.trim().replace(',', ".").parse::<f64>().unwrap_or(1.0);
            let chan = chan_str.trim().to_uppercase();
            terms.push(ChannelTerm {
                coefficient: coeff,
                source_channel: chan,
            });
        } else {
            // No multiplication symbol, e.g. "L" or "-R"
            if p.starts_with('-') {
                terms.push(ChannelTerm {
                    coefficient: -1.0,
                    source_channel: p[1..].trim().to_uppercase(),
                });
            } else {
                terms.push(ChannelTerm {
                    coefficient: 1.0,
                    source_channel: p.to_uppercase(),
                });
            }
        }
    }

    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfeed_matrix() {
        let matrix = ChannelMatrix::parse("L=0.7*L+0.3*R R=0.3*L+0.7*R");
        assert_eq!(matrix.rules.len(), 2);

        let mut inputs = HashMap::new();
        inputs.insert("L".to_string(), 1.0);
        inputs.insert("R".to_string(), 0.0);

        let outputs = matrix.process_channels(&inputs);
        assert!((outputs["L"] - 0.7).abs() < 1e-6);
        assert!((outputs["R"] - 0.3).abs() < 1e-6);
    }
}
