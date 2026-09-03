# Tasks
Pending tasks to implement. I will be updating this as I need to.

- [ ] Check kame's colors work well across terminal color schemes
- [ ] Remove tokio macros to improve compile time
    - Hopefully remove all uses of the syn crate

## SSH config parsing (`host_finder`)
- [ ] Recognize `Host=alpha` and `Host= alpha`
    - The first whitespace token is `Host=alpha`, which fails the
      `eq_ignore_ascii_case("host")` check in `host_finder.rs:30`, so the whole
      block is dropped and the alias never reaches the picker
    - `equal_sign_seen` (`host_finder.rs:41`) only covers the `Host =alpha`
      spelling
    - Failing test: `the_keyword_may_be_separated_by_an_equals_sign`
- [ ] Unquote arguments
    - `Host "alpha"` currently yields the alias `"alpha"`, quotes included,
      which is then handed to `ssh -G "\"alpha\""` and resolves the wrong host
    - `Host "hotel india"` yields two junk aliases
    - Failing tests: `quoted_arguments_are_unquoted`,
      `an_equals_sign_inside_a_pattern_is_an_ordinary_character`
- [ ] Consider replacing the ad-hoc token handling with a real tokenizer pass
    (strip one optional `=` after the keyword, then `argv_split`-style quote
    handling) instead of adding more special cases
- [ ] Decide what to do about `Include`
    - `parse_hosts` (`host_finder.rs:13`) opens exactly one file, so a config
      built around `Include ~/.ssh/config.d/*.conf` shows an empty or
      near-empty picker with no explanation
    - `REALISTIC_CONFIG` already pins the current behaviour; the tests need
      updating either way

## Error handling and exit codes
- [ ] Replace the `todo!()`s on documented flags with a clean error
    - `probe -v/-p/--json/-N` (`probe.rs:18-29`) and `pick --json/--preview-cmd`
      (`pick.rs:70-75`) panic with a backtrace note and exit 101
    - All of them are listed in `--help`, so users will find them
    - Either hide them (`.hide(true)`) until implemented, or return
      `anyhow!("--json is not implemented yet")`
- [ ] Fail gracefully when `pick` has no tty
    - `pick.rs:92` unwraps `Skim::run_items`; `kame pick < /dev/null` panics
      with `No such device or address (os error 6)` and exits 101
    - Want a one-line "kame pick needs an interactive terminal" on stderr
    - Same for `build_skim_options(...).unwrap()` at `pick.rs:91`
- [ ] Report `probe` failures on stderr with a non-zero exit
    - `probe.rs:46-51` writes `Error: Could not parse information for host: …`
      into the returned `String`, which `main.rs:32` prints to stdout
    - `kame probe "$h" > out` puts the error message in the caller's data file
      and still exits 0
- [ ] Stop leaking `???` into stdout for missing fields
    - `pick.rs:133` and `probe.rs:65`
    - `kame pick -L localhost -f nosuchfield` prints `???` and exits 0; `-L` is
      documented as "meant for scripting", so a typo'd field silently poisons
      the caller's variable
    - An unknown field should be a stderr error with a non-zero exit; a
      legitimately unset field should probably print nothing
- [ ] Add path context to IO errors
    - `File::open(&path).await?` (`host_finder.rs:15`) drops the path:
      `kame pick -F /tmp/nope.conf` prints only
      `Error: No such file or directory (os error 2)`
    - The common case is a first-time user with no `~/.ssh/config` at all, who
      gets the same bare message — probably deserves its own wording
- [ ] Do not abort multi-select halfway through
    - `pick.rs:150-153` uses `?` inside the loop, so if host 3 of 5 fails
      `ssh -G`, hosts 1-2 are already on stdout and the process exits non-zero
    - Collect the failures and report at the end, or keep going and exit
      non-zero
- [ ] Handle an empty selection
    - `pick.rs:146-148` falls back to `output.query`, which is `""` when Enter
      is pressed on an empty query, so `ssh $(kame pick)` degenerates into bare
      `ssh`
    - Also worth erroring up front when the config yields zero aliases
- [ ] Settle the exit-code convention
    - Abort is currently `exit(1)` (`pick.rs:143`)
    - fzf-style codes (130 for Ctrl-C, 1 for no match, 2 for error) are what
      scripts expect, and `-L` targets scripting

## Colors
- [ ] Actually apply `--color`
    - `main.rs:27` parses `cli.color` and never uses it, and `probe.rs:31-33`
      calls `control::set_override(true)` unconditionally
    - `--color never`, `NO_COLOR=1` and piping to a non-tty all still emit
      escape codes
    - The override was added to fix color in the skim preview (37f2c82); it
      belongs at the preview call site in `pick.rs`, not in `probe` globally
    - Colors should key off `cli.color` plus tty detection
- [ ] Drop the dead `plain` handling in `probe`
    - `plain` is `todo!()`'d at `probe.rs:21`, so the `if !plain` guard at
      `probe.rs:31` and the `plain` parameter of `render_host` are unreachable

## Output formatting
- [ ] Settle on one representation for multi-value fields
    - The `--field` help promises values "joined with commas"
    - `print_host` prints one per line (`pick.rs:135`)
    - `render_field` prints Rust debug syntax
      `["~/.ssh/id_rsa", "~/.ssh/id_ed25519"]` (`probe.rs:70`)
    - `identityfile` returns 5 values on a stock system, so this is reachable
    - Pick one and make the docs match
- [ ] Check the host glyph in `probe.rs:57`
    - It is `모` (U+BAA8, a Korean syllable), not a turtle or a box-drawing
      character; `pick` uses `🐢`/`🐚`, so this looks unintentional

## pick subcommand
- [ ] Ensure the only way to print to stdout is for printing skim's output
- [ ] Cache preview results
    - `SshHost::preview` (`pick.rs:32-46`) does a blocking `block_in_place` plus
      an `ssh -G` fork on every cursor move
    - A config with a slow `Match exec` freezes the whole TUI with no way out
    - A `Mutex<HashMap>` keyed by alias is a cheap win
- [ ] Stop the preview truncating long values
    - `PREVIEW_LAYOUT` is `Size::Fixed(40)` with `wrap: false`, so
      `Hostname some.long.name.example.com` is silently cut
    - Consider `Percentage`, or enable wrapping
- [ ] Make `-L` conflict with `-m`
    - `-m` is meaningless in literal mode but currently accepted
- [ ] Remove the unreachable "No host was given" branch
    - `query` is `required_if_eq(ARG_LITERAL, "true")`, so `pick.rs:81-82`
      cannot run
