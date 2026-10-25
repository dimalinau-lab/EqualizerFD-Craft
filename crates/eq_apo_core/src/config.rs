//! Equalizer FD CRAFT Configuration File Parser
//! Parses declarative directives from config.txt into an executable DSP AST.

use crate::biquad::{BiquadParams, FilterType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelId {
    All,
    Left,             // L
    Right,            // R
    Center,           // C
    Subwoofer,        // SUB / LFE
    RearLeft,         // RL
    RearRight,        // RR
    SideLeft,         // SL
    SideRight,        // SR
    Custom(String),   // e.g. L2, R2
}

impl ChannelId {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "ALL" => ChannelId::All,
            "L" => ChannelId::Left,
            "R" => ChannelId::Right,
            "C" => ChannelId::Center,
            "SUB" | "LFE" => ChannelId::Subwoofer,
            "RL" => ChannelId::RearLeft,
            "RR" => ChannelId::RearRight,
            "SL" => ChannelId::SideLeft,
            "SR" => ChannelId::SideRight,
            other => ChannelId::Custom(other.to_string()),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigDirective {
    Preamp {
        gain_db: f64,
        channels: Vec<ChannelId>,
    },
    Filter {
        is_enabled: bool,
        params: BiquadParams,
        channels: Vec<ChannelId>,
        index: Option<usize>,
    },
    Delay {
        delay_ms: f64,
        channels: Vec<ChannelId>,
    },
    Copy {
        assignments: Vec<(String, String)>,
    },
    Include {
        path: String,
    },
    Device {
        device_match: String,
    },
    GraphicEQ {
        points: Vec<(f64, f64)>, // (freq_hz, gain_db)
        channels: Vec<ChannelId>,
    },
    VSTPlugin {
        library: String,
        channels: Vec<ChannelId>,
        #[serde(default)]
        vst_id: Option<String>,
        #[serde(default)]
        params_json: Option<String>,
        #[serde(default = "default_true")]
        is_enabled: bool,
    },
    BossRc505 {
        effect_name: String,
        channels: Vec<ChannelId>,
    },
    Comment(String),
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub directives: Vec<ConfigDirective>,
}

impl ConfigFile {
    pub fn expand_includes(content: &str, depth: usize) -> String {
        if depth > 8 {
            return content.to_string();
        }
        let mut expanded = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.to_uppercase().starts_with("INCLUDE:") {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() > 1 {
                    let sub_path = parts[1].trim().trim_matches('"');
                    let candidate_paths = [
                        std::path::PathBuf::from(sub_path),
                        std::path::Path::new(r"C:\Program Files\EqualizerAPO\config").join(sub_path),
                        std::path::Path::new(r"d:\testik").join(sub_path),
                    ];
                    let mut found_content = None;
                    for p in &candidate_paths {
                        if let Ok(c) = std::fs::read_to_string(p) {
                            found_content = Some(c);
                            break;
                        }
                    }
                    if let Some(sub_content) = found_content {
                        expanded.push_str(&format!("# [Expanded Include: {}]\n", sub_path));
                        expanded.push_str(&Self::expand_includes(&sub_content, depth + 1));
                        expanded.push('\n');
                        continue;
                    }
                }
            }
            expanded.push_str(line);
            expanded.push('\n');
        }
        expanded
    }

    pub fn parse(content: &str, default_sample_rate: f64) -> Result<Self, String> {
        let expanded = Self::expand_includes(content, 0);
        let mut directives = Vec::new();
        let mut current_channels = vec![ChannelId::All];
        let mut pending_vst_meta: Option<(String, String)> = None;

        for (_line_num, raw_line) in expanded.lines().enumerate() {
            let mut line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            let mut is_explicit_disabled = false;
            if line.to_uppercase().starts_with("# [DISABLED]") {
                is_explicit_disabled = true;
                line = line[12..].trim();
            }

            // Comments
            if line.starts_with('#') || line.starts_with(';') {
                let comment_body = line.trim_start_matches(|c| c == '#' || c == ';').trim();
                // Check if comment contains VST metadata: [VSTPlugin: <id>] Params: <json>
                if let Some(rest) = comment_body.strip_prefix("[VSTPlugin:") {
                    if let Some((id_part, params_part)) = rest.split_once(']') {
                        let vst_id = id_part.trim().to_string();
                        let params_json = if let Some(p) = params_part.split_once("Params:") {
                            p.1.trim().to_string()
                        } else {
                            String::new()
                        };
                        pending_vst_meta = Some((vst_id, params_json));
                    }
                }
                directives.push(ConfigDirective::Comment(raw_line.to_string()));
                continue;
            }

            // Split directive name from body by colon
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.is_empty() {
                continue;
            }

            let cmd_name = parts[0].trim().to_uppercase();
            let body = if parts.len() > 1 { parts[1].trim() } else { "" };

            if cmd_name == "CHANNEL" {
                current_channels = body
                    .split_whitespace()
                    .map(ChannelId::parse)
                    .collect();
                if current_channels.is_empty() {
                    current_channels = vec![ChannelId::All];
                }
            } else if cmd_name == "PREAMP" {
                let gain = parse_db(body).unwrap_or(0.0);
                directives.push(ConfigDirective::Preamp {
                    gain_db: gain,
                    channels: current_channels.clone(),
                });
            } else if cmd_name.starts_with("FILTER") {
                // e.g. "Filter 1" or "Filter"
                let idx = cmd_name
                    .strip_prefix("FILTER")
                    .and_then(|s| s.trim().parse::<usize>().ok());

                if let Some(mut directive) = parse_filter_line(body, idx, default_sample_rate, current_channels.clone()) {
                    if is_explicit_disabled {
                        if let ConfigDirective::Filter { ref mut is_enabled, .. } = directive {
                            *is_enabled = false;
                        }
                    }
                    directives.push(directive);
                }
            } else if cmd_name == "DELAY" {
                let ms = parse_ms(body).unwrap_or(0.0);
                directives.push(ConfigDirective::Delay {
                    delay_ms: ms,
                    channels: current_channels.clone(),
                });
            } else if cmd_name == "COPY" {
                let assignments = parse_copy_assignments(body);
                directives.push(ConfigDirective::Copy { assignments });
            } else if cmd_name == "INCLUDE" {
                directives.push(ConfigDirective::Include {
                    path: body.trim().trim_matches('"').to_string(),
                });
            } else if cmd_name == "DEVICE" {
                directives.push(ConfigDirective::Device {
                    device_match: body.trim().to_string(),
                });
            } else if cmd_name == "GRAPHICEQ" {
                let points = parse_graphic_eq(body);
                directives.push(ConfigDirective::GraphicEQ {
                    points,
                    channels: current_channels.clone(),
                });
            } else if cmd_name == "VSTPLUGIN" {
                let trimmed = body.trim();
                let clean_lib = if let Some(stripped) = trimmed.strip_prefix("Library") {
                    stripped.trim().trim_matches('"').to_string()
                } else if let Some(stripped) = trimmed.strip_prefix("library") {
                    stripped.trim().trim_matches('"').to_string()
                } else {
                    trimmed.trim_matches('"').to_string()
                };
                let (vst_id, params_json) = pending_vst_meta.take().unzip();
                directives.push(ConfigDirective::VSTPlugin {
                    library: clean_lib,
                    channels: current_channels.clone(),
                    vst_id,
                    params_json,
                    is_enabled: !is_explicit_disabled,
                });
            } else if cmd_name == "BOSSRC505" || cmd_name == "RC505" {
                directives.push(ConfigDirective::BossRc505 {
                    effect_name: body.trim().to_string(),
                    channels: current_channels.clone(),
                });
            }
        }

        Ok(ConfigFile { directives })
    }
}

fn parse_db(s: &str) -> Option<f64> {
    let clean = s.to_lowercase().replace("db", "").replace(',', ".");
    clean.trim().parse::<f64>().ok()
}

fn parse_ms(s: &str) -> Option<f64> {
    let clean = s.to_lowercase().replace("ms", "").replace(',', ".");
    clean.trim().parse::<f64>().ok()
}

fn parse_copy_assignments(s: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    for token in s.split_whitespace() {
        if let Some((target, expr)) = token.split_once('=') {
            result.push((target.trim().to_string(), expr.trim().to_string()));
        }
    }
    result
}

fn parse_graphic_eq(s: &str) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for pair in s.split(';') {
        let parts: Vec<&str> = pair.split_whitespace().collect();
        if parts.len() >= 2 {
            if let (Ok(f), Ok(g)) = (
                parts[0].replace(',', ".").parse::<f64>(),
                parts[1].replace(',', ".").parse::<f64>(),
            ) {
                points.push((f, g));
            }
        }
    }
    points
}

