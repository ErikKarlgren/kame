// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::cli::PickArgs;

/// Choose a host
pub fn pick(
    PickArgs {
        query,
        fields: _,
        multi,
        config,
        literal,
        json,
        preview_cmd,
    }: PickArgs,
) {
    if multi {
        eprintln!("--multi not implemented yet")
    }
    if config.is_some() {
        eprintln!("--config not implemented yet")
    }
    if literal {
        todo!("--literal not implemented yet");
    }
    if json {
        todo!("--json not implemented yet");
    }
    if preview_cmd.is_some() {
        todo!("--preview-cmd not implemented yet");
    }

    //let options
}
