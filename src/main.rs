use std::env;

use anyhow::Result;

use crate::ssh::shell_cfg::ShellCfg;

mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    for host in env::args().skip(1) {
        let cfg = ShellCfg::new(&host).await?;
        println!(">>> {host}");
        print_field(&cfg, "hostname");
        print_field(&cfg, "user");
        print_field(&cfg, "port");
        print_field(&cfg, "controlmaster");
        println!();
    }
    Ok(())
}

fn print_field(shell_cfg: &ShellCfg, field: &str) {
    println!(
        "{}: {:?}",
        field,
        shell_cfg.get(field).unwrap_or(&["???".to_owned()])
    );
}
