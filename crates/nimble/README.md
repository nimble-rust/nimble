# Nimble

[![Crates.io](https://img.shields.io/crates/v/nimble-rust)](https://crates.io/crates/nimble-rust)
[![Documentation](https://docs.rs/nimble-rust/badge.svg)](https://docs.rs/nimble-rust)

Welcome to the `nimble-rust` crate! This combined crate brings together the core components of the
Nimble ecosystem for building deterministic networked game simulations.

Nimble intentionally has no built in support for authentication, encryption, verify datagrams or provide rate limiting. The reason is that most game platform and relay service already provide that, e.g. [Steam Datagram Relay](https://partner.steamgames.com/doc/features/multiplayer/steamdatagramrelay).

## 🎮 What’s Inside?

- [**nimble-client**](https://crates.io/crates/nimble-client): The nimble client for receiving game state updates, sending steps and receiving authoritative steps.
- [**nimble-host**](https://crates.io/crates/nimble-host): The nimble host for authoritative game state management and authoritative steps.

## Recommended Crates

### Transport layers

If you are on a transport layer that is lacking in hash-verification or do not provide challenge-response, or you just need it for testing purposes, you can use:

- [**datagram-connections**](https://crates.io/crates/datagram-connections): Challenge response connections. Can be good for testing locally over UDP or similar.

- [**connection-layer**](https://crates.io/crates/connection-layer): Verify hash on each datagram. Can be good if you have a transport (relay service), but that transport does not have a connection-layer or that connection-layer may may cache or reuse previous connections.

### Internet Simulator

- [**hazy-transport**](https://crates.io/crates/hazy-transport): It is strongly recommended that you test your game
  using a internet simulator. This is a basic, but very useful one. Supports varying latency, packet drops and reordering.

## 📦 Installation

To include nimble-rust in your project, add the following to your Cargo.toml:

```toml
[dependencies]
nimble-rust = "0.0.15-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
