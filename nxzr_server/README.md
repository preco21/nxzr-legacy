# nxzr_server

> Server daemon application

## Prerequisite

- [Protoc](https://github.com/protocolbuffers/protobuf): Make sure to install the appropriate protoc binary for your platform.

## Installation

```shell
# From Alpine Linux
apk add git curl build-base pkgconfig gcompat dbus-glib-dev bluez protoc

# From Ubuntu
apt install libdbus-1-dev pkg-config
```

## Building

```shell
cargo build
```

## Topics

### Pulling out a binary out of WSL

When you are building a binary from WSL environment, you will want to pull the binary out in order to put it in the GUI application.

You can use [wslu](https://wslutiliti.es/) to make things easy for working between Windows and WSL such as moving files, etc...

Assuming you are on Ubuntu, to install `wslu`:

```shell
# From Alpine Linux
sudo apk add wslu

# From Ubuntu
sudo add-apt-repository ppa:wslutilities/wslu
sudo apt update
sudo apt install wslu
```

After building, you can use this command to pull the binary out of the `target` folder, moves to the Windows.

```shell
mkdir -p "$(wslpath $(wslvar USERPROFILE))/.nxzr-out"
cp ../target/release/nxzr_server "$(wslpath $(wslvar USERPROFILE))/.nxzr-out"
```
