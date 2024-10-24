# 🚀 Nimble FFI Library

Welcome to the **Nimble FFI Library**! This library serves as a bridge between Nimble, a powerful
network engine implemented in Rust, and other programming languages through its
Foreign Function Interface (FFI).

## ✨ Features

- **Cross-Language Compatibility**: Easily integrate Nimble with languages like C, C++, Python, and more.
- **High Performance**: Leverage Rust's performance to handle intensive network operations.

## Advanced

### Inspect .dylib

#### using `otool`

```bash
otool -Iv  ../../target/debug/libnimble_lib.dylib
```

#### Using `nm`

```bash
nm ../../target/debug/libnimble_lib.dylib
```

#### Show dependencies

```bash
otool -L  ../../target/debug/libnimble_lib.dylib
```
