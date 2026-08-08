//! Command-line parsing for `kame`.
//!
//! The whole interface lives here and is built with clap's *builder* API — the
//! `derive` feature is deliberately off, so no proc macros (and no `syn`) are
//! pulled into the build.
//!
//! [`parse_args`] is the entry point: it either returns a fully normalized [`Cli`]
//! or exits the process the way clap does for `--help`, `--version` and usage
//! errors. [`try_parse_from`] does the same without exiting and is what the
//! tests use.
//!
//! "Normalized" means the parser resolves the interface's shorthands so that
//! the rest of the program never has to:
//!
//! - `-i`/`-u`/`-p`/`-c` are folded into [`PickArgs::fields`], in the order the
//!   user wrote them on the command line and interleaved with `-f/--field`.
//! - Field names are lowercased (matching `ssh -G` output keys) and duplicates
//!   are dropped, keeping the first position.
//! - `-L/--literal` implies `hostname` when no other field was requested.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::styling::{Ansi256Color, Style};
use clap::builder::{EnumValueParser, PossibleValue, Styles, ValueParser};
use clap::{Arg, ArgAction, ArgMatches, Command, Error, ValueEnum};

/// Subcommand name for the fuzzy picker.
const CMD_PICK: &str = "pick";
/// Subcommand name for the host prober.
const CMD_PROBE: &str = "probe";

const ARG_COLOR: &str = "color";
const ARG_VERSION: &str = "version";

const ARG_QUERY: &str = "query";
const ARG_FIELD: &str = "field";
const ARG_HOSTNAME: &str = "hostname";
const ARG_USER: &str = "user";
const ARG_PORT: &str = "port";
const ARG_CONTROL_PATH: &str = "control-path";
const ARG_MULTI: &str = "multi";
const ARG_CONFIG: &str = "config";
const ARG_LITERAL: &str = "literal";
const ARG_PREVIEW_CMD: &str = "preview-cmd";

const ARG_HOST: &str = "host";
const ARG_VERBOSE: &str = "verbose";
const ARG_PLAIN: &str = "plain";
const ARG_NO_PROBES: &str = "no-probes";

const ARG_JSON: &str = "json";

/// Short flags that are aliases for a `--field <FIELD>` lookup, paired with the
/// `ssh -G` key they stand for.
const FIELD_ALIASES: [(&str, &str); 4] = [
    (ARG_HOSTNAME, "hostname"),
    (ARG_USER, "user"),
    (ARG_PORT, "port"),
    (ARG_CONTROL_PATH, "controlpath"),
];

/// The field `-L/--literal` falls back to when no other field is requested.
const LITERAL_DEFAULT_FIELD: &str = "hostname";

/// When to colorize output, from `--color <WHEN>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorChoice {
    /// Colorize only when stdout is a terminal.
    #[default]
    Auto,
    /// Always colorize, even when piped.
    Always,
    /// Never colorize.
    Never,
}

impl ValueEnum for ColorChoice {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Auto, Self::Always, Self::Never]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Auto => PossibleValue::new("auto"),
            Self::Always => PossibleValue::new("always"),
            Self::Never => PossibleValue::new("never"),
        })
    }
}

/// A fully parsed and normalized `kame` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    /// Value of the global `--color <WHEN>` flag.
    pub color: ColorChoice,
    /// The subcommand to run, with its own arguments.
    pub command: Subcommand,
}

/// The subcommand the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subcommand {
    /// `kame pick` — fuzzily pick a host from the SSH config.
    Pick(PickArgs),
    /// `kame probe` — probe a single host and report on it.
    Probe(ProbeArgs),
}

