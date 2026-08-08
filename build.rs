// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Build-time check that every Rust file in the repository opens with the
//! project's SPDX header, and that the identifier it declares is the same one
//! `package.license` gives in `Cargo.toml`.
//!
//! Every `.rs` file must start with exactly these two lines:
//!
//! ```text
//! // SPDX-FileCopyrightText: <holder>
//! // SPDX-License-Identifier: <the license from Cargo.toml>
//! ```
//!
//! The holder is only required to be non-empty, so the year and the name can
//! change without touching this file. The identifier is compared verbatim.
//!
//! The check runs only inside a working copy, detected by a `.git` entry next
//! to `Cargo.toml`. Published crates do not ship `.git`, so this never runs —
//! and so can never fail — while someone is installing kame.

use std::fs;
use std::path::{Path, PathBuf};

/// Expected prefix of the first line of every source file.
const COPYRIGHT_TAG: &str = "// SPDX-FileCopyrightText:";
/// Expected prefix of the second line of every source file.
const LICENSE_TAG: &str = "// SPDX-License-Identifier:";

fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo always sets CARGO_MANIFEST_DIR");
    let root = Path::new(&root);

    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=build.rs");

    // `.git` is a directory in a normal clone but a file in a git worktree,
    // so test for existence rather than for a directory.
    if !root.join(".git").exists() {
        return;
    }

    let license = std::env::var("CARGO_PKG_LICENSE").unwrap_or_default();
    if license.is_empty() {
        println!("cargo::error=package.license is missing from Cargo.toml");
        return;
    }

    let mut files = Vec::new();
    collect(root, true, &mut files);
    files.sort();

    for file in files {
        if let Err(reason) = check(&file, &license) {
            let shown = file.strip_prefix(root).unwrap_or(&file);
            println!("cargo::error={}: {reason}", shown.display());
        }
    }
}

/// Gathers every `.rs` file below `dir`, skipping `target/` and dotted
/// directories — `.claude` holds worktrees, which are whole copies of the
/// repository and would otherwise be scanned as well.
fn collect(dir: &Path, is_root: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    // Watching a directory is what makes cargo notice a *newly added* file.
    // The root is deliberately left unwatched: it contains `target/`, whose
    // mtime changes on every build, which would rerun this script every time.
    if !is_root {
        println!("cargo::rerun-if-changed={}", dir.display());
    }

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if path.is_dir() {
            if name == "target" || name.starts_with('.') {
                continue;
            }
            collect(&path, false, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Returns `Err` with a reason to show the user when `file` does not open with
/// the two expected SPDX lines.
fn check(file: &Path, license: &str) -> Result<(), String> {
    let text = fs::read_to_string(file).map_err(|e| format!("cannot be read: {e}"))?;
    // Trim a byte order mark so an editor that writes one cannot hide the tag.
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    let first = lines.next().unwrap_or_default().trim_end();
    let second = lines.next().unwrap_or_default().trim_end();

    let holder = first.strip_prefix(COPYRIGHT_TAG).map(str::trim);
    if holder.is_none_or(str::is_empty) {
        return Err(format!("first line must be `{COPYRIGHT_TAG} <holder>`"));
    }

    match second.strip_prefix(LICENSE_TAG).map(str::trim) {
        None => Err(format!("second line must be `{LICENSE_TAG} {license}`")),
        Some(found) if found != license => Err(format!(
            "declares `{found}`, but Cargo.toml says `{license}`"
        )),
        Some(_) => Ok(()),
    }
}
