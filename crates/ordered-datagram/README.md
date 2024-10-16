# 🚀 nimble-ordered-datagram

`nimble-ordered-datagram` ensures that datagrams are received and processed in order, discarding duplicates and handling reordering efficiently.

This crate is ideal for real-time networked applications, where maintaining the correct order of datagrams is crucial for smooth gameplay or data flow.

## Features ✨

- **Datagram ID Management**: Assign unique IDs to your datagrams for ordered transmission.
- **Duplicate & Reordering Handling**: Automatically discard duplicate or out-of-order datagrams.
- **Low Latency**: Optimized for use in high-throughput environments with expected low-latency networks.

## How It Works 🤔

- Each datagram is assigned a `DatagramId` which is serialized and deserialized efficiently.
- The `OrderedOut` struct keeps track of the next `DatagramId` to send.
- The `OrderedIn` struct ensures that incoming datagrams are verified to be in order and discards any duplicates or reordered packets.

## Installation 📦

To include `nimble-ordered-datagram` in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
nimble-ordered-datagram = "0.0.14-dev"
```
