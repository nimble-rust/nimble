# hexify

`hexify` is a Rust library for formatting octet slices `[u8]` into hexadecimal strings. It provides utilities to
convert octets into easily readable hex strings with additional features for comparison and assertions.

## Installation

Add `hexify` to your `Cargo.toml`:

```toml
[dependencies]
hexify = "^0.0.1"
```

## Usage

```rust
use hexify::format_hex;

fn main() {
    let data = [0x42, 0xA4, 0xAE, 0x09, 0xAF, 0x00, 0x01, 0x00, 0x00, 0x04, 0x03, 0x00, 0x00];
    let output = format_hex(&data);
    println!("{}", output); // Outputs: 42 A4 AE 09 AF 00 01 00 00 04 03 00 00
}
```

## Features

* Hexadecimal Formatting: Convert `[u8]` slices into uppercase, space-separated hex strings.
* Comparison and Assertions: Easily compare octet slices and assert equality with detailed hex dump outputs for
  debugging.
* Flexible Formatting Options: Customize grouping and prefix options for hex output.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.
