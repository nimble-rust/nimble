/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::out_stream::{BlobStreamOut, OutStreamError};
use crate::prelude::{SetChunkData, SetChunkFrontData, TransferId};
use std::time::{Duration, Instant};

#[allow(unused)]
#[derive(Debug)]
pub struct Logic {
    out_stream: BlobStreamOut,
    blob: Vec<u8>,
    fixed_chunk_size: usize,
    transfer_id: TransferId,
}

impl Logic {
    pub fn new(
        transfer_id: TransferId,
        fixed_chunk_size: usize,
        resend_duration: Duration,
        blob: &[u8],
    ) -> Self {
        let chunk_count = blob.len().div_ceil(fixed_chunk_size);
        Self {
            out_stream: BlobStreamOut::new(chunk_count, resend_duration),
            blob: blob.to_vec(),
            transfer_id,
            fixed_chunk_size,
        }
    }

    #[inline]
    fn get_range(&self, index: usize) -> (usize, usize) {
        assert!(index < self.blob.len(), "out logic index out of bounds");
        let start = index * self.fixed_chunk_size;
        let is_last_chunk = index + 1 == self.out_stream.chunk_count();
        let count = if is_last_chunk {
            let remaining_size = self.blob.len() % self.fixed_chunk_size;
            if remaining_size == 0 {
                self.fixed_chunk_size
            } else {
                remaining_size
            }
        } else {
            self.fixed_chunk_size
        };

        (start, start + count)
    }

    #[must_use]
    pub fn send(&mut self, now: Instant, max_count: usize) -> Vec<SetChunkFrontData> {
        let indices = self.out_stream.send(now, max_count);
        let mut set_chunks = Vec::new();
        for chunk_index in indices {
            let (start, end) = self.get_range(chunk_index);
            let payload = &self.blob[start..end];
            let set_chunk = SetChunkFrontData {
                transfer_id: self.transfer_id,
                data: SetChunkData {
                    chunk_index: chunk_index as u32,
                    payload: payload.to_vec(),
                },
            };
            set_chunks.push(set_chunk);
        }
        set_chunks
    }

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
    pub fn octet_size(&self) -> usize {
        self.blob.len()
    }

    #[must_use]
    pub fn chunk_size(&self) -> usize {
        self.fixed_chunk_size
    }

    #[must_use]
    pub fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }
}
