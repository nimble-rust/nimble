# 🎮 nimble-participant

[![Crates.io](https://img.shields.io/crates/v/nimble-participant)](https://crates.io/crates/nimble-participant)
[![Documentation](https://docs.rs/nimble-participant/badge.svg)](https://docs.rs/nimble-participant)

`nimble-participant` is a Rust library designed to represent participants in a deterministic simulation. It provides a `ParticipantId` type with built-in serialization, deserialization, and formatting support.

## ✨ Features

- **🆔 ParticipantId**: A simple wrapper around a `u8`, representing a unique identifier for a participant.
- **💾 Serialization and Deserialization**: Implements the `Serialize` and `Deserialize` traits from the `flood_rs` crate for efficient binary streaming.
- **🖨️ Display**: Provides a human-readable format for `ParticipantId`, making it easy to print or log participant identifiers.

## 📦 Installation

Add `nimble-participant` to your `Cargo.toml`:

```toml
[dependencies]
nimble-participant = "0.0.16"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
