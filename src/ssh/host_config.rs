// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Access to the SSH configuration that OpenSSH resolves for a given host.
//!
//! Rather than parsing `~/.ssh/config` (and its `Include`s, `Match` blocks,
//! wildcards and system-wide defaults) ourselves, we ask `ssh` itself via
//! `ssh -G <host>` and read back the fully resolved key/value pairs.

use anyhow::{Result, anyhow};
use std::{collections::HashMap, ffi::OsString, path::Path, vec::Vec};
use tokio::process::Command;

/// The effective SSH configuration for a single host, as reported by
/// `ssh -G <host>`.
///
/// Keys are the lowercase option names printed by `ssh -G` (`hostname`, `user`,
/// `port`, `controlmaster`, ...). A key may appear more than once in the output
/// — `identityfile` and `sendenv` typically do — so every key maps to a list of
/// values in the order `ssh` printed them.
///
/// The configuration is a snapshot taken when [`HostConfig::parse`] ran; it is
/// not refreshed if the underlying config files change.
pub struct HostConfig {
    var_map: HashMap<String, Vec<String>>,
}

impl HostConfig {
    /// Resolves the SSH configuration for `hostname` by running
    /// `ssh -G <hostname>` and parsing its output.
    ///
    /// `hostname` is the alias as you would type it after `ssh`, so it may be a
    /// `Host` entry from the SSH config rather than a real DNS name. No network
    /// connection is made: `-G` only evaluates the configuration.
    ///
    /// `custom_config` is an optional path to a non-default ssh config path. If
    /// `None`, the default ssh config path will be used.
    ///
    /// # Errors
    ///
    /// Returns an error if `ssh` cannot be spawned (e.g. not on `PATH`), if it
    /// exits with a non-zero status, in which case its stderr is included in the
    /// message, or if the output format isn't the expected one.
    pub async fn parse(hostname: &str, custom_config: Option<&Path>) -> Result<Self> {
        let mut args: Vec<OsString> = vec!["-G".into(), hostname.into()];
        if let Some(custom_config) = custom_config {
            if !custom_config.is_file() {
                return Err(anyhow!(
                    "Error: path '{}' is not a regular file",
                    custom_config.display()
                ));
            }
            args.extend(["-F".into(), custom_config.into()]);
        }
        let output = Command::new("ssh").args(args).output().await?;

        if !output.status.success() {
            return Err(anyhow!(
                "Command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let var_map = parse_stdout(&stdout)?;
        Ok(Self { var_map })
    }

    /// Returns the values `ssh` reported for the option `key`, or `None` if the
    /// option was not present in the output.
    ///
    /// `key` must be lowercase, matching the way `ssh -G` prints option names.
    /// Options that appear only once still yield a single-element slice.
    pub fn get(&self, key: &str) -> Option<&[String]> {
        self.var_map.get(key).map(Vec::as_slice)
    }
}

/// Parses `ssh -G` output into a map from option name to its values.
///
/// Each line is expected to be `<name> <value>`, split at the first space so
/// values containing spaces stay intact. Repeated names accumulate in output
/// order.
///
/// # Errors
///
/// Returns an error if a line fails to be parsed
fn parse_stdout(stdout: &str) -> Result<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (num, line) in stdout.lines().enumerate() {
        let (key, val) = line
            .split_once(' ')
            .ok_or_else(|| anyhow!("Unexpected format: could not parse line \"{}\"", num + 1))?;
        map.entry(key.to_owned()).or_default().push(val.to_owned());
    }
    Ok(map)
}