/// Arguments of `kame pick`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PickArgs {
    /// Optional text to seed the fuzzy query with, or the literal pattern when
    /// `literal` is set.
    pub query: Option<String>,
    /// Fields to resolve and print, lowercased, deduplicated and in the order
    /// given on the command line. Empty means "print the alias itself".
    pub fields: Vec<String>,
    /// `-m/--multi`: allow picking several hosts.
    pub multi: bool,
    /// `-F/--config`: search this file instead of `~/.ssh/config`.
    pub config: Option<PathBuf>,
    /// `-L/--literal`: skip the fuzzy search and pass `query` to `ssh -G`.
    pub literal: bool,
    /// `--json`: print a JSON object (or array with `--multi`).
    pub json: bool,
    /// `--preview-cmd`: the command and arguments overriding the default
    /// preview. `{}` is a placeholder for the SSH alias.
    pub preview_cmd: Option<Vec<String>>,
}

/// Arguments of `kame probe`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per command-line flag; a state machine would obscure the mapping"
)]
pub struct ProbeArgs {
    /// The SSH host (or config alias) to probe.
    pub host: String,
    /// `-v/--verbose`: show detailed diagnostics.
    pub verbose: bool,
    /// `-p/--plain`: plain text output, no colors or decorations.
    pub plain: bool,
    /// `--json`: print a JSON object. Mutually exclusive with `plain`.
    pub json: bool,
    /// `-N/--no-probes`: only report what `ssh -G` says, run no network probes.
    pub no_probes: bool,
}

/// Parses the process arguments, exiting on `--help`, `--version` or a usage
/// error just like clap's own `get_matches`.
#[must_use]
pub fn parse_args() -> Cli {
    match try_parse_from(std::env::args_os()) {
        Ok(cli) => cli,
        Err(err) => err.exit(),
    }
}

/// Parses an arbitrary argument list, including `argv[0]`.
///
/// Unlike [`parse_args`] this never terminates the process, which makes it usable
/// from tests.
///
/// # Errors
///
/// Returns the [`clap::Error`] describing a usage problem, or the "error" clap
/// uses to carry the rendered `--help` / `--version` output.
pub fn try_parse_from<I, T>(args: I) -> Result<Cli, Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut cmd = command();
    let matches = cmd.try_get_matches_from_mut(args)?;
    from_matches(&mut cmd, &matches)
}

/// Create styles following the color-scheme of a slider turtle
fn turtle_styles() -> Styles {
    Styles::styled()
        .header(Style::new().fg_color(Some(Ansi256Color(64).into())).bold())
        .usage(Style::new().fg_color(Some(Ansi256Color(64).into())).bold())
        .literal(Style::new().fg_color(Some(Ansi256Color(178).into())))
        .placeholder(Style::new().fg_color(Some(Ansi256Color(184).into())))
}

/// Builds the clap [`Command`] describing the whole interface.
#[must_use]
pub fn command() -> Command {
    Command::new("kame")
        .version(env!("CARGO_PKG_VERSION"))
        .styles(turtle_styles())
        .about("An SSH toolkit")
        // `-V` is top level only: `pick` already uses `-p` for `--port` and
        // `probe` for `--plain`, so a propagated version flag would be
        // confusing.
        .disable_version_flag(true)
        // The reference help lists `pick` and `probe` only.
        .disable_help_subcommand(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new(ARG_VERSION)
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help("Print version info and exit"),
        )
        .arg(
            Arg::new(ARG_COLOR)
                .long("color")
                .value_name("WHEN")
                .global(true)
                .default_value("auto")
                .hide_default_value(true)
                .value_parser(EnumValueParser::<ColorChoice>::new())
                .help("Coloring"),
        )
        .subcommand(pick_command())
        .subcommand(probe_command())
}

