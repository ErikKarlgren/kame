// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;

use crate::{
    cli::ProbeArgs,
    ssh::{host_config::HostConfig, host_properties::prop_to_pretty_alias},
};
use colored::{Colorize, control};

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
) -> String {
    if verbose {
        todo!("verbose not implemented")
    }
    if plain {
        todo!("plain not implemented")
    }
    if json {
        todo!("json not implemented")
    }
    if no_probes {
        todo!("no_probes not implemented")
    }

    if !plain {
        control::set_override(true); // force colors
    }

    let mut output = format!("{}\n", render_host(&host, plain));

    match HostConfig::from_host(&host, config.as_deref()).await {
        Ok(config) => {
            const SSH_FIELDS: [&'static str; 3] = ["hostname", "user", "port"];
            let props_to_show = SSH_FIELDS
                .into_iter()
                .chain(props_to_highlight.iter().flat_map(|&props| {
                    props
                        .iter()
                        .filter(|p| !SSH_FIELDS.contains(&p.as_str()))
                        .map(|p| p.as_str())
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
        }
        Err(err) => {
            _ = writeln!(
                &mut output,
                "Error: Could not parse information for host: {err}"
            );
        }
    }
    output
}

fn render_host(host: &str, plain: bool) -> String {
    let mut line = format!("모{host}");
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
    let value_not_found = ["???".to_owned()];
    let values = config.get(property).unwrap_or(&value_not_found);
    let label = prop_to_pretty_alias(property).unwrap_or(property);
    let values: String = if values.len() == 1 {
        values.first().unwrap().clone()
    } else {
        let markdown_list: String = values.iter().map(|v| format!(" {v}\n")).collect();
        format!("\n{markdown_list}")
    };
    let colored_label = match intensity {
        LabelIntensity::Normal => label.blue(),
        LabelIntensity::Bright => label.bright_cyan().bold(),
    };
    _ = writeln!(output, "{colored_label} {values}");
}
