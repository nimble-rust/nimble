# 🎮 nimble-participant

`nimble-participant` is a Rust library designed to represent participants in a deterministic simulation. It provides a `ParticipantId` type with built-in serialization, deserialization, and formatting support.

## ✨ Features

- **🆔 ParticipantId**: A simple wrapper around a `u8`, representing a unique identifier for a participant.
- **💾 Serialization and Deserialization**: Implements the `Serialize` and `Deserialize` traits from the `flood_rs` crate for efficient binary streaming.
- **🖨️ Display**: Provides a human-readable format for `ParticipantId`, making it easy to print or log participant identifiers.

## 🚀 Usage

Add `nimble-participant` to your `Cargo.toml`:

```toml
[dependencies]
nimble-participant = "0.0.14-dev"
```
