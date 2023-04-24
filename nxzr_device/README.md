# nxzr_device

> NXZR project: A collection of tools, actual transport layer and connection helpers

## Tracing & Logging

This module leverages [tracing](https://github.com/tokio-rs/tracing) crate in order to record all the events that can be dispatched across the modules' life-cycle.

To catch up those information, you may want to use [tracing_subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) crate from the top-level end.
