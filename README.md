# Nimble

Nimble is a Rust library designed for sending and receiving game state and input efficiently.
It provides both client and host crates, making it ideal for multiplayer game development.
The library emphasizes deterministic simulation through its dedicated packages and robust datagram handling.

## Overview

Nimble includes the following key components:

- **Client and Host Crates**: Easy-to-use interfaces for both client and server-side implementation.
- **Packages**:
  - **assent**: Handles the agreement on game state across clients.
  - **seer**: Provides a mechanism for observing game state changes.
  - **rectify**: Corrects discrepancies in game state for deterministic simulation.
- **Datagram Handling**:
  - **Datagram Traits**: Define standard behaviors for sending and receiving datagrams.
  - **Datagram Builder**: Facilitates building and sending datagrams efficiently.
  - **Ordered Datagram**: Ensures datagrams are processed in the order they are sent.
- **Steps**:
  - **Steps Collection**: Manages game steps for simulation consistency.

## Features

- **Deterministic Simulation**: Leverage the `assent`, `seer`, and `rectify` packages to ensure all clients experience the same game state.
- **Efficient Data Transmission**: Utilize datagram traits and builders for optimal network performance.
- **Easy Integration**: Designed for seamless integration into your game projects.

## Getting Started

### Installation

Add Nimble to your `Cargo.toml`:

```toml
[dependencies]
nimble-rust = "0.0.1"
```
