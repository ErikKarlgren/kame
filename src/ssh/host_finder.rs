// SPDX-FileCopyrightText: 2026 Erik Karlgren Domercq
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{collections::HashSet, path::PathBuf};

use anyhow::Result;
use tokio::{
    fs::File,
    io::{AsyncBufRead, AsyncBufReadExt, BufReader},
};

/// Parse hosts from a single file
pub async fn parse_hosts(path: PathBuf) -> Result<Vec<String>> {
    let mut hosts: HashSet<String> = HashSet::new();
    let file = File::open(&path).await?;
    extract_hosts(&mut hosts, BufReader::new(file)).await?;
    let mut hosts: Vec<_> = hosts.into_iter().collect();
    hosts.sort();
    Ok(hosts)
}

/// Reads `reader` line by line and adds every host alias it declares to
/// `hosts`. Any buffered async source will do, which keeps the parsing rules
/// testable without touching the file system.
async fn extract_hosts(
    hosts: &mut HashSet<String>,
    reader: impl AsyncBufRead + Unpin,
) -> Result<(), anyhow::Error> {
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        let mut words = line.split_whitespace();
        if words.next().is_some_and(|w| w.eq_ignore_ascii_case("host")) {
            let mut equal_sign_seen = false;
            for host in words {
                if host.starts_with('!') {
                    continue; // Ignore empty and negated hosts
                }
                if host.starts_with('#') {
                    break; // Skip comments
                }
                // The only glob patterns allowed for hosts
                if !host.contains('*') && !host.contains('?') {
                    let host = if !equal_sign_seen && host.starts_with('=') {
                        equal_sign_seen = true;
                        &host[1..]
                    } else {
                        host
                    };
                    hosts.insert(host.to_owned());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every expectation below follows ssh_config(5) — the "DESCRIPTION",
    // "Host" and "PATTERNS" sections — and was cross-checked against
    // `ssh -F <config> -G <host>` with OpenSSH 10.3. The tests describe what
    // the format allows, not what `extract_hosts` currently does.

    /// A configuration in the shape a real one takes, `Include`s and all.
    const REALISTIC_CONFIG: &str = "\
# Personal hosts.
Include ~/.ssh/config.d/*.conf
Include work/*.conf

Host alpha
    HostName alpha.example.com
    User erik

Host bravo charlie
    Port 2222

Include extra-hosts

Match host delta
    ForwardAgent yes

Host delta
    HostName 10.0.0.4
";

    /// Runs [`extract_hosts`] over `content`. A `&[u8]` is an
    /// [`AsyncBufRead`], so no file system is involved.
    fn extract_from(content: &str, hosts: &mut HashSet<String>) {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("could not build a test runtime")
            .block_on(extract_hosts(hosts, content.as_bytes()))
            .expect("expected the config to be read");
    }

    /// The aliases [`extract_hosts`] finds in `content`.
    fn hosts_in(content: &str) -> HashSet<String> {
        let mut hosts = HashSet::new();
        extract_from(content, &mut hosts);
        hosts
    }

    fn set(aliases: &[&str]) -> HashSet<String> {
        aliases.iter().map(|alias| (*alias).to_owned()).collect()
    }

    #[test]
    fn every_pattern_on_a_host_line_is_an_alias() {
        assert_eq!(hosts_in("Host alpha\n"), set(&["alpha"]));
        assert_eq!(
            hosts_in("Host alpha bravo charlie\n"),
            set(&["alpha", "bravo", "charlie"])
        );
        // "they should be separated by whitespace": any run of it will do.
        assert_eq!(
            hosts_in("Host\talpha \t  bravo\n"),
            set(&["alpha", "bravo"])
        );
    }

    #[test]
    fn the_rest_of_a_block_is_not_a_host() {
        let hosts = hosts_in(
            "\
Host alpha
    HostName alpha.example.com
    User erik
    Port 2222
    IdentityFile ~/.ssh/id_ed25519
",
        );
        assert_eq!(hosts, set(&["alpha"]));
    }

    /// `HostName` and friends start with `Host` but declare no alias at all.
    #[test]
    fn keywords_that_merely_start_with_host_are_ignored() {
        let hosts = hosts_in(
            "\
Hostname nope-1
HostName nope-2
HostKeyAlias nope-3
HostbasedAuthentication no
Hosts nope-4
Host_alpha nope-5
Hostalpha nope-6
Host real
",
        );
        assert_eq!(hosts, set(&["real"]));
        assert!(!hosts.contains("nope-1"));
        assert!(!hosts.contains("nope-2"));
    }

    /// A `Hostname` value looks exactly like an alias, so it is the most likely
    /// thing to leak into the picker if the keyword check ever loosens up.
    #[test]
    fn hostname_values_never_become_aliases() {
        let hosts = hosts_in(
            "\
Host real
    Hostname fake.example.com
    HostName fake2
",
        );
        assert_eq!(hosts, set(&["real"]));
        assert!(!hosts.contains("fake.example.com"));
        assert!(!hosts.contains("fake2"));
    }

    /// "note that keywords are case-insensitive and arguments are
    /// case-sensitive".
    #[test]
    fn the_keyword_is_case_insensitive() {
        assert_eq!(hosts_in("host alpha\n"), set(&["alpha"]));
        assert_eq!(hosts_in("HOST alpha\n"), set(&["alpha"]));
        assert_eq!(hosts_in("hOsT alpha\n"), set(&["alpha"]));
    }

    /// The other half of that sentence: an alias is reported exactly as it was
    /// written.
    #[test]
    fn arguments_keep_their_case_and_punctuation() {
        assert_eq!(
            hosts_in("Host Alpha-1 BRAVO beta_2 gamma.example.com 10.0.0.4 user@host\n"),
            set(&[
                "Alpha-1",
                "BRAVO",
                "beta_2",
                "gamma.example.com",
                "10.0.0.4",
                "user@host",
            ])
        );
    }

    #[test]
    fn indented_host_lines_are_recognized() {
        let hosts = hosts_in("    Host spaces\n\tHost tab\n \t Host both\n");
        assert_eq!(hosts, set(&["spaces", "tab", "both"]));
    }

    /// "Lines starting with '#' and empty lines are interpreted as comments."
    #[test]
    fn comment_lines_and_empty_lines_are_ignored() {
        let hosts = hosts_in(
            "\
# Host commented-out
#Host also-commented-out
   # Host indented-comment

\t
Host kept
",
        );
        assert_eq!(hosts, set(&["kept"]));
    }

    /// A `#` that opens a word ends the line, whatever came before it stands.
    #[test]
    fn a_trailing_comment_is_not_an_alias() {
        assert_eq!(hosts_in("Host alpha # the jump box\n"), set(&["alpha"]));
        assert_eq!(hosts_in("Host alpha #the-jump-box\n"), set(&["alpha"]));
        assert_eq!(hosts_in("Host # nothing but a comment\n"), set(&[]));
        assert_eq!(
            hosts_in("Host alpha # bravo\nHost charlie\n"),
            set(&["alpha", "charlie"])
        );
    }

    /// Only a `#` that opens a word starts a comment: `ssh bravo#1` resolves
    /// against `Host bravo#1`, so the alias is a real one.
    #[test]
    fn a_hash_inside_a_pattern_is_an_ordinary_character() {
        assert_eq!(
            hosts_in("Host alpha bravo#1 charlie\n"),
            set(&["alpha", "bravo#1", "charlie"])
        );
    }

    /// "Configuration options may be separated by whitespace or optional
    /// whitespace and exactly one '='".
    #[test]
    fn the_keyword_may_be_separated_by_an_equals_sign() {
        assert_eq!(hosts_in("Host=alpha\n"), set(&["alpha"]));
        assert_eq!(hosts_in("Host =alpha\n"), set(&["alpha"]));
        assert_eq!(hosts_in("Host= alpha\n"), set(&["alpha"]));
        assert_eq!(hosts_in("Host = alpha\n"), set(&["alpha"]));
        assert_eq!(
            hosts_in("Host   =   alpha bravo\n"),
            set(&["alpha", "bravo"])
        );
    }

    /// Only the separator `=` is special — "Allow only one '=' to be skipped"
    /// in `strdelim_internal`, and `argv_split` gives the character no meaning
    /// at all — so `ssh alpha=beta` resolves against `Host alpha=beta`.
    #[test]
    fn an_equals_sign_inside_a_pattern_is_an_ordinary_character() {
        assert_eq!(hosts_in("Host alpha=beta\n"), set(&["alpha=beta"]));
        assert_eq!(hosts_in("Host one two=three\n"), set(&["one", "two=three"]));
        assert_eq!(hosts_in("Host=alpha=beta\n"), set(&["alpha=beta"]));
        assert_eq!(hosts_in("Host = alpha=beta\n"), set(&["alpha=beta"]));
    }

    /// "A pattern consists of zero or more non-whitespace characters, '*' (a
    /// wildcard that matches zero or more characters), or '?' (a wildcard that
    /// matches exactly one character)." A pattern with a wildcard in it is not
    /// a host anyone can connect to.
    #[test]
    fn wildcard_patterns_are_not_aliases() {
        assert_eq!(hosts_in("Host *\n"), set(&[]));
        assert_eq!(hosts_in("Host web* *.example.com a*b\n"), set(&[]));
        assert_eq!(hosts_in("Host web? 192.168.0.?\n"), set(&[]));
        // The rest of the line is unaffected by the patterns dropped from it.
        assert_eq!(
            hosts_in("Host web* alpha 192.168.0.? bravo\n"),
            set(&["alpha", "bravo"])
        );
    }

    /// `*` and `?` are the only wildcards the format has: brackets are
    /// ordinary characters, and `ssh srv[1-3]` matches `Host srv[1-3]`
    /// literally rather than `srv1`.
    #[test]
    fn brackets_are_not_wildcards() {
        assert_eq!(
            hosts_in("Host srv[1-3] [abc]x y]z\n"),
            set(&["srv[1-3]", "[abc]x", "y]z"])
        );
    }

    /// "A pattern entry may be negated by prefixing it with an exclamation
    /// mark ('!')." The negated entry itself is never an alias: a line with
    /// nothing but negations matches no host at all.
    #[test]
    fn a_negated_pattern_is_not_an_alias() {
        assert_eq!(hosts_in("Host !skipped\n"), set(&[]));
        assert_eq!(hosts_in("Host !skipped !also-skipped\n"), set(&[]));
    }

    /// Only a leading `!` negates — `negated = *arg == '!'` in `readconf.c` —
    /// so `ssh foo!bar` resolves against `Host foo!bar` and the alias is real.
    #[test]
    fn a_bang_inside_a_pattern_is_an_ordinary_character() {
        assert_eq!(hosts_in("Host alpha foo!bar\n"), set(&["alpha", "foo!bar"]));
    }

    /// A negation does not end the line: patterns after it still declare
    /// aliases. `ssh keep` resolves against `Host !skipped keep`.
    #[test]
    fn patterns_after_a_negation_are_still_aliases() {
        assert_eq!(hosts_in("Host !skipped keep\n"), set(&["keep"]));
        assert_eq!(
            hosts_in("Host alpha !skipped bravo\n"),
            set(&["alpha", "bravo"])
        );
    }

    /// "If a negated entry is matched, then the Host entry is ignored,
    /// regardless of whether any other patterns on the line match." A negation
    /// that matches nothing on its line therefore leaves the line alone.
    #[test]
    fn a_negation_leaves_the_patterns_it_does_not_match_alone() {
        assert_eq!(hosts_in("Host alpha !other\n"), set(&["alpha"]));
        assert_eq!(hosts_in("Host alpha !other-*\n"), set(&["alpha"]));
        assert_eq!(
            hosts_in("Host alpha bravo !other-?\n"),
            set(&["alpha", "bravo"])
        );
    }

    /// A pattern its own line negates is unreachable — `ssh self` skips the
    /// block below and connects with plain defaults — so `ssh_config(5)`
    /// declares no usable alias on either of these lines.
    ///
    /// Collecting them anyway is a deliberate divergence: deciding otherwise
    /// means matching every positive pattern against the line's negations,
    /// which needs a port of ssh's `*`/`?` matcher, and a line that cancels
    /// its own literals is self-defeating enough that it is not worth the
    /// code. Pins that choice so it stays a choice.
    #[test]
    fn a_pattern_cancelled_by_a_negation_is_collected_regardless() {
        assert_eq!(hosts_in("Host self !self\n"), set(&["self"]));
        assert_eq!(
            hosts_in("Host prod-a prod-b !prod-*\n"),
            set(&["prod-a", "prod-b"])
        );
    }

    /// "Arguments may optionally be enclosed in double quotes (\") in order to
    /// represent arguments containing spaces."
    ///
    /// A quote is not a delimiter but a state toggle inside the token
    /// (`argv_split` in `misc.c`): a token still ends at the first *unquoted*
    /// space, and the quote characters are dropped from the result. So a
    /// partly quoted word glues back into one pattern, and a quoted space
    /// keeps a pattern in one piece. `'` behaves exactly like `"`.
    #[test]
    fn quoted_arguments_are_unquoted() {
        assert_eq!(hosts_in(r#"Host "alpha""#), set(&["alpha"]));
        assert_eq!(
            hosts_in(r#"Host alpha "bravo" charlie"#),
            set(&["alpha", "bravo", "charlie"])
        );

        // Quotes that open and close mid-word leave the word intact.
        assert_eq!(hosts_in(r#"Host chi"na""#), set(&["china"]));
        assert_eq!(hosts_in(r"Host chi'na'"), set(&["china"]));

        // One pattern, spaces and all.
        assert_eq!(hosts_in(r#"Host "hotel india""#), set(&["hotel india"]));
        assert_eq!(hosts_in(r#"Host "india i"ndia"#), set(&["india india"]));
        assert_eq!(hosts_in(r#"Host chi"na china""#), set(&["china china"]));
        assert_eq!(hosts_in(r#"host pi"zza pizz"a"#), set(&["pizza pizza"]));
    }

    #[test]
    fn repeated_aliases_are_collected_once() {
        let hosts = hosts_in("Host alpha alpha\nHost alpha bravo\nHost alpha\n");
        assert_eq!(hosts, set(&["alpha", "bravo"]));
    }

    #[test]
    fn aliases_are_added_to_the_hosts_already_found() {
        let mut hosts = set(&["preexisting", "alpha"]);
        extract_from("Host alpha bravo\n", &mut hosts);
        assert_eq!(hosts, set(&["preexisting", "alpha", "bravo"]));
    }

    #[test]
    fn a_host_keyword_without_arguments_yields_nothing() {
        assert_eq!(hosts_in("Host\n"), set(&[]));
        assert_eq!(hosts_in("Host   \nHost after\n"), set(&["after"]));
    }

    #[test]
    fn empty_input_yields_no_hosts() {
        assert_eq!(hosts_in(""), set(&[]));
        assert_eq!(hosts_in("\n\n   \n\t\n"), set(&[]));
    }

    #[test]
    fn input_without_a_trailing_newline_is_read_whole() {
        assert_eq!(hosts_in("Host alpha bravo"), set(&["alpha", "bravo"]));
    }

    #[test]
    fn aliases_are_collected_from_a_realistic_config() {
        assert_eq!(
            hosts_in(REALISTIC_CONFIG),
            set(&["alpha", "bravo", "charlie", "delta"])
        );
    }

    #[test]
    fn every_alias_is_usable_as_an_ssh_target() {
        let hosts = hosts_in(REALISTIC_CONFIG);
        assert!(!hosts.is_empty());
        for host in &hosts {
            assert!(!host.is_empty(), "an empty alias was collected");
            assert!(
                !host.chars().any(char::is_whitespace),
                "{host:?} contains whitespace"
            );
            assert!(
                !host.contains('*') && !host.contains('?'),
                "{host:?} is a wildcard pattern, not a host"
            );
            assert!(!host.starts_with('!'), "{host:?} is a negated pattern");
            assert!(!host.starts_with('#'), "{host:?} is part of a comment");
        }
    }
}
