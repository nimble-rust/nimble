# 🎮 Nimble Assent

**Nimble Assent** is a Rust library for deterministic simulation of game logic based on player input (called **steps**). 
It ensures that all participants in a multiplayer game receive and process the same input in the same order, leading to identical results. 
This is crucial in real-time multiplayer games where fairness and synchronization across all clients are a must! 🚀

## 🤔 What is "Assent"?

The name **"Assent"** was chosen because it means **agreement** or **approval** — exactly what this library ensures between 
players in a networked game. Just like in a game, all players must be in **assent** (agreement) on the actions taken at each step, 
so everyone shares the same view of the game state.

In deterministic simulations:
- 🛠️ **All clients** need to process the same actions (steps) in the same order to maintain the same deterministic outcome.

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
