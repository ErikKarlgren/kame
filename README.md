# kame
<a href="https://crates.io/crates/kame">
    <img src="https://img.shields.io/crates/v/kame.svg" alt="Crates.io" />
</a>

An SSH toolkit for your terminal

**NOTE:** This program is still in *alpha*, so there are still a lot of bugs.

## Features
- `pick`: Show the aliases present in your SSH config in an interactive menu and select one (or more) with its fuzzy finder
    - This subcommand follows the UNIX philosophy, so you can run `ssh "$(kame pick)` to choose which SSH host to connect to
    - By using flags like `-i`/`--hostname` you can use `curl https://$(kame pick -i)/api/v1/users` to run a GET request to any of your servers
- `probe`: Show detailed info about a given host, including: hostname, user, port, *latency* (TBD), *connection status* (TBD), ...

## Dependencies

`kame` requires OpenSSH in your system, or at least an `ssh` binary in your `$PATH` that can be called as `ssh -G <host>` that has the same format as OpenSSH's. If you already have `ssh` installed in your system, it's probably OpenSSH.

## Installation

If you have `cargo` installed, run:
```bash
cargo install kame
```

Otherwise, please open an issue if you would like having a dedicated install script.

## License

Licensed under the [GNU Affero General Public License v3.0 or later](LICENSE).
