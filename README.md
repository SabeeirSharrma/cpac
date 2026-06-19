# CPAC

Community Package Analysis Client: a package trust layer for Arch-based Linux distributions.

CPAC is an advisor, not a package manager replacement. It helps answer whether a package looks trustworthy before you install it.

## Week 1 Scope

Implemented commands:

```bash
cpac search <query>
cpac trust <package>
```

Stubbed commands:

```bash
cpac audit [package]
cpac install <package>
cpac remove <package>
cpac update
cpac aur <enable|disable>
```

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

Currently implemented commands:

```bash
cpac search <query>
cpac trust <package>
```

Commands that are available as placeholders but not implemented yet:

```bash
cpac audit [package]
cpac install <package>
cpac remove <package>
cpac update
cpac aur <enable|disable>
```

## Requirements

- Arch-based Linux distribution
- `pacman` available on `PATH`
- Network access for AUR search and AUR trust metadata

## Trust Scoring

The Week 1 trust algorithm uses metadata available from official repositories and the AUR:

- repository source
- package age
- maintainer status
- votes/popularity
- update recency
- out-of-date and orphan status

See [docs/trust-algorithm.md](docs/trust-algorithm.md) for details.
