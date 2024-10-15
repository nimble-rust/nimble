# 🎮 Nimble Client Logic

[![Crates.io](https://img.shields.io/crates/v/nimble-client-logic)](https://crates.io/crates/nimble-client-logic)
[![Documentation](https://docs.rs/nimble-client-logic/badge.svg)](https://docs.rs/nimble-client-logic)

`nimble-client-logic` is a robust Rust crate designed to manage client-side logic for multiplayer game
sessions using the Nimble protocol messages. It facilitates seamless communication between the client and host, 
ensuring synchronized game states and smooth gameplay experiences.

## 🚀 Features

- **🔗 Connection Management**: Establish and manage connections with the host, ensuring protocol compatibility 
 and handling connection states.
- **🕹️ Game State Handling**: Download and maintain the complete game state from the host, ensuring consistency across
 all clients.
- **👥 Participant Management**: Dynamically add and remove players from the game session.
- **⚡ Step Prediction & Reconciliation**: Send predicted player steps to the host and reconcile them with 
 authoritative steps received, ensuring responsive gameplay.
- **📦 Blob Streaming**: Efficiently handle large game state transfers using blob streaming.
- **📈 Metrics Tracking**: Monitor server buffer delta ticks to optimize performance and synchronization.

## 📦 Installation

Add `nimble-client-logic` to your project's `Cargo.toml`:

```toml
[dependencies]
nimble-client-logic = "0.0.14-dev"
```
