use anyhow::{Result, anyhow};
use std::collections::HashMap;
use tokio::process::Command;

pub struct ShellCfg {
    var_map: HashMap<String, Vec<String>>,
}

impl ShellCfg {
    pub async fn new(hostname: &str) -> Result<ShellCfg> {
        let output = Command::new("ssh").args(&["-G", hostname]).output().await?;

        if !output.status.success() {
            return Err(anyhow!(
                "Command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let var_map = parse_stdout(&stdout);
        Ok(ShellCfg { var_map })
    }

    pub fn get(&self, key: &str) -> Option<&[String]> {
        self.var_map.get(key).map(|val| val.as_slice())
    }
}

fn parse_stdout(stdout: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for line in stdout.lines() {
        let (key, val) = line.split_once(" ").unwrap();
        map.entry(key.to_owned()).or_default().push(val.to_owned());
    }
    map
}
