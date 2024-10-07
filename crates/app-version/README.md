# Version

A simple Rust library for handling semantic versioning. 
The `Version` struct allows for easy representation and 
manipulation of version numbers in the format `major.minor.patch`.
Used for serializing version information over the network, particularly for deterministic software.

## Features

- Parse version strings (e.g., "1.2.3") into `Version` objects.
- Convert tuples of the form `(u16, u16, u16)` into `Version` objects.
- Display versions in a user-friendly format.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
version = "0.0.1"
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
