# 🎮 nimble-sample-step

[![Crates.io](https://img.shields.io/crates/v/nimble-sample-step)](https://crates.io/crates/nimble-sample-step)
[![Documentation](https://docs.rs/nimble-sample-step/badge.svg)](https://docs.rs/nimble-sample-step)

This crate provides an example `Step` implementation to use in tests or for other similar purposes.
It's built with flexibility and simplicity in mind, giving you basic game-pad-like
inputs such as moving left, right, jumping, or doing nothing.

## ✨ Features

- **🚶 MoveLeft(i16)**: Move your character to the left with a specified amount.
- **🏃 MoveRight(i16)**: Move your character to the right.
- **🦘 Jump**: Make your character jump.
- **⛔ Nothing**: No game-pad input.
- **🗃️ SampleState**: A simple state structure to simulate stateful deserialization with a buffer of data.

These actions are ready for you to plug into your test cases!

## 📦 Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
nimble-sample-step = "0.0.17-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
