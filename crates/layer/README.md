# Nimble Layer

Nimble Layer is a Rust crate that provides a networking layer for handling ordered datagrams with
built-in latency measurement and metrics. It is designed to facilitate reliable communication by ensuring the order of
datagrams and tracking performance metrics such as latency and datagram drops.

## Features

- Ordered Datagram Handling: Ensures that datagrams are processed in the correct order
 (discards out of order and duplicated datagrams).
- Latency Measurement: Tracks and aggregates latency metrics to monitor network performance.
- Datagram Drop Tracking: Monitors and records dropped datagrams to help identify network issues.

## 📦 Installation

Add nimble-layer to your Cargo.toml:

```toml
nimble-layer = "0.0.14-dev"
```