fn parse_filter_line(
    body: &str,
    index: Option<usize>,
    sample_rate: f64,
    channels: Vec<ChannelId>,
) -> Option<ConfigDirective> {
    let tokens: Vec<&str> = body.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let is_enabled = !tokens[0].eq_ignore_ascii_case("OFF");
    let mut i = 1;
    if i >= tokens.len() {
        return None;
    }

    let type_str = tokens[i].to_uppercase();
    i += 1;

    let filter_type = match type_str.as_str() {
        "PK" => FilterType::Peaking,
        "LS" => FilterType::LowShelf,
        "HS" => FilterType::HighShelf,
        "LP" => FilterType::LowPass,
        "HP" => FilterType::HighPass,
        "BP" => FilterType::BandPass,
        "NO" => FilterType::Notch,
        "AP" => FilterType::AllPass,
        "NONE" => return None,
        _ => FilterType::Peaking,
    };

    let mut fc = 1000.0;
    let mut gain = 0.0;
    let mut q = 1.4142;

    while i < tokens.len() {
        let key = tokens[i].to_uppercase();
        if key == "FC" && i + 1 < tokens.len() {
            if let Ok(val) = tokens[i + 1].replace(',', ".").parse::<f64>() {
                fc = val;
            }
            i += 2;
            if i < tokens.len() && tokens[i].eq_ignore_ascii_case("HZ") {
                i += 1;
            }
        } else if key == "GAIN" && i + 1 < tokens.len() {
            if let Ok(val) = tokens[i + 1].replace(',', ".").parse::<f64>() {
                gain = val;
            }
            i += 2;
            if i < tokens.len() && tokens[i].eq_ignore_ascii_case("DB") {
                i += 1;
            }
        } else if key == "Q" && i + 1 < tokens.len() {
            if let Ok(val) = tokens[i + 1].replace(',', ".").parse::<f64>() {
                q = val;
            }
            i += 2;
        } else if key == "BW" && i + 2 < tokens.len() && tokens[i + 1].eq_ignore_ascii_case("OCT") {
            if let Ok(bw) = tokens[i + 2].replace(',', ".").parse::<f64>() {
                // Convert bandwidth in octaves to Q factor
                let term = 2.0_f64.powf(bw);
                q = term.sqrt() / (term - 1.0);
            }
            i += 3;
        } else {
            i += 1;
        }
    }

    Some(ConfigDirective::Filter {
        is_enabled,
        params: BiquadParams {
            filter_type,
            frequency_hz: fc,
            gain_db: gain,
            q_factor: q,
            sample_rate,
        },
        channels,
        index,
    })
}
