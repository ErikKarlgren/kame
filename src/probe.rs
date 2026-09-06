// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;

use crate::{
    cli::ProbeArgs,
    ssh::{host_config::HostConfig, host_properties::prop_to_pretty_alias},
};
use anyhow::{Result, bail};
use colored::{Colorize, control};
use itertools::Itertools;

#[derive(Copy, Clone, Debug)]
enum LabelIntensity {
    Normal,
    Bright,
}

pub async fn probe(
    ProbeArgs {
        host,
        verbose,
        plain,
        json,
        no_probes,
        config,
    }: ProbeArgs,
    // 99% of the time <3 elements, so no need for a HashSet
    props_to_highlight: Option<&[String]>,
) -> Result<String> {
    if verbose {
        bail!("--verbose not implemented")
    }
    if plain {
        bail!("--plain not implemented")
    }
    if json {
        bail!("--json not implemented")
    }
    if no_probes {
        bail!("--no-probes not implemented")
    }

    if !plain {
        control::set_override(true); // force colors
    }

    let mut output = format!("{}\n", render_host(&host, plain));

    match HostConfig::from_host(&host, config.as_deref()).await {
        Ok(config) => {
            const SSH_FIELDS: [&str; 3] = ["hostname", "user", "port"];
            let props_to_show = SSH_FIELDS
                .into_iter()
                .chain(props_to_highlight.iter().flat_map(|&props| {
                    props
                        .iter()
                        .filter(|p| !SSH_FIELDS.contains(&p.as_str()))
                        .map(std::string::String::as_str)
                }));
            for property in props_to_show {
                let intensity = if let Some(props) = props_to_highlight
                    && props.iter().any(|p| p == property)
                {
                    LabelIntensity::Bright
                } else {
                    LabelIntensity::Normal
                };
                render_field(&mut output, &config, property, intensity);
            }
            render_latency(&mut output, "Latency", 120.0);
        }
        Err(err) => {
            _ = writeln!(
                &mut output,
                "Error: Could not parse information for host: {err}"
            );
        }
    }
    Ok(output)
}

fn render_host(host: &str, plain: bool) -> String {
    let mut line = format!("📡 {host}");
    if !plain {
        line = line.green().bold().to_string();
    }
    line
}

fn render_field(
    output: &mut String,
    config: &HostConfig,
    property: &str,
    intensity: LabelIntensity,
) {
    let mut label = String::new();
    _ = write!(
        &mut label,
        "{}:",
        prop_to_pretty_alias(property).unwrap_or(property)
    );

    match intensity {
        LabelIntensity::Normal => {
            _ = write!(output, "{} ", label.blue());
        }
        LabelIntensity::Bright => {
            _ = write!(output, "{} ", label.bright_cyan().bold());
        }
    }

    let value_not_found = ["???".to_owned()];
    let values = config.get(property).unwrap_or(&value_not_found);
    let mut plain_output = String::new();

    #[expect(unstable_name_collisions)]
    for v in values.iter().map(String::as_str).intersperse(",") {
        _ = write!(&mut plain_output, "{v}");
    }
    match intensity {
        LabelIntensity::Normal => {
            _ = writeln!(output, "{plain_output}");
        }
        LabelIntensity::Bright => {
            _ = writeln!(output, "{}", plain_output.bold());
        }
    }
}

fn render_latency(output: &mut String, property: &str, time_ms: f32) {
    const GOOD_THRESHOLD: f32 = 500.0;
    const WARN_THRESHOLD: f32 = 5000.0;

    let value_str = format!("{time_ms:.2} ms");
    let time_ms = if time_ms < GOOD_THRESHOLD {
        value_str.green()
    } else if time_ms < WARN_THRESHOLD {
        value_str.yellow()
    } else {
        value_str.red()
    };

    let property = format!("{property}:");
    _ = writeln!(output, "{} {time_ms} ", property.blue());
}
