# Nimble Sample Game

[![Crates.io](https://img.shields.io/crates/v/nimble-sample-game)](https://crates.io/crates/nimble-sample-game)
[![Documentation](https://docs.rs/nimble-sample-game/badge.svg)](https://docs.rs/nimble-sample-game)

nimble-sample-game provides a simple example of a deterministic game simulation and game state
for use in testing. It demonstrates how to handle authoritative and predicted game states,
 apply player input (steps), and integrate with the nimble crates for deterministic simulation.

## ✨ Features

- Supports both predicted and authoritative game state management.
- Example steps for player input such as moving left, right, and jumping.
- Serializes and deserializes the game state for network transmission.
- Provides callbacks for predicted (seer), authoritative (assent), and reconciliation (rectify) game loops.

## 📦 Installation

To use this crate, add the following to your Cargo.toml:

```toml
[dependencies]
nimble-sample-game = "0.0.15-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
