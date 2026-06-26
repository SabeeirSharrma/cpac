---
title: Installation
description: How to install CPAC.
order: 2
---

# Installation

CPAC runs on any Arch-based Linux distribution, including Arch, EndeavourOS, Garuda, CachyOS, and Manjaro.

To install CPAC, you can build from source (recommended) or alternatively you can install it from the AUR via yay/paru.

## Quick Install Script (Recommended)

For a quick build and install, run:

```bash
$ curl -sL https://thecinderproject.qd.je/install.sh -o install.sh
$ less install.sh   # review the script we provide you
$ bash install.sh
```

This script will install **Rust+Cargo** and `cpac`, since `cpac` is dependent on **Rust+Cargo** for building from source.

## Building from source

To build from source you must have **Rust** installed on your system, to do this you can run:

```bash
$ sudo pacman -S rustup 
```

refresh/restart your terminal and run:

```bash
$ rustup install stable
```

Then clone and build:

```bash
$ git clone https://github.com/sabeeirsharrma/cpac.git
$ cd cpac
$ cargo build
$ cargo install --path .
```

This installs the CLI binary as cpac in Cargo's bin directory, usually `~/.cargo/bin`.
Next, to make it available system wide you must add this to ~/.bashrc or ~/.zshrc etc.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

This will add Cargo's bin directory (where cpac is installed) to your path.
Now you can refresh/restart your terminal and run `cpac` to test.

## Installing from AUR (Using yay/paru)

>This will install cpac from the AUR but **updates to the AUR version may lag behind by up to 24 hours**.

To directly install cpac from the AUR, run:

```bash
$ yay -S cpac
```

or if you use paru

```bash
$ paru -S cpac
```

> **Note:** CPAC isn't installed yet at this point, so its own trust analysis isn't
> available to evaluate this install -- this is the one bootstrapping exception to
> CPAC's usual AUR-last, trust-checked install flow. Once CPAC is installed via any
> method, all future package installs go through its normal resolution and trust scoring.

## Verify Installation

After installing, run:

```bash
cpac --help
```

If your shell prints `cpac: command not found`, Cargo's bin directory is not on your `PATH`. Add it for the current terminal session:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

To make that permanent, add the same line to your shell config (`~/.bashrc`, `~/.zshrc`, etc.) and restart your terminal.
