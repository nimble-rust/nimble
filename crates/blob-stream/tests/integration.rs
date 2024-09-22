/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use blob_stream::out_logic::Logic;
use blob_stream::prelude::TransferId;
use blob_stream::protocol::AckChunkData;
use log::info;

use crate::helper::generate_deterministic_blob_array;
use std::time::{Duration, Instant};
pub mod helper;

#[test_log::test]
fn test_blob_stream() {
    let seed = 12345678;
    let blob_to_transfer = generate_deterministic_blob_array(OCTET_COUNT, seed);
    const CHUNK_SIZE: usize = 4;
    const CHUNK_COUNT: usize = 30;
    const OCTET_COUNT: usize = (CHUNK_SIZE * (CHUNK_COUNT - 1)) + 1;
    const ITERATION_COUNT: usize = 9;
    const MAX_CHUNK_COUNT_EACH_SEND: usize = 10;

    let mut in_logic = blob_stream::in_logic::Logic::new(blob_to_transfer.len(), CHUNK_SIZE);
    let mut out_logic = Logic::new(
        TransferId(0),
        CHUNK_SIZE,
        Duration::from_millis(31 * 3),
        blob_to_transfer.as_slice(),
    );

    let mut now = Instant::now();

    for i in 0..ITERATION_COUNT {
        let set_chunks = out_logic.send(now, MAX_CHUNK_COUNT_EACH_SEND);
        assert!(set_chunks.len() <= MAX_CHUNK_COUNT_EACH_SEND);

        if (i % 3) == 0 {
            // Intentionally drop a few chunks every third iteration
            info!("dropped those chunks");
            continue;
        }

        let mut ack: Option<AckChunkData> = None;

        for set_chunk in set_chunks {
            in_logic
                .receive(&set_chunk.data)
                .expect("should always be valid in test");
            ack = Some(in_logic.send());
        }

        if let Some(ack) = ack {
            info!("ack: {:?}", ack);
            out_logic
                .set_waiting_for_chunk_index(
                    ack.waiting_for_chunk_index as usize,
                    ack.receive_mask_after_last,
                )
                .expect("ack chunk index and receive mask should work in the test");
        }

        now += Duration::from_millis(32);
    }

    assert_eq!(
        in_logic.blob().expect("blob should be ready"),
        blob_to_transfer
    );

    assert!(out_logic.is_received_by_remote());
}
