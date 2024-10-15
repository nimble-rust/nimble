# 🏃‍♂️ nimble-steps

`nimble-steps` is a Rust library designed to manage a sequence of steps (moves, decisions) in a deterministic simulation.
Each step is associated with a unique `TickId`, ensuring steps are processed in the correct order.

## ✨ Features

- **Step Management**: Queue steps with associated `TickId` to ensure correct processing.
- **Iterator Support**: Iterate through steps with both standard and indexed iteration.
- **Flexible Step Handling**: Push, pop, and take steps from the queue with tick validation.
- **Error Handling**: Robust error handling for cases such as incorrect `TickId` order.

## 🚀 Getting Started

Add `nimble-steps` to your `Cargo.toml`:

```toml
[dependencies]
nimble-steps = "0.0.14-dev"
```
