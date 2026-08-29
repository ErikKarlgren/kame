// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{borrow::Cow, fmt::Display, process::exit};

use anyhow::{Result, anyhow};
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
                })
                .await
            })
        });
        ItemPreview::Text(text)
    }
}

const PREVIEW_LAYOUT: PreviewLayout = PreviewLayout {
    direction: skim::tui::Direction::Left,
    size: skim::tui::Size::Fixed(40),
    hidden: false,
    wrap: false,
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
    if literal {
        todo!("--literal not implemented yet");
    }
    if json {
        todo!("--json not implemented yet");
    }
    if preview_cmd.is_some() {
        todo!("--preview-cmd not implemented yet");
    }

    let path = choose_config_path(config)?;
    let hosts = parse_hosts(path)
        .await?
        .into_iter()
        .map(|hostname| SshHost { hostname });

    let options = build_skim_options(query, multi).unwrap();
    let output = Skim::run_items(options, hosts).unwrap();
    print_to_stdout(&output, &fields).await?;
    Ok(())
}

fn build_skim_options(
    query: Option<String>,
    multi: bool,
) -> Result<SkimOptions, SkimOptionsBuilderError> {
    SkimOptionsBuilder::default()
        .multi(multi)
        .query(query.unwrap_or_default())
        .height("16")
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
        .color(
            "16,current_bg:2,current:0:bold,matched:1,current_match:9,current_match_bg:2,border:8,prompt:3,header:3,selected:3",
        )
        .cycle(true)
        .show_cmd_error(true)
        .border(BorderType::Rounded)
        .border_no_collapse(true)
        .info(Info{ display: InfoDisplay::Hidden, separator: None })
        .build()
}

async fn print_to_stdout(output: &SkimOutput, fields: &[String]) -> Result<()> {
    async fn print_host<D: Display + Into<String>>(host: D, fields: &[String]) -> Result<()> {
        if fields.is_empty() {
            println!("{host}");
            return Ok(());
        }
        let host_cfg = HostConfig::parse(&host.into()).await?;
        for field in fields {
            let value_not_found = ["???".to_owned()];
            for value in host_cfg.get(field).unwrap_or(&value_not_found) {
                println!("{value}");
            }
        }
        Ok(())
    }

    if output.is_abort {
        exit(1);
    }

    if output.selected_items.is_empty() {
        return print_host(&output.query, fields).await;
    }

    for host in &output.selected_items {
        let host = host.text();
        print_host(host, fields).await?;
    }
    Ok(())
}

fn choose_config_path(
    config: Option<std::path::PathBuf>,
) -> Result<std::path::PathBuf, anyhow::Error> {
    config.map_or_else(
        #[allow(clippy::option_if_let_else, reason="if let else is clearer")]
        || match dirs::home_dir() {
            Some(p) => Ok(p.join(".ssh/config")),
            None => Err(anyhow!(
                "No home directory found. Please use the flag --config and provide an ssh config file"
            )),
        }
        , Ok)
}
