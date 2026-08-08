// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![warn(clippy::cargo)]
#![deny(clippy::style)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::nursery)]

use anyhow::anyhow;
use std::env;

use anyhow::Result;

use crate::{
    cli::parse_args,
    ssh::{host_config::HostConfig, host_finder::parse_hosts},
};

mod cli;
mod pick;
mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = parse_args();
    let tasks: Vec<_> = env::args()
        .skip(1)
        .map(|host| {
            tokio::spawn(async move {
                let cfg = HostConfig::parse(&host).await;
                (host, cfg)
            })
        })
        .collect();

    if tasks.is_empty() {
        println!("Parsing hosts");
        let hosts = parse_hosts(
            dirs::home_dir()
                .ok_or_else(|| anyhow!("no home dir found"))?
                .join(".ssh/config"),
        )
        .await?;
        for h in hosts {
            println!("{h}");
        }
        return Ok(());
    }

    for task in tasks {
        let (host, cfg) = task.await?;
        let cfg = cfg?;
        println!(">>> {host}");
        print_field(&cfg, "hostname");
        print_field(&cfg, "user");
        print_field(&cfg, "port");
        print_field(&cfg, "controlmaster");
        println!();
    }
    Ok(())
}

fn print_field(cfg: &HostConfig, field: &str) {
    let default = &["???".to_owned()];
    println!("{}: {:?}", field, cfg.get(field).unwrap_or(default));
}
