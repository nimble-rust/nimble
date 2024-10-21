/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::out_stream::{BlobStreamOut, OutStreamError};
use crate::prelude::{SetChunkData, SetChunkFrontData, TransferId};
use monotonic_time_rs::Millis;
use std::time::Duration;

#[allow(unused)]
#[derive(Debug)]
pub struct Logic {
    out_stream: BlobStreamOut,
    blob: Vec<u8>,
    fixed_chunk_size: u16,
    transfer_id: TransferId,
}

impl Logic {
    /// # Errors
    /// `OutStreamError` // TODO:
    pub fn new(
        transfer_id: TransferId,
        fixed_chunk_size: u16,
        resend_duration: Duration,
        blob: &[u8],
    ) -> Result<Self, OutStreamError> {
        let chunk_count = blob.len().div_ceil(fixed_chunk_size as usize);
        let chunk_count = u32::try_from(chunk_count).map_err(OutStreamError::BlobIsTooLarge)?;
        Ok(Self {
            out_stream: BlobStreamOut::new(chunk_count, resend_duration),
            blob: blob.to_vec(),
            transfer_id,
            fixed_chunk_size,
        })
    }

    #[must_use]
    #[inline]
    fn get_range(&self, index: u32) -> Option<(usize, usize)> {
        if index >= self.out_stream.chunk_count() {
            return None;
        }
        let is_last_chunk = index + 1 == self.out_stream.chunk_count();
        let count = if is_last_chunk {
            let remaining_size = self.blob.len() % (self.fixed_chunk_size as usize);
            if remaining_size == 0 {
                self.fixed_chunk_size
            } else {
                remaining_size as u16
            }
        } else {
            self.fixed_chunk_size
        };
        let start = index * self.fixed_chunk_size as u32;
        assert!(
            start < self.blob.len() as u32,
            "out logic index out of bounds"
        );
        assert!(
            (start + count as u32) <= (self.blob.len() as u32),
            "out logic index out of bounds"
        );

        Some((start as usize, start as usize + count as usize))
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn send(&mut self, now: Millis, max_count: usize) -> Vec<SetChunkFrontData> {
        let indices = self.out_stream.send(now, max_count);
        let mut set_chunks = Vec::new();
        for chunk_index in indices {
            let (start, end) = self
                .get_range(chunk_index)
                .expect("indices returned should be valid");
            let payload = &self.blob[start..end];
            let set_chunk = SetChunkFrontData {
                transfer_id: self.transfer_id,
                data: SetChunkData {
                    chunk_index,
                    payload: payload.to_vec(),
                },
            };
            set_chunks.push(set_chunk);
        }
        set_chunks
    }

    /// # Errors
    /// `OutStreamError` // TODO:
    pub fn set_waiting_for_chunk_index(
        &mut self,
        waiting_for_index: usize,
        receive_mask: u64,
    ) -> Result<(), OutStreamError> {
        self.out_stream
            .set_waiting_for_chunk_index(waiting_for_index, receive_mask)
    }

    #[must_use]
    pub fn is_received_by_remote(&self) -> bool {
        self.out_stream.is_received_by_remote()
    }

    #[must_use]
    pub fn octet_size(&self) -> u32 {
        self.blob.len() as u32
    }

    #[must_use]
    pub const fn chunk_size(&self) -> u16 {
        self.fixed_chunk_size
    }

    #[must_use]
    pub const fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }
}
