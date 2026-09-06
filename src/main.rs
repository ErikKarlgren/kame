// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::stdout;

use anyhow::{Context, Result};

use crate::{
    cli::{Subcommand, parse_args},
    pick::pick,
    probe::probe,
};

mod cli;
mod pick;
mod probe;
mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_args();
    match cli.command {
        Subcommand::Pick(pick_args) => pick(pick_args).await?,
        Subcommand::Probe(probe_args) => {
            let stdout = stdout();
            probe(&mut stdout.lock(), probe_args, None)
                .await
                .context("Could not run `kame probe`")?;
        }
    }
    Ok(())
}
