# 🎮 Nimble Assent

**Nimble Assent** is a Rust library for deterministic simulation of game logic based on player inputs (called **steps**).
It ensures that all participants in a multiplayer game process the same input in the same order, leading to identical results.

## 🤔 What is "Assent"?

The name **"Assent"** was chosen because it means **agreement**, in this context all clients agree on the same deterministic simulation.

## 📚 Overview

Nimble Assent's primary structure is the `Assent` struct, which manages the deterministic application of **player steps** over game ticks. It provides:

- **Step queueing** based on tick IDs ⏳
- **Synchronized step processing** to keep all participants on the same page 👥
- Limiting the number of ticks processed per update to prevent overloads 🛑

📦 Installation

Add this to your Cargo.toml:

```toml
[dependencies]
nimble-assent = "0.0.14-dev"
```
