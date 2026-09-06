// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{io::Write, time::Duration};

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

pub async fn probe<W: Write>(
    output: &mut W,
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
) -> Result<()> {
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

    render_host(output, &host, plain)?;

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
                render_field(output, &config, property, intensity)?;
            }

            let prober = SshProber::new(Duration::from_secs(10));
            let hostname = config.hostname()?;
            let port = config.port()?;
            let result = prober.connect(hostname, port).await?;
            render_latency(output, "Latency", result.latency)?;
        }
        Err(err) => {
            writeln!(output, "Error: Could not parse information for host: {err}")?;
        }
    }
    Ok(())
}

fn render_host<W: Write>(output: &mut W, host: &str, plain: bool) -> Result<()> {
    let mut host = host.to_string();
    if !plain {
        host = host.green().bold().to_string();
    }
    writeln!(output, "📡 {host}")?;
    Ok(())
}

fn render_field<W: Write>(
    output: &mut W,
    config: &HostConfig,
    property: &str,
    intensity: LabelIntensity,
) -> Result<()> {
    let mut label = String::new();
    use std::fmt::Write;
    write!(
        &mut label,
        "{}:",
        prop_to_pretty_alias(property).unwrap_or(property)
    )?;

    match intensity {
        LabelIntensity::Normal => {
            write!(output, "{} ", label.blue())?;
        }
        LabelIntensity::Bright => {
            write!(output, "{} ", label.bright_cyan().bold())?;
        }
    }

    let value_not_found = ["???".to_owned()];
    let values = config.get(property).unwrap_or(&value_not_found);
    let mut plain_output = String::new();

    #[expect(unstable_name_collisions)]
    for v in values.iter().map(String::as_str).intersperse(",") {
        write!(&mut plain_output, "{v}")?;
    }
    match intensity {
        LabelIntensity::Normal => {
            writeln!(output, "{plain_output}")?;
        }
        LabelIntensity::Bright => {
            writeln!(output, "{}", plain_output.bold())?;
        }
    }
    Ok(())
}

fn render_latency<W: Write>(output: &mut W, property: &str, latency: Duration) -> Result<()> {
    const GOOD_THRESHOLD: u128 = 500;
    const WARN_THRESHOLD: u128 = 5000;

    let lat_ms = latency.as_millis();
    let lat_str = format!("{lat_ms:.2} ms");
    let lat_str = if lat_ms < GOOD_THRESHOLD {
        lat_str.green()
    } else if lat_ms < WARN_THRESHOLD {
        lat_str.yellow()
    } else {
        lat_str.red()
    };

    let property = format!("{property}:");
    writeln!(output, "{} {lat_str} ", property.blue())?;
    Ok(())
}
