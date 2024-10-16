/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::err::BlobError;
use crate::in_stream::BlobStreamIn;
use crate::protocol::{AckChunkData, SetChunkData};
use crate::ChunkIndex;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Info {
    pub total_octet_size: usize,
    pub chunk_octet_size: usize,
    pub chunk_count: usize,
    pub chunk_count_received: usize,
    pub waiting_for_chunk_index: ChunkIndex,
}

/// `Logic` handles the logic for receiving and processing chunks of data
/// in a streaming context. It manages the internal state and interactions
/// between the sender and receiver commands.
#[derive(Debug)]
pub struct Logic {
    in_stream: BlobStreamIn,
}

impl Logic {
    /// Creates a new `Logic` instance with the specified `octet_count` and `chunk_size`.
    ///
    /// # Arguments
    ///
    /// * `octet_count` - The total number of octets (bytes) expected in the stream.
    /// * `chunk_size` - The size of each chunk in the stream.
    ///
    /// # Returns
    ///
    /// A new `Logic` instance.
    ///
    /// # Example
    ///
    /// ```
    /// use nimble_blob_stream::in_logic::Logic;
    /// let in_logic = Logic::new(1024, 64);
    /// ```
    #[must_use]
    pub fn new(octet_count: usize, chunk_size: usize) -> Self {
        Self {
            in_stream: BlobStreamIn::new(octet_count, chunk_size),
        }
    }

    #[must_use]
    pub fn info(&self) -> Info {
        Info {
            total_octet_size: self.in_stream.octet_count,
            chunk_octet_size: self.in_stream.fixed_chunk_size,
            chunk_count: self.in_stream.bit_array.bit_count(),
            chunk_count_received: self.in_stream.bit_array.count_set_bits(),
            waiting_for_chunk_index: self
                .in_stream
                .bit_array
                .first_unset_bit()
                .unwrap_or_else(|| self.in_stream.bit_array.bit_count()),
        }
    }

    /// Processes a `SenderToReceiverCommands` command, applying it to the internal stream.
    ///
    /// Currently, this function only handles the `SetChunk` command, which updates the
    /// stream with a new chunk of data.
    ///
    /// # Arguments
    ///
    /// * `command` - The command sent by the sender, containing the chunk data.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] if the chunk cannot be set due to an I/O error.
    ///
    /// # Example
    ///
    /// ```
    /// use nimble_blob_stream::in_logic::Logic;
    /// use nimble_blob_stream::protocol::{SetChunkData};
    ///
    /// let mut in_logic = Logic::new(1024, 5);
    /// let chunk_data = SetChunkData {
    ///   chunk_index: 1,
    ///   payload: [0x8f, 0x23, 0x98, 0xfa, 0x99].into(),
    /// };
    /// in_logic.receive(&chunk_data).unwrap();
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    pub fn receive(&mut self, chunk_data: &SetChunkData) -> Result<(), BlobError> {
        self.in_stream
            .set_chunk(chunk_data.chunk_index as ChunkIndex, &chunk_data.payload)
    }

    pub fn send(&mut self) -> AckChunkData {
        let waiting_for_chunk_index = self
            .in_stream
            .bit_array
            .first_unset_bit()
            .unwrap_or_else(|| self.in_stream.bit_array.bit_count());

        let receive_mask = self
            .in_stream
            .bit_array
            .atom_from_index(waiting_for_chunk_index + 1);

        AckChunkData {
            waiting_for_chunk_index: waiting_for_chunk_index as u32,
            receive_mask_after_last: receive_mask,
        }
    }

    /// Retrieves the full blob data if all chunks have been received.
    ///
    /// # Returns
    ///
    /// An `Some(&[u8])` containing the full blob data if all chunks have been received,
    /// or `None` if the blob is incomplete.
    ///
    /// # Example
    ///
    /// ```
    /// use nimble_blob_stream::in_logic::Logic;
    /// let mut in_logic = Logic::new(1024, 64);
    /// if let Some(blob) = in_logic.blob() {
    ///     // Use the blob data
    /// }
    /// ```
    #[must_use]
    pub fn blob(&self) -> Option<&[u8]> {
        self.in_stream.blob()
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.in_stream.is_complete()
    }
}