fn pick_command() -> Command {
    Command::new(CMD_PICK)
        .visible_alias("p")
        .about("Fuzzy search and pick host from your ssh config. Output can be controlled with flags")
        .long_about(
            "Fuzzily pick an SSH alias from your SSH config and prints it. \
             Uses \"kame probe\" as the default preview.",
        )
        .arg(
            Arg::new(ARG_QUERY)
                .value_name("query")
                .required_if_eq(ARG_LITERAL, "true")
                .help("Optional text to start the fuzzy query with"),
        )
        .args(pick_field_args())
        .arg(
            Arg::new(ARG_MULTI)
                .short('m')
                .long("multi")
                .action(ArgAction::SetTrue)
                .help(
                    "Choose multiple hosts and print desired info for all of them in separate lines",
                ),
        )
        .arg(
            Arg::new(ARG_CONFIG)
                .short('F')
                .long("config")
                .value_name("FILE")
                .value_parser(ValueParser::path_buf())
                .help("Search aliases in the given file instead of ~/.ssh/config"),
        )
        .arg(
            Arg::new(ARG_LITERAL)
                .short('L')
                .long("literal")
                .action(ArgAction::SetTrue)
                .help(
                    "Skip the fuzzy search and pass [query] directly to `ssh -G`. \
                     Implies `-i` by default unless another flag is used. Meant for scripting",
                ),
        )
        .arg(
            Arg::new(ARG_JSON)
                .long("json")
                .action(ArgAction::SetTrue)
                .help(
                    "Print output as json object (or array of objects with `-m`) instead of \
                     plain text. If a field has multiple values, print them as a json array",
                ),
        )
        .arg(
            // Greedy on purpose: everything after the flag belongs to the
            // preview command, hyphens included, so it has to come last.
            Arg::new(ARG_PREVIEW_CMD)
                .long("preview-cmd")
                .value_name("CMD")
                .num_args(1..)
                .allow_hyphen_values(true)
                .help(
                    "Override the preview command. Anything after this flag will be treated as \
                     an argument to it (e.g. `--preview-cmd ssh -G {}` will run `ssh -G {}`, \
                     no need for quotes). `{}` is a placeholder for the SSH alias",
                ),
        )
        .after_help(pick_examples())
}

/// The `--field` flag and its four single-letter shorthands.
fn pick_field_args() -> [Arg; 5] {
    [
        Arg::new(ARG_FIELD)
            .short('f')
            .long("field")
            .value_name("FIELD")
            .action(ArgAction::Append)
            .help(
                "Resolve the exact host (via `ssh -G`) and print the specified field. \
                 If field has multiple values (e.g. identityfile), they are joined with \
                 commas. Can be called multiple times",
            ),
        Arg::new(ARG_HOSTNAME)
            .short('i')
            .long("hostname")
            .action(ArgAction::SetTrue)
            .help("Alias of `--field hostname`"),
        Arg::new(ARG_USER)
            .short('u')
            .long("user")
            .action(ArgAction::SetTrue)
            .help("Alias of `--field user`"),
        Arg::new(ARG_PORT)
            .short('p')
            .long("port")
            .action(ArgAction::SetTrue)
            .help("Alias of `--field port`"),
        Arg::new(ARG_CONTROL_PATH)
            .short('c')
            .long("control-path")
            .action(ArgAction::SetTrue)
            .help("Alias of `--field controlpath`"),
    ]
}

/// Example invocations listed at the bottom of `kame pick --help`, as
/// (command, description) pairs.
///
/// A description may contain newlines; continuation lines are re-indented to
/// the description column by [`pick_examples`], so the pairs stay readable here
/// no matter how the columns end up laid out.
const PICK_EXAMPLES: [(&str, &str); 8] = [
    ("ssh $(kame pick)", "Pick a host and connect to it with SSH"),
    (
        "ssh -J proxy $(kame pick prod)",
        "Pick a host and connect to it through a proxy. Starts fuzzy search with \"prod\"",
    ),
    (
        "scp file $(kame pick):/tmp",
        "Pick a host and copy a file to its /tmp dir",
    ),
    (
        "curl https://$(kame pick -i)/v1/api",
        "Pick a host and curl its hostname",
    ),
    (
        "rm $(kame pick -m -c)",
        "Pick hosts and remove their control paths",
    ),
    (
        "kame pick -m -i -u --json",
        "Pick hosts and print their hostnames and users as a json array",
    ),
    (
        "kame pick -L -f identityfile 'web-*'",
        "Print the SSH key files used for web-* hosts (quote any args with '*' when run in a shell)",
    ),
    (
        "kame pick --preview-cmd cowsay -s {}",
        "Pick a host with a funny preview (calls `cowsay -s {}`)",
    ),
];

