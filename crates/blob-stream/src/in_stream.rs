/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::err::BlobError;
use crate::ChunkIndex;
use bit_array_rs::BitArray;

/// A struct representing a stream of binary data divided into fixed-size chunks.
#[allow(unused)]
#[derive(Debug)]
pub struct BlobStreamIn {
    pub(crate) bit_array: BitArray,
    pub(crate) fixed_chunk_size: usize,
    pub(crate) octet_count: usize,
    blob: Vec<u8>,
}

impl BlobStreamIn {
    /// Creates a new `BlobStreamIn` instance with the specified number of octets and chunk size.
    ///
    /// # Parameters
    /// - `octet_count`: The total number of octets (bytes) in the stream.
    /// - `fixed_chunk_size`: The size of each chunk in the stream.
    ///
    /// # Panics
    /// Will panic if `fixed_chunk_size` is zero.
    ///
    /// # Returns
    /// A new `BlobStreamIn` instance.
    #[allow(unused)]
    #[must_use]
    pub fn new(octet_count: usize, fixed_chunk_size: usize) -> Self {
        assert!(
            fixed_chunk_size > 0,
            "fixed_chunk_size must be greater than zero"
        );

        let chunk_count = octet_count.div_ceil(fixed_chunk_size);
        Self {
            bit_array: BitArray::new(chunk_count),
            fixed_chunk_size,
            octet_count,
            blob: vec![0u8; octet_count],
        }
    }

    /// Returns the total number of expected chunks.
    ///
    /// This function provides the total count of chunks that are expected
    /// based on the size of the data and the chunk size.
    ///
    /// # Returns
    ///
    /// The total number of chunks (`usize`) that are expected for the data.
    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        self.bit_array.bit_count()
    }

    /// Checks if all chunks have been received.
    ///
    /// # Returns
    /// `true` if all chunks have been received; `false` otherwise.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.bit_array.all_set()
    }

    /// Returns a reference to the complete blob if all chunks have been received.
    ///
    /// # Returns
    /// An `Option` containing a reference to the blob if complete; otherwise, `None`.
    #[must_use]
    pub fn blob(&self) -> Option<&[u8]> {
        self.is_complete().then(|| &self.blob[..])
    }

    /// Sets a chunk of data at the specified `chunk_index` with the provided `payload`.
    ///
    /// # Parameters
    /// - `chunk_index`: The index of the chunk to set.
    /// - `payload`: A slice of octets representing the chunk's data.
    ///
    /// # Errors
    /// Returns a `BlobError` if:
    /// - The `chunk_index` is invalid.
    /// - The `payload` size does not match the expected size for the chunk.
    /// - The chunk has already been set, with either the same or different contents.
    ///
    /// # Returns
    /// `Ok(())` if the chunk was set successfully; otherwise, a `BlobError`.
    pub fn set_chunk(&mut self, chunk_index: ChunkIndex, payload: &[u8]) -> Result<(), BlobError> {
        let chunk_count = self.bit_array.bit_count();
        if chunk_index >= chunk_count {
            return Err(BlobError::InvalidChunkIndex(chunk_index, chunk_count));
        }

        let expected_size = if chunk_index == chunk_count - 1 {
            // It was the last chunk
            if self.octet_count % self.fixed_chunk_size == 0 {
                self.fixed_chunk_size
            } else {
                self.octet_count % self.fixed_chunk_size
            }
        } else {
            self.fixed_chunk_size
        };

        if payload.len() != expected_size {
            return Err(BlobError::UnexpectedChunkSize(
                expected_size,
                payload.len(),
                chunk_index,
            ));
        }
        let octet_offset = chunk_index * self.fixed_chunk_size;
        if octet_offset + expected_size > self.blob.len() {
            return Err(BlobError::OutOfBounds);
        }

        if self.bit_array.get(chunk_index) {
            // It has been set previously
            let is_same_contents =
                &self.blob[octet_offset..octet_offset + expected_size] == payload;

            let err = if is_same_contents {
                return Ok(());
            } else {
                BlobError::RedundantContentDiffers(chunk_index)
            };

            return Err(err);
        }

        self.blob[octet_offset..octet_offset + expected_size].copy_from_slice(payload);

        self.bit_array.set(chunk_index);

        Ok(())
    }
}
