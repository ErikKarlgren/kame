<help>
An SSH toolkit

Usage: kame [OPTIONS] [COMMAND]

Options:
  -V, --version                  Print version info and exit
      --color <WHEN>             Coloring [possible values: auto, always, never]
  -h, --help                     Print help

Commands:
    pick, p     Fuzzy search and pick host from your ssh config. Output can be controlled with
                flags
    probe       Probe a host for detailed ssh info
</help>



<pickHelp>
Fuzzily pick an SSH alias from your SSH config and prints it. Uses "kame probe" as the default
preview.

Usage: kame pick [OPTIONS] [query]

Arguments:
  [query]                 Optional text to start the fuzzy query with

Options:
  -f, --field <FIELD>     Resolve the exact host (via `ssh -G`) and print the specified field.
                          If field has multiple values (e.g. identityfile), they are joined with
                          commas.
                          Can be called multiple times.
  -i, --hostname          Alias of `--field hostname`
  -u, --user              Alias of `--field user`
  -p, --port              Alias of `--field port`
  -c, --control-path      Alias of `--field controlpath`
  -m, --multi             Choose multiple hosts and print desired info for all of them in separate
                          lines
  -F, --config <FILE>     Search aliases in the given file instead of ~/.ssh/config
  -L, --literal           Skip the fuzzy search and pass [query] directly to `ssh -G`. Implies
                          `-i` by default unless another flag is used. Meant for scripting
      --json              Print output as json object (or array of objects with `-m`) instead of
                          plain text. If a field has multiple values, print them as a json array.
      --preview-cmd       Override the preview command. Anything after this flag will be treated as
                          an argument to it (e.g. `--preview-cmd ssh -G {}` will run `ssh -G {}`,
                          no need for quotes). `{}` is a placeholder for the SSH alias.

Examples:
  ssh $(kame pick)                      Pick a host and connect to it with SSH
  ssh -J proxy $(kame pick prod)        Pick a host and connect to it through a proxy. Starts fuzzy
                                        search with "prod"
  scp file $(kame pick):/tmp            Pick a host and copy a file to its /tmp dir
  curl https://$(kame pick -i)/v1/api   Pick a host and curl its hostname
  rm $(kame pick -m -c)                 Pick hosts and remove their control paths
  kame pick -m -i -u --json             Pick hosts and print their hostnames and users as a json
                                        array
  kame pick -L -f identityfile 'web-*'  Print the SSH key files used for web-* hosts (quote any
                                        args with '*' when run in a shell)
  kame pick --preview-cmd cowsay -s {}  Pick a host with a funny preview (calls `cowsay -s {}`)

</pickHelp>

<probeHelp>
Probe an SSH host and show config info with optional network health checks

Usage: kame probe [OPTIONS] [host]

Arguments
  [host]                  SSH host to probe and show info about. Works with aliases defined in ssh
                          config

Options:
  -v, --verbose           Show detailed diagnostic info: SSH banner, auth methods, negotiated
                          ciphers, TCP vs SSH handshake timing, ProxyJump chain
  -p, --plain             Plain text output, without colors or decorations. Incompatible with
                          `--json`
      --json              Print output as json object. Incompatible with `--plain`
  -N, --no-probes         Skip network probes, only show info parsed from `ssh -G`

</probeHelp>

<probeDefaultOutput>
$ kame probe prod-server
Hostname:       server-12345.cloud.com    (ssh -G)
User:           admin                     (ssh -G)
Port:           22                        (ssh -G)
Reachable:      🟢 yes                    (tcp probe ip/port)
SSH Latency:    213 ms                    (ssh probe ip/port)
Connected:      🟢 online                 (netstat2 crate)
TCP Latency:    145 ms                    ([verbose] tcp probe ip/port)
Control:        🟢 active                 ([verbose] ssh -o check <host>)
ProxyJump:      proxy1 -> proxy2          ([verbose] ssh -G)
Identity File:  ~/.ssh/id_rsa, ...        ([verbose] ssh -G)
Auth:           passkey, password, ...    ([verbose] ssh probe)
</probeDefaultOutput>

<probeJsonOutput>
$ kame probe --json prod-server
{"hostname": "server-12345.cloud.com", "user": "admin", "port": "22", "session": "online", "ssh_latency": "213 ms"}
</probeJsonOutput>