/// Indentation of an example row.
const EXAMPLE_INDENT: &str = "  ";
/// Gap between the command column and the description column.
const EXAMPLE_GAP: &str = "  ";

/// Renders the `Examples:` block, colored like the rest of the help.
///
/// clap does not style `after_help` for us — it treats it as opaque text and
/// appends it verbatim — so the escapes are written by hand here, reusing the
/// very [`Styles`] the rest of the help is built from so the two can't drift
/// apart. When color is off, `anstream` strips these escapes on the way out
/// along with clap's own, leaving the plain columns below.
fn pick_examples() -> String {
    use std::fmt::Write as _;

    let styles = turtle_styles();
    let header = styles.get_header();
    let literal = styles.get_literal();

    let command_width = PICK_EXAMPLES
        .iter()
        .map(|(command, _)| command.chars().count())
        .max()
        .unwrap_or_default();
    let continuation = format!(
        "\n{:width$}",
        "",
        width = EXAMPLE_INDENT.len() + command_width + EXAMPLE_GAP.len()
    );

    let mut examples = format!("{header}Examples:{header:#}");
    for (command, description) in PICK_EXAMPLES {
        let padding = command_width - command.chars().count();
        let description = description.replace('\n', &continuation);
        write!(
            examples,
            "\n{EXAMPLE_INDENT}{literal}{command}{literal:#}{:padding$}{EXAMPLE_GAP}{description}",
            ""
        )
        .expect("writing to a String cannot fail");
    }
    examples
}

fn probe_command() -> Command {
    Command::new(CMD_PROBE)
        .about("Probe a host for detailed ssh info")
        .long_about("Probe an SSH host and show config info with optional network health checks")
        .arg(Arg::new(ARG_HOST).value_name("host").required(true).help(
            "SSH host to probe and show info about. Works with aliases defined in ssh config",
        ))
        .arg(
            Arg::new(ARG_VERBOSE)
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help(
                    "Show detailed diagnostic info: SSH banner, auth methods, negotiated ciphers, \
                     TCP vs SSH handshake timing, ProxyJump chain",
                ),
        )
        .arg(
            Arg::new(ARG_PLAIN)
                .short('p')
                .long("plain")
                .action(ArgAction::SetTrue)
                .conflicts_with(ARG_JSON)
                .help(
                    "Plain text output, without colors or decorations. Incompatible with `--json`",
                ),
        )
        .arg(
            Arg::new(ARG_JSON)
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Print output as json object. Incompatible with `--plain`"),
        )
        .arg(
            Arg::new(ARG_NO_PROBES)
                .short('N')
                .long("no-probes")
                .action(ArgAction::SetTrue)
                .help("Skip network probes, only show info parsed from `ssh -G`"),
        )
}

/// Turns clap's [`ArgMatches`] into the normalized [`Cli`].
///
/// `cmd` is only needed to render the help text when no subcommand was given.
fn from_matches(cmd: &mut Command, matches: &ArgMatches) -> Result<Cli, Error> {
    let Some((name, sub)) = matches.subcommand() else {
        let help = cmd.render_help();
        return Err(cmd.error(
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
            help,
        ));
    };

    // `--color` is global, so it lands in the subcommand's matches whichever
    // side of the subcommand name the user wrote it on.
    let color = sub
        .get_one::<ColorChoice>(ARG_COLOR)
        .copied()
        .unwrap_or_default();

    let command = match name {
        CMD_PICK => Subcommand::Pick(pick_args(sub)),
        CMD_PROBE => Subcommand::Probe(probe_args(sub)),
        other => {
            return Err(cmd.error(
                clap::error::ErrorKind::InvalidSubcommand,
                format!("unknown subcommand '{other}'"),
            ));
        }
    };

    Ok(Cli { color, command })
}

