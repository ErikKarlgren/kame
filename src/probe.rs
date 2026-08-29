// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt::Write;

use crate::{cli::ProbeArgs, ssh::host_config::HostConfig};
use colored::Colorize;

pub async fn probe(
    ProbeArgs {
        host,
        verbose,
        plain,
        json,
        no_probes,
    }: ProbeArgs,
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

    let mut output = format!("{}\n", render_host(&host, plain));

    let config = HostConfig::parse(&host).await;
    match config {
        Ok(config) => {
            const SSH_FIELDS: [(&str, &str); 3] =
                [("Hostname", "hostname"), ("User", "user"), ("Port", "port")];
            for (pretty_alias, setting) in SSH_FIELDS {
                render_field(&mut output, &config, pretty_alias, setting);
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

fn render_field(output: &mut String, config: &HostConfig, pretty_alias: &str, setting: &str) {
    let value_not_found = ["???".to_owned()];
    let values = config.get(setting).unwrap_or(&value_not_found);
    let values: String = if values.len() == 1 {
        values.first().unwrap().clone()
    } else {
        format!("{values:?}")
    };
    _ = writeln!(output, "{} {values}", pretty_alias.yellow());
}
