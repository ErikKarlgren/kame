use std::env;

use anyhow::Result;

use crate::ssh::host_config::HostConfig;

mod ssh;

#[tokio::main]
async fn main() -> Result<()> {
    let tasks: Vec<_> = env::args()
        .skip(1)
        .map(|host| {
            tokio::spawn(async move {
                let cfg = HostConfig::parse(&host).await;
                (host, cfg)
            })
        })
        .collect();

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
    println!(
        "{}: {:?}",
        field,
        cfg.get(field).unwrap_or(&["???".to_owned()])
    );
}
