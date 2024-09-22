/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram::{DatagramBuilder, DatagramError};
use flood_rs::{out_stream::OutOctetStream, Serialize};
use log::trace;
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;

/// Serializes a list of items into datagrams.
///
/// Each datagram is constructed using the provided [`DatagramBuilder`]. Items are serialized
/// and packed into datagrams without exceeding the `max_packet_size`.
///
/// # Arguments
///
/// * `items` - A slice of items to serialize.
/// * `max_packet_size` - The maximum size of each packet.
/// * `builder` - A mutable reference to an implementation of `DatagramBuilder`.
///
/// # Returns
///
/// * `io::Result<Vec<Vec<u8>>>` - A vector of serialized datagrams.
///
/// # Errors
///
/// Returns an `io::Error` if any item is too large to fit in a packet or if serialization fails.
pub fn serialize_datagrams<T, I>(
    items: I,
    builder: &mut impl DatagramBuilder,
) -> io::Result<Vec<Vec<u8>>>
where
    T: Serialize + Debug,
    I: AsRef<[T]>,
{
    let mut packets = Vec::new();

    builder.clear()?;

    for item in items.as_ref() {
        trace!("serializing item {item:?}");

        // Serialize the item into the buffer
        let mut item_stream = OutOctetStream::new();
        item.serialize(&mut item_stream)?;
        let item_octets = item_stream.octets_ref();
        let item_len = item_octets.len();

        if item_len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "zero item length is illegal for serialization",
            ));
        }

        match builder.push(item_octets) {
            Err(DatagramError::BufferFull) => {
                packets.push(builder.finalize()?.to_vec());
                builder.clear()?;
                builder
                    .push(item_octets)
                    .map_err(|err| io::Error::new(ErrorKind::InvalidData, err))?;
            }
            Err(DatagramError::IoError(io_err)) => {
                return Err(io_err);
            }
            Err(err) => {
                // Handle any unexpected errors or provide a default error handling
                // Optionally, you can log or return an error here if needed
                return Err(io::Error::new(ErrorKind::InvalidData, err));
            }
            Ok(_) => {}
        }
    }

    if !builder.is_empty() {
        packets.push(builder.finalize()?.to_vec());
    }

    Ok(packets)
}
