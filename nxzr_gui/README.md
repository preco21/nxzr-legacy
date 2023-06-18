# nxzr_gui

> NXZR project: Host GUI application

## Prerequisite

Please take a look https://tauri.app/v1/guides/getting-started/prerequisites for prerequisites.

## Installation

```shell
yarn
```

## Providing external resources

The program requires external binaries of a ready-to-use WSL kernel, distribution and `nxzr_server` binary.

You can find and download each WSL-related pre-built resource from GitHub Releases here: https://github.com/preco21/nxzr/releases/tag/nxzr-helper

- `nxzr-bzImage`
- `nxzr-agent.tar`

And you will need to manually build the `nxzr_server` from this repository by cloning it. It's located at `nxzr_server` folder from the root.

After building, the executable binary can be obtained from `target/` folder.

- `nxzr_server`

## Building

You can build this project by choosing either of following options:

### Windows

1. Install Microsoft toolchains + Microsoft Visual Studio from [here](https://visualstudio.microsoft.com/visual-cpp-build-tools/), and make sure the "Visual Studio C++ Build tools" is installed.
2. Install [Rustup](https://www.rust-lang.org/tools/install) for Windows.

### WSL2

This is my prefer way to build projects as Windows as a programming environment is not really so good.

Although, I still want to take advantage of MSVC, empolyed the [msvc-wsl-rust](https://github.com/strickczq/msvc-wsl-rust) to the project.

Assuming you have installed Rustup for WSL2 already,

First, install Microsoft toolchains + Microsoft Visual Studio from [here](https://visualstudio.microsoft.com/visual-cpp-build-tools/), and make sure the "Visual Studio C++ Build tools" is ready.

Run following commands:

```shell
rustup target add x86_64-pc-windows-msvc
rustup target add i686-pc-windows-msvc
```

Install [msvc-wsl-rust](https://github.com/strickczq/msvc-wsl-rust) and make an edit for the config in `msvc-linker/config.sh` if necessary.

```shell
git clone https://github.com/strickczq/msvc-wsl-rust.git msvc-linker
chmod a+x msvc-linker/*.sh
```

## Building a new distro image for the user's WSL

Please refer to https://github.com/preco21/nxzr-helper
