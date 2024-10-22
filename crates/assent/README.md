# 🎮 Nimble Assent

[![Crates.io](https://img.shields.io/crates/v/nimble-assent)](https://crates.io/crates/nimble-assent)
[![Documentation](https://docs.rs/nimble-assent/badge.svg)](https://docs.rs/nimble-assent)

**Nimble Assent** is a Rust library for deterministic simulation of game logic based on player inputs (called **steps**).
It ensures that all participants in a multiplayer game process the same input in the same order, leading to identical results.

## 🤔 What is "Assent"?

The name **"Assent"** was chosen because it means **agreement**, in this context that assent makes sure that the deterministic simulation is updated so all clients agree on the same deterministic simulation.

## ✨ Features

Nimble Assent's primary structure is the `Assent` struct, which manages the deterministic application of **authoritative player steps** over game ticks. It provides:

- **Step queueing** based on tick IDs
- **Synchronized step processing** to keep all participants on the same page
- Limiting the number of ticks processed per update to prevent overloads

## 📦 Installation

Add this to your Cargo.toml:

```toml
[dependencies]
nimble-assent = "0.0.14-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
