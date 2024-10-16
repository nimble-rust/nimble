# 🎮 nimble-sample-step

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

## 🔧 Usage

Include the `nimble-sample-step` crate in your `Cargo.toml` to start using it for your test scenarios.

```toml
[dependencies]
nimble-sample-step = "0.0.14-dev"
```
