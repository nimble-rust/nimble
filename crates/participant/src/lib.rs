/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*!
# nimble-participant

`nimble-participant` is a crate for managing participants in a deterministic simulation.

## Features ✨
- **`ParticipantId`**: Provides a unique identifier for participants using a simple wrapper around `u8`.
- **Serialization/Deserialization**: Supports binary serialization and deserialization through the `flood_rs` crate.
- **Display Formatting**: Offers a formatted, human-readable display for logging or printing participant identifiers.

## Usage 🚀

 Add the following to your `Cargo.toml`:

```toml
[dependencies]
nimble-participant = "0.1"
```

## Example:

```rust
use nimble_participant::ParticipantId;

let participant = ParticipantId(42);
println!("{}", participant); // Outputs: Participant(42)
```
*/

use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use std::fmt::Display;

/// Represents a unique participant in a simulation.
///
/// The `ParticipantId` wraps a `u8` and provides serialization,
/// deserialization, and display capabilities.
#[derive(PartialEq, Eq, Copy, Ord, Hash, Clone, Debug, PartialOrd)]
pub struct ParticipantId(pub u8);

impl Serialize for ParticipantId {
    /// Serializes the `ParticipantId` into the given stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to an object that implements `WriteOctetStream`.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if writing to the stream fails.
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> std::io::Result<()> {
        stream.write_u8(self.0)
    }
}

impl Deserialize for ParticipantId {
    /// Deserializes a `ParticipantId` from the given stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - A mutable reference to an object that implements `ReadOctetStream`.
    ///
    /// # Errors
    ///
    /// Returns an `std::io::Error` if reading from the stream fails.
    fn deserialize(stream: &mut impl ReadOctetStream) -> std::io::Result<Self> {
        Ok(Self(stream.read_u8()?))
    }
}

impl Display for ParticipantId {
    /// Formats the `ParticipantId` for display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Participant({})", self.0)
    }
}
