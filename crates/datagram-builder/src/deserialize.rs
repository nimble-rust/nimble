/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram::DatagramParser;
use flood_rs::{in_stream::InOctetStream, Deserialize, ReadOctetStream};
use std::io;

/// Deserializes a collection of datagrams into a vector of items.
///
/// Each datagram is parsed to extract its payload, and the payload is then
/// deserialized into items of type `T`. All successfully deserialized items
/// are aggregated into a single vector.
///
/// # Arguments
///
/// * `datagrams` - An iterable collection of byte slices, each representing a serialized datagram.
/// * `parser` - A mutable reference to an implementation of `DatagramParser`.
///
/// # Returns
///
/// * `io::Result<Vec<T>>` - A vector of deserialized items.
///
/// # Errors
///
/// Returns an `io::Error` if parsing any datagram or deserializing any item fails.
pub fn deserialize_datagrams<T, I>(
    datagrams: I,
    parser: &mut impl DatagramParser,
) -> io::Result<Vec<T>>
where
    T: Deserialize,
    I: IntoIterator<Item = Vec<u8>>,
{
    let mut items = Vec::new();

    for datagram in datagrams.into_iter() {
        let payload_slice = parser.parse(&datagram)?;

        let mut in_stream = InOctetStream::new(payload_slice);
        while !in_stream.has_reached_end() {
            let item = T::deserialize(&mut in_stream)?;
            items.push(item);
        }
    }

    Ok(items)
}
