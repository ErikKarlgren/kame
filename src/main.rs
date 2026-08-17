// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![warn(clippy::cargo)]
#![deny(clippy::style)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]

use anyhow::Result;

use crate::{
    cli::{Subcommand, parse_args},
    pick::pick,
};

mod cli;
mod colors;
mod pick;
mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_args();
    match cli.command {
        Subcommand::Pick(pick_args) => pick(pick_args).await?,
        Subcommand::Probe(_probe_args) => todo!(),
    }
    Ok(())
}