fn pick_args(m: &ArgMatches) -> PickArgs {
    let literal = m.get_flag(ARG_LITERAL);
    let mut fields = collect_fields(m);
    if literal && fields.is_empty() {
        fields.push(LITERAL_DEFAULT_FIELD.to_owned());
    }

    PickArgs {
        query: m.get_one::<String>(ARG_QUERY).cloned(),
        fields,
        multi: m.get_flag(ARG_MULTI),
        config: m.get_one::<PathBuf>(ARG_CONFIG).cloned(),
        literal,
        json: m.get_flag(ARG_JSON),
        preview_cmd: m
            .get_many::<String>(ARG_PREVIEW_CMD)
            .map(|values| values.cloned().collect()),
    }
}

/// Collects every requested field — `-f/--field` values and the `-i`/`-u`/`-p`/
/// `-c` aliases alike — in command-line order, lowercased and deduplicated.
fn collect_fields(m: &ArgMatches) -> Vec<String> {
    let mut ordered: Vec<(usize, String)> = Vec::new();

    if let (Some(values), Some(indices)) =
        (m.get_many::<String>(ARG_FIELD), m.indices_of(ARG_FIELD))
    {
        ordered.extend(indices.zip(values.map(|value| value.to_lowercase())));
    }

    for (id, field) in FIELD_ALIASES {
        if m.get_flag(id) {
            // A flag repeated on the command line yields several indices; the
            // first one is where the user "asked" for the field.
            if let Some(index) = m.indices_of(id).and_then(Iterator::min) {
                ordered.push((index, field.to_owned()));
            }
        }
    }

    ordered.sort_by_key(|(index, _)| *index);

    let mut fields: Vec<String> = Vec::with_capacity(ordered.len());
    for (_, field) in ordered {
        if !fields.contains(&field) {
            fields.push(field);
        }
    }
    fields
}

