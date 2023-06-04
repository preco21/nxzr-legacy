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
