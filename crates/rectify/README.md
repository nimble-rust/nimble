# 🛠️ nimble-rectify

[![Crates.io](https://img.shields.io/crates/v/nimble-rectify)](https://crates.io/crates/nimble-rectify)
[![Documentation](https://docs.rs/nimble-rectify/badge.svg)](https://docs.rs/nimble-rectify)

`nimble-rectify` is a Rust crate that brings together both authoritative and predicted steps in a
deterministic simulation. It combines the power of
[nimble-assent](https://github.com/nimble-rust/nimble-assent) (authoritative steps)
and [nimble-seer](https://github.com/nimble-rust/nimble-seer) (predicted steps)
to manage game state seamlessly.

## ✨ Features

- Integrates **authoritative** and **predicted** steps for smooth state management.
- Provides **callbacks** for both [`Assent`](https://github.com/nimble-rust/nimble-assent) and [`Seer`](https://github.com/nimble-rust/nimble-seer).

## 📦 Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
nimble-rectify = "0.0.14-dev"
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