fn probe_args(m: &ArgMatches) -> ProbeArgs {
    ProbeArgs {
        host: m.get_one::<String>(ARG_HOST).cloned().unwrap_or_default(),
        verbose: m.get_flag(ARG_VERBOSE),
        plain: m.get_flag(ARG_PLAIN),
        json: m.get_flag(ARG_JSON),
        no_probes: m.get_flag(ARG_NO_PROBES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn parse_ok(args: &[&str]) -> Cli {
        try_parse_from(args).expect("expected the arguments to parse")
    }

    fn parse_err(args: &[&str]) -> ErrorKind {
        try_parse_from(args)
            .expect_err("expected the arguments to be rejected")
            .kind()
    }

    fn pick(args: &[&str]) -> PickArgs {
        match parse_ok(args).command {
            Subcommand::Pick(args) => args,
            Subcommand::Probe(_) => panic!("expected the pick subcommand"),
        }
    }

    fn probe(args: &[&str]) -> ProbeArgs {
        match parse_ok(args).command {
            Subcommand::Probe(args) => args,
            Subcommand::Pick(_) => panic!("expected the probe subcommand"),
        }
    }

    #[test]
    fn command_definition_is_valid() {
        command().debug_assert();
    }

    #[test]
    fn no_subcommand_shows_help() {
        assert_eq!(
            parse_err(&["kame"]),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        // Global options alone are still not an invocation.
        assert_eq!(
            parse_err(&["kame", "--color", "never"]),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn version_and_help_are_reported() {
        assert_eq!(parse_err(&["kame", "-V"]), ErrorKind::DisplayVersion);
        assert_eq!(parse_err(&["kame", "--help"]), ErrorKind::DisplayHelp);
        assert_eq!(parse_err(&["kame", "pick", "-h"]), ErrorKind::DisplayHelp);
    }

    #[test]
    fn color_defaults_to_auto_and_is_global() {
        assert_eq!(parse_ok(&["kame", "pick"]).color, ColorChoice::Auto);
        assert_eq!(
            parse_ok(&["kame", "--color", "never", "pick"]).color,
            ColorChoice::Never
        );
        assert_eq!(
            parse_ok(&["kame", "pick", "--color", "always"]).color,
            ColorChoice::Always
        );
        assert_eq!(
            parse_ok(&["kame", "probe", "--color", "never", "host"]).color,
            ColorChoice::Never
        );
        assert_eq!(
            parse_err(&["kame", "--color", "sometimes", "pick"]),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn pick_alias_p_works() {
        assert_eq!(pick(&["kame", "p", "prod"]).query.as_deref(), Some("prod"));
    }

    #[test]
    fn pick_defaults_are_empty() {
        assert_eq!(pick(&["kame", "pick"]), PickArgs::default());
    }

    #[test]
    fn field_flags_keep_command_line_order() {
        assert_eq!(
            pick(&["kame", "pick", "-u", "-i"]).fields,
            ["user", "hostname"]
        );
        assert_eq!(
            pick(&["kame", "pick", "-i", "-u"]).fields,
            ["hostname", "user"]
        );
        assert_eq!(
            pick(&["kame", "pick", "-p", "-f", "identityfile", "-c"]).fields,
            ["port", "identityfile", "controlpath"]
        );
        // Combined shorts keep their relative order too.
        assert_eq!(
            pick(&["kame", "pick", "-cu"]).fields,
            ["controlpath", "user"]
        );
    }

    #[test]
    fn field_names_are_lowercased_and_deduplicated() {
        assert_eq!(
            pick(&["kame", "pick", "-f", "HostName"]).fields,
            ["hostname"]
        );
        assert_eq!(
            pick(&["kame", "pick", "-i", "-f", "hostname"]).fields,
            ["hostname"]
        );
        assert_eq!(
            pick(&["kame", "pick", "-f", "user", "-f", "USER", "-i"]).fields,
            ["user", "hostname"]
        );
    }

    #[test]
    fn literal_implies_hostname_unless_a_field_is_given() {
        let args = pick(&["kame", "pick", "-L", "web"]);
        assert!(args.literal);
        assert_eq!(args.query.as_deref(), Some("web"));
        assert_eq!(args.fields, ["hostname"]);

        assert_eq!(pick(&["kame", "pick", "-L", "-u", "web"]).fields, ["user"]);
        assert_eq!(
            pick(&["kame", "pick", "-L", "-f", "identityfile", "web-*"]).fields,
            ["identityfile"]
        );
    }

    #[test]
    fn literal_requires_a_query() {
        assert_eq!(
            parse_err(&["kame", "pick", "-L"]),
            ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn preview_cmd_swallows_everything_after_it() {
        let args = pick(&[
            "kame",
            "pick",
            "prod",
            "--preview-cmd",
            "cowsay",
            "-s",
            "{}",
        ]);
        assert_eq!(args.query.as_deref(), Some("prod"));
        assert_eq!(
            args.preview_cmd.as_deref(),
            Some(["cowsay", "-s", "{}"].map(str::to_owned).as_slice())
        );

        let args = pick(&["kame", "pick", "--preview-cmd", "ssh", "-G", "{}"]);
        assert_eq!(
            args.preview_cmd.as_deref(),
            Some(["ssh", "-G", "{}"].map(str::to_owned).as_slice())
        );
    }

    #[test]
    fn preview_cmd_needs_at_least_one_argument() {
        assert_eq!(
            parse_err(&["kame", "pick", "--preview-cmd"]),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn pick_remaining_flags() {
        let args = pick(&["kame", "pick", "-m", "--json", "-F", "/tmp/cfg"]);
        assert!(args.multi);
        assert!(args.json);
        assert_eq!(args.config, Some(PathBuf::from("/tmp/cfg")));
    }

    #[test]
    fn probe_requires_a_host() {
        assert_eq!(
            parse_err(&["kame", "probe"]),
            ErrorKind::MissingRequiredArgument
        );
        assert_eq!(probe(&["kame", "probe", "prod-server"]).host, "prod-server");
    }

    #[test]
    fn probe_flags() {
        let args = probe(&["kame", "probe", "-v", "-N", "--json", "host"]);
        assert!(args.verbose);
        assert!(args.no_probes);
        assert!(args.json);
        assert!(!args.plain);
    }

    #[test]
    fn probe_rejects_json_with_plain() {
        assert_eq!(
            parse_err(&["kame", "probe", "--json", "--plain", "host"]),
            ErrorKind::ArgumentConflict
        );
    }

    #[test]
    fn probe_has_no_pick_flags() {
        assert_eq!(
            parse_err(&["kame", "probe", "--multi", "host"]),
            ErrorKind::UnknownArgument
        );
    }

    /// `pick --help`, with the ANSI escapes kept.
    fn pick_help_ansi() -> String {
        command()
            .find_subcommand_mut(CMD_PICK)
            .expect("the pick subcommand exists")
            .render_long_help()
            .ansi()
            .to_string()
    }

    /// `pick --help` as it comes out when color is off. `Display for StyledStr`
    /// strips escapes exactly like `anstream` does when printing.
    fn pick_help_plain() -> String {
        command()
            .find_subcommand_mut(CMD_PICK)
            .expect("the pick subcommand exists")
            .render_long_help()
            .to_string()
    }

    /// Width of the command column, mirroring [`pick_examples`].
    fn example_command_width() -> usize {
        PICK_EXAMPLES
            .iter()
            .map(|(command, _)| command.chars().count())
            .max()
            .expect("there is at least one example")
    }

    #[test]
    fn examples_use_the_same_styles_as_the_rest_of_the_help() {
        let styles = turtle_styles();
        let ansi = pick_help_ansi();

        let header = styles.get_header();
        assert!(
            ansi.contains(&format!("{header}Examples:{header:#}")),
            "the Examples heading should use the same style as Options:/Arguments:"
        );

        let literal = styles.get_literal();
        assert!(
            ansi.contains(&format!("{literal}ssh $(kame pick){literal:#}")),
            "example commands should use the same style as flag names"
        );
    }

    #[test]
    fn examples_strip_cleanly_when_color_is_off() {
        let plain = pick_help_plain();
        assert!(
            !plain.contains('\u{1b}'),
            "no escape should survive into uncolored help"
        );
        assert!(plain.contains("Examples:"));
    }

    #[test]
    fn example_descriptions_share_a_column() {
        let plain = pick_help_plain();
        let width = example_command_width();

        for (command, description) in PICK_EXAMPLES {
            let mut lines = description.lines();
            let first = lines.next().expect("a description is never empty");
            let padding = width - command.chars().count();
            let row = format!(
                "{EXAMPLE_INDENT}{command}{:padding$}{EXAMPLE_GAP}{first}",
                ""
            );
            assert!(
                plain.lines().any(|line| line == row),
                "missing example row: {row:?}"
            );

            // Wrapped descriptions hang under the first line, not under the command.
            let indent = EXAMPLE_INDENT.len() + width + EXAMPLE_GAP.len();
            for rest in lines {
                let row = format!("{:indent$}{rest}", "");
                assert!(
                    plain.lines().any(|line| line == row),
                    "misaligned continuation line: {row:?}"
                );
            }
        }
    }

    #[test]
    fn examples_only_appear_under_pick() {
        assert!(!pick_help_plain().contains("{n}"), "{{n}} is a clap token");
        let probe_help = command()
            .find_subcommand_mut(CMD_PROBE)
            .expect("the probe subcommand exists")
            .render_long_help()
            .to_string();
        assert!(!probe_help.contains("Examples:"));
    }
}
