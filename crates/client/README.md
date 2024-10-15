# Nimble Client 🕹️

[![Crates.io](https://img.shields.io/crates/v/nimble-client)](https://crates.io/crates/nimble-client)
[![Documentation](https://docs.rs/nimble-client/badge.svg)](https://docs.rs/nimble-client)

Welcome to **Nimble Client**, a robust and efficient Rust crate designed to handle networking tasks for multiplayer games. 🎮✨

## 📦 Overview

Nimble Client is a lightweight library that manages:

- **Downloading the Complete Game State** from a host
- **Adding and Removing Participants** in a game session by sending request messages to the host
- **Sending Predicted Inputs (Steps)** to the host for smoother gameplay
- **Receiving Authoritative Steps** from the host to ensure game state consistency

**Note:** Nimble Client **does not** handle game logic directly. Instead, it interfaces with your game logic through callbacks, ensuring a clean separation of concerns.

## 🌟 Features

- **Efficient Network Communication:** Handles sending and receiving data with optimized performance.
- **Participant Management:** Easily add or remove players from your game sessions.
- **Prediction and Reconciliation:** Sends predicted inputs and processes authoritative steps to maintain game state integrity.
- **Metrics and Logging:** Built-in network metrics and logging for monitoring and debugging.
- **Extensible Callbacks:** Integrate seamlessly with your game logic through customizable callbacks.

## 🚀 Getting Started

### 📋 Prerequisites

- 🦀 **Rust:** Ensure you have Rust installed. If not, download it from [rustup.rs](https://rustup.rs/). 

### 📦 Installation

Add `nimble-client` to your `Cargo.toml`:

```toml
[dependencies]
nimble-client = "0.0.14-dev"
```
