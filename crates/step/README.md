# nimble-step

[![Crates.io](https://img.shields.io/crates/v/nimble-step)](https://crates.io/crates/nimble-step)
[![Documentation](https://docs.rs/nimble-step/badge.svg)](https://docs.rs/nimble-step)

nimble-step is a Rust crate that helps you manage participant state changes in deterministic
simulations. Whether participants are joining, leaving, or reconnecting, this crate provides
a flexible way to track these changes over time.

## ✨ Features

- Track participants joining or leaving the session
- Handle forced states and reconnections
- Customize participant actions with your own types
- Supports serialization/deserialization for efficient serialization

## 📦 Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
nimble-step = "0.0.14-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
