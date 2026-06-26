# CPAC

Community Package Analysis Client: a package trust layer for Arch-based Linux distributions.

CPAC is an advisor, not a package manager replacement. It helps answer whether a package looks trustworthy before you install it.

## Build

```bash
cargo build
```

## Install Locally

From the repository root:

```bash
cargo install --path .
```

This installs the CLI binary as `cpac` in Cargo's bin directory, usually
`~/.cargo/bin`.

After installation, run:

```bash
cpac --help
```

If your shell prints `cpac: command not found`, Cargo's bin directory is not on
your `PATH`. Add it for the current terminal session:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

To make that permanent, add the same line to your shell config, such as
`~/.bashrc` or `~/.zshrc`, then restart your terminal or reload the config:

```bash
source ~/.bashrc
```

## Usage

Search official repositories and the AUR:

```bash
cpac search firefox
```

Show a trust report:

```bash
cpac trust firefox
```

All commands:

```bash
cpac search <query>              # search across all sources
cpac trust <package>             # full trust report
cpac audit [package]             # system-wide or per-package audit
cpac install <package>           # trust-gated install with PKGBUILD diffing
cpac remove <package>            # trust-gated removal
cpac update [--aur]              # refresh official + AUR databases
cpac diff <package>              # local PKGBUILD diff
cpac config show                 # show current configuration
cpac config set aur on|off       # enable/disable AUR
cpac config set consent ...      # set data sharing level
cpac config set cache ...        # set auto-clear interval
cpac config reset                # reset to defaults
cpac config path                 # show config file location
cpac clear-cache                 # manually clear cache
```

## First Run

On first launch, CPAC asks about crowdsourced data sharing (opt-in, anonymous). This can be changed anytime via `cpac config set consent`.

## Auto Cache Clearing

CPAC automatically clears its metadata cache on a configurable interval (daily/weekly/monthly, default monthly). Set it with `cpac config set cache`.

## Requirements

- Arch-based Linux distribution
- `pacman` available on `PATH`
- Network access for AUR search and AUR trust metadata

## Trust Scoring

The trust algorithm uses metadata available from official repositories and the AUR:

- repository source
- package age
- maintainer status
- votes/popularity
- update recency
- out-of-date and orphan status
- PKGBUILD diff results

See [docs/trust-algorithm.md](docs/trust-algorithm.md) for details.

## Made By

**Developer/Maintainer: [Sabeeir Sharrma](https://github.com/SabeeirSharrma)**

**Made under [The Cinder Project - Burn all the Blind Spots](https://thecinderproject.qd.je/)**
