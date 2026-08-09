// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

/// Parse hosts from a single file
pub async fn parse_hosts(path: PathBuf) -> Result<HashSet<String>> {
    let mut hosts: HashSet<String> = HashSet::new();
    let file = File::open(&path).await?;
    let reader = BufReader::new(file);
    let lines = reader.lines();
    extract_hosts(&mut hosts, lines).await?;
    Ok(hosts)
}

async fn extract_hosts(
    hosts: &mut HashSet<String>,
    mut lines: tokio::io::Lines<BufReader<File>>,
) -> Result<(), anyhow::Error> {
    while let Some(line) = lines.next_line().await? {
        let mut words = line.split_ascii_whitespace();
        if words.next() == Some("Host") {
            for host in words {
                // We only care about real aliases, not patterns
                if host.contains('!') {
                    break;
                }
                if !host.contains('*') && !host.contains('[') && !host.contains(']') {
                    hosts.insert(host.to_owned());
                }
            }
        }
    }
    Ok(())
}
