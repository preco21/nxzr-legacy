# NXZR

> 니 스위치 쩔드라

## Project structure

- [nxzr_core](./nxzr_core/): The NXZR core modules
- [nxzr_device](./nxzr_device/): Device and transport modules for working with unix sockets
- [nxzr_server](./nxzr_server/): Server daemon application that runs on the guest side
- [nxzr_gui](./nxzr_gui/): Host GUI application that interacts with `nxzr_server`

## Troubleshooting

### `.cargo/config.toml` is not respected

At the time of writing, Rust Workspaces feature doesn't respect per-workspace `.cargo/config.toml`. So, you will want to directly move into each crate instead to run build:

```shell
cd nxzr_server && cargo build
```

Tracking issues:
- https://github.com/rust-lang/cargo/issues/7004

### Caveats when using VSCode Workspaces feature

You may want to open each workspace per editor since rust-analyzer does not work well with VSCode Workspaces feature.

Due to platform mismatches, some dependencies are marked as "optional" for some crates.

Because of the this, opening the entire project as a whole in VSCode might fail to run `cargo check` internally.

In order to work with cross-compiling projects, you will want to use [`cross`](https://github.com/cross-rs/cross) instead.

Make sure the [Docker daemon](https://www.docker.com/) is running on your system which allows you to cross-compile projects.

Then, open a project directly to work with `cross`:

```shell
code nxzr_device
```

Tracking issues:
- https://github.com/rust-lang/rust-analyzer/issues/11900
- https://github.com/rust-lang/rust-analyzer/issues/11268#issuecomment-1012659059

### `cargo check` fails

Make sure to install required components:

```shell
rustup component add rustfmt
rustup component add clippy
```

This project includes multi compilation targets, so it may fail to build when
commands like `cargo check` is executed on the project root.

Please refer to "Caveats when using VSCode Workspaces feature" section above if you want to cross-compile projects.
