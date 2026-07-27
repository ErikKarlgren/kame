use anyhow::Result;

use crate::ssh::shell_cfg::ShellCfg;

mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    println!("kame probe!");
    let cfg = ShellCfg::new("google.com").await?;
    println!("hostname: {:?}", cfg.get("sendenv"));
    Ok(())
}
