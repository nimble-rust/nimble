# Nimble Layer

[![Crates.io](https://img.shields.io/crates/v/nimble-layer)](https://crates.io/crates/nimble-layer)
[![Documentation](https://docs.rs/nimble-layer/badge.svg)](https://docs.rs/nimble-layer)

Nimble Layer is a Rust crate that provides a networking layer for handling ordered datagrams with
built-in latency measurement and metrics. It is designed to facilitate reliable communication by
ensuring the order of datagrams and tracking performance metrics for datagram drops.

## ✨ Features

- Ordered Datagram Handling: Ensures that datagrams are processed in the correct order
 (discards out of order and duplicated datagrams).
- Datagram Drop Tracking: Monitors and records dropped datagrams to help identify network issues.

## 📦 Installation

Add nimble-layer to your Cargo.toml:

```toml
nimble-layer = "0.0.14-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
