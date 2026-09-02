// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    process::exit,
};

use anyhow::{Result, anyhow};
use clap::builder::styling::AnsiColor;
use skim::{
    ItemPreview, PreviewContext, Skim, SkimOutput,
    prelude::{SkimItem, SkimOptions, SkimOptionsBuilder, options::SkimOptionsBuilderError},
    tui::{
        BorderType,
        options::PreviewLayout,
        statusline::{Info, InfoDisplay},
    },
};
use tokio::runtime::Handle;

use crate::{
    cli::{PickArgs, ProbeArgs},
    probe::probe,
    ssh::{host_config::HostConfig, host_finder::parse_hosts},
};

struct SshHost {
    hostname: String,
    ssh_config: Option<PathBuf>,
}

impl SkimItem for SshHost {
    fn text(&self) -> std::borrow::Cow<'_, str> {
        Cow::Borrowed(&self.hostname)
    }
    fn preview(&self, _ctx: PreviewContext<'_>) -> ItemPreview {
        let text = tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                probe(ProbeArgs {
                    host: self.hostname.clone(),
                    verbose: false,
                    plain: false,
                    json: false,
                    no_probes: false,
                    config: self.ssh_config.clone(),
                })
                .await
            })
        });
        ItemPreview::Text(text)
    }
}

const PREVIEW_LAYOUT: PreviewLayout = PreviewLayout {
    direction: skim::tui::Direction::Left,
    size: skim::tui::Size::Percent(40),
    hidden: false,
    wrap: true,
    pty: false,
    offset: None,
};

/// Choose a host
pub async fn pick(
    PickArgs {
        query,
        fields,
        multi,
        config,
        literal,
        json,
        preview_cmd,
    }: PickArgs,
) -> Result<()> {
    if json {
        todo!("--json not implemented yet");
    }
    if preview_cmd.is_some() {
        todo!("--preview-cmd not implemented yet");
    }

    if literal {
        if let Some(host) = query {
            return print_host(host, &fields, config.as_deref()).await;
        }
        eprintln!("No host was given");
        exit(1);
    }

    let path = choose_config_path(config.as_deref())?;
    let hosts = parse_hosts(&path)
        .await?
        .into_iter()
        .map(|hostname| SshHost {
            hostname,
            ssh_config: config.clone(),
        });

    let options = build_skim_options(query, multi).unwrap();
    let output = Skim::run_items(options, hosts).unwrap();
    print_skim_output(&output, &fields, config.as_deref()).await?;
    Ok(())
}

fn build_skim_options(
    query: Option<String>,
    multi: bool,
) -> Result<SkimOptions, SkimOptionsBuilderError> {
    use AnsiColor::*;
    let skim_colors = format!(
        "16,current:{}:bold,current_bg:{},matched:{},current_match:{}:bold:underline,border:{},prompt:{},header:{},selected:{}",
        Black as u8,
        Yellow as u8,
        Green as u8,
        Blue as u8,
        BrightBlack as u8,
        Green as u8,
        Green as u8,
        Yellow as u8,
    );
    SkimOptionsBuilder::default()
        .multi(multi)
        .query(query.unwrap_or_default())
        .height("40%")
        .min_height("16")
        .preview("") // needed to enable the preview pane
        .preview_window(PREVIEW_LAYOUT)
        .header(if multi {
            "Pick hosts with TAB or SHIFT+TAB and press Enter"
        } else {
            "Pick a host and press Enter"
        })
        .selector_icon("🐢")
        .multi_select_icon("🐚")
        .highlight_line(true)
        .color(skim_colors)
        .cycle(true)
        .show_cmd_error(true)
        .border(BorderType::Rounded)
        .border_no_collapse(true)
        .info(Info {
            display: InfoDisplay::Hidden,
            separator: None,
        })
        .build()
}

async fn print_host<S: AsRef<str>>(
    host: S,
    fields: &[String],
    custom_config: Option<&Path>,
) -> Result<()> {
    if fields.is_empty() {
        println!("{}", host.as_ref());
        return Ok(());
    }
    let host_cfg = HostConfig::from_host(host.as_ref(), custom_config).await?;
    for field in fields {
        let value_not_found = ["???".to_owned()];
        for value in host_cfg.get(field).unwrap_or(&value_not_found) {
            println!("{value}");
        }
    }
    Ok(())
}

async fn print_skim_output(
    output: &SkimOutput,
    fields: &[String],
    custom_config: Option<&Path>,
) -> Result<()> {
    if output.is_abort {
        exit(1);
    }

    if output.selected_items.is_empty() {
        return print_host(&output.query, fields, custom_config).await;
    }

    for host in &output.selected_items {
        let host = host.text();
        print_host(host, fields, custom_config).await?;
    }
    Ok(())
}

fn choose_config_path(config: Option<&Path>) -> Result<PathBuf> {
    config.map_or_else(default_ssh_config_path, |path| Ok(path.to_owned()))
}

fn default_ssh_config_path() -> std::prelude::v1::Result<PathBuf, anyhow::Error> {
    dirs::home_dir()
        .map(|dir| dir.join(".ssh/config"))
        .ok_or_else(|| {
            anyhow!(
                "No home directory found. Please use the flag --config and provide an ssh config file"
            )
        })
}
