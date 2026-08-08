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
    if config.is_some() {
        eprintln!("--config not implemented yet")
    }
    if literal {
        todo!("--literal not implemented yet");
    }
    if json {
        todo!("--json not implemented yet");
    }
    if preview_cmd.is_some() {
        todo!("--preview-cmd not implemented yet");
    }

    let hosts = parse_hosts(
        dirs::home_dir()
            .ok_or(anyhow!("no home dir found"))?
            .join(".ssh/config"),
    )
    .await?;

    let options = SkimOptionsBuilder::default()
        .multi(multi)
        .query(query.unwrap_or("".into()))
        .height("16")
        .build()
        .unwrap();
    let output = Skim::run_items(options, hosts).unwrap();
    for item in output.selected_items {
        println!("Selected: {}", item.output());
    }
    Ok(())
}
