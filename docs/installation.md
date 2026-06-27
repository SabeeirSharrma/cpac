---
title: Installation
description: How to install CPAC.
order: 2
---

# Installation

CPAC runs on any Arch-based Linux distribution, including Arch, EndeavourOS, Garuda, CachyOS, and Manjaro.

## Quick Install (Recommended)

One command to build and install cpac from source:

```bash
curl -sSf https://thecinderproject.qd.je/cpac/install.sh | bash
```

This script will:
- Detect if Rust is already installed on your system
- If not, install Rust temporarily just for building
- Clone and build cpac from source
- Install the binary to `/usr/local/bin`
- If Rust was not present before, **automatically remove it** after installation

No dependencies are left behind. The script is fully transparent — you can review it before running:

```bash
curl -sSf https://thecinderproject.qd.je/cpac/install.sh -o install.sh
less install.sh
bash install.sh
```

## Building from Source

If you prefer to build manually:

```bash
git clone https://github.com/SabeeirSharrma/cpac.git
cd cpac
cargo build --release
sudo cp target/release/cpac /usr/local/bin/cpac
```

## Installing from AUR

> Updates to the AUR version may lag behind by up to 24 hours.

```bash
yay -S cpac
```

or with paru:

```bash
paru -S cpac
```

> **Note:** CPAC isn't installed yet at this point, so its own trust analysis isn't
> available to evaluate this install — this is the one bootstrapping exception to
> CPAC's usual AUR-last, trust-checked install flow. Once CPAC is installed via any
> method, all future package installs go through its normal resolution and trust scoring.

## Verify Installation

After installing, run:

```bash
cpac --help
```

If your shell prints `cpac: command not found`, add `/usr/local/bin` to your PATH:

```bash
export PATH="/usr/local/bin:$PATH"
```

To make that permanent, add the same line to your shell config (`~/.bashrc`, `~/.zshrc`, etc.) and restart your terminal.
