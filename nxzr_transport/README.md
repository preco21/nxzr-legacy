# nxzr_transport

> 니 스위치 쩔드라

## Tracing & Logging

This module leverages [tracing](https://github.com/tokio-rs/tracing) crate in order to record all the events that can be dispatched across the transport life-cycles.

To catch up those information, you may want to use [tracing_subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) crate from the top-level end.