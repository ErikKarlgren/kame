// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Result, anyhow};
use skim::{Skim, prelude::SkimOptionsBuilder};

use crate::{cli::PickArgs, ssh::host_finder::parse_hosts};

/// Choose a host
pub async fn pick(
    PickArgs {
        query,
        fields: _,
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
        .build()
        .unwrap();
    let output = Skim::run_items(options, hosts).unwrap();
    for item in output.selected_items {
        println!("Selected: {}", item.output());
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
