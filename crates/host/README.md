# 🚀 Nimble Host
[![Crates.io](https://img.shields.io/crates/v/nimble-host)](https://crates.io/crates/nimble-host)
[![Documentation](https://docs.rs/nimble-host/badge.svg)](https://docs.rs/nimble-host)

Welcome to **Nimble Host**! 🕹️✨ The core server-side component of the Nimble multiplayer framework, designed 
to manage game sessions, handle client connections, and ensure smooth communication between clients and the host.

## 🌟 Features

- **🔗 Connection Management**: Easily create, manage, and destroy client connections.
- **🧠 Host Logic Integration**: Seamlessly integrates with `nimble_host_logic` to handle game state and client commands.
- **📦 Efficient Datagram Handling**: Processes incoming and outgoing datagrams with support for chunking.
- **🔄 Serialization**: Robust serialization and deserialization of commands using `flood_rs`.
- **🔒 Error Handling**: Detailed error management to handle connection issues and other host-related errors.

## 📦 Installation

Add `nimble-host` to your `Cargo.toml`:

```toml
[dependencies]
nimble-host = "0.0.14-dev"
```
