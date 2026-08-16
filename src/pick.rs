// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use anyhow::{Result, anyhow};
use skim::{
    Skim,
    prelude::{SkimItem, SkimOptionsBuilder},
    tui::PreviewCallback,
};
use tokio::runtime::Handle;

use crate::{
    cli::PickArgs,
    ssh::{host_config::HostConfig, host_finder::parse_hosts},
};

/// Choose a host
pub async fn pick(
    PickArgs {
        query,
        fields,
        multi,
        config,
        literal,
        json,
        preview_cmd,
    }: PickArgs,
) -> Result<()> {
    if literal {
        todo!("--literal not implemented yet");
    }
    if json {
        todo!("--json not implemented yet");
    }
    if preview_cmd.is_some() {
        todo!("--preview-cmd not implemented yet");
    }

    let path = choose_config_path(config)?;
    let hosts = parse_hosts(path).await?;

    let options = SkimOptionsBuilder::default()
        .multi(multi)
        .query(query.unwrap_or(String::new()))
        .height("16")
        .preview_fn(PreviewCallback::from(default_preview))
        .build()
        .unwrap();
    let output = Skim::run_items(options, hosts).unwrap();
    for host in output.selected_items {
        let host_cfg = HostConfig::parse(host.output().as_ref()).await?;
        for field in &fields {
            for value in host_cfg.get(&field).unwrap_or(&["???".to_owned()]) {
                println!("{}", &value);
            }
        }
    }
    Ok(())
}

fn choose_config_path(
    config: Option<std::path::PathBuf>,
) -> Result<std::path::PathBuf, anyhow::Error> {
    if let Some(p) = config {
        Ok(p)
    } else {
        match dirs::home_dir() {
            Some(p) => Ok(p.join(".ssh/config")),
            None => Err(anyhow!(
                "No home directory found. Please use the flag --config and provide an ssh config file"
            )),
        }
    }
}

fn default_preview(hosts: Vec<Arc<dyn SkimItem>>) -> Vec<String> {
    let results = tokio::task::block_in_place(move || {
        Handle::current().block_on(async {
            let mut results = Vec::with_capacity(hosts.len());
            for host in hosts {
                results.push((
                    host.text().into_owned(),
                    HostConfig::parse(host.text().as_ref()).await,
                ))
            }
            results
        })
    });

    let mut lines = vec![];
    for (host, config) in results {
        lines.push(format!("모{host}"));
        let value_not_found = ["???".to_owned()];
        match config {
            Ok(config) => {
                for (pretty_alias, setting) in
                    [("Hostname", "hostname"), ("User", "user"), ("Port", "port")]
                {
                    let values = config.get(setting).unwrap_or(&value_not_found);
                    let output: String = if values.len() == 1 {
                        values.first().unwrap().clone()
                    } else {
                        format!("{:?}", values)
                    };
                    lines.push(format!("{pretty_alias}: {output}"));
                }
            }
            Err(err) => lines.push(format!(
                "Error: Could not parse information for host: {err}"
            )),
        }
    }
    lines
}
