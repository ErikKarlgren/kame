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
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim_start();
        if trimmed.starts_with("Host") {
            for host in trimmed.split_whitespace().skip(1) {
                // We only care about real aliases, not patterns
                if host.contains("!") {
                    break;
                }
                if !host.contains('*') && !host.contains('[') && !host.contains(']') {
                    hosts.insert(host.to_owned());
                }
            }
        }
    }

    Ok(hosts)
}
