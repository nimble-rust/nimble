# Nimble-Seer

`nimble-seer` is a Rust library designed to handle predicted steps and provide callbacks for
deterministic simulations. It efficiently manages predicted game state steps and syncs them
 with authoritative steps, ensuring a smooth and predictable simulation experience.

## Features

- **Predicted Steps Queue**: Manages a queue of predicted steps, allowing for simulation without immediate authoritative feedback.
- **Callbacks**: Provides `SeerCallback` trait to handle actions before, during, and after each tick in the simulation.
- **Authoritative Step Integration**: Syncs predicted steps with authoritative steps, discarding outdated predictions.

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
nimble-seer = "0.0.14-dev"
```
