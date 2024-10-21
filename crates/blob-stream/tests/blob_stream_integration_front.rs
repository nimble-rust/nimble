/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::helper::generate_deterministic_blob_array;
use log::trace;
use monotonic_time_rs::{Millis, MillisDuration};
use nimble_blob_stream::out_logic_front::OutLogicFront;
use nimble_blob_stream::prelude::TransferId;
use rand::prelude::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::time::Duration;

pub mod helper;

#[test_log::test]
fn blob_stream_front() {
    const CHUNK_SIZE: u16 = 4;
    const CHUNK_COUNT: u32 = 30;
    const OCTET_COUNT: usize = (CHUNK_SIZE as usize * (CHUNK_COUNT as usize - 1)) + 1;
    const ITERATION_COUNT: usize = 5;

    let seed = 12345678;
    let blob_to_transfer = generate_deterministic_blob_array(OCTET_COUNT, seed);
    let mut drop_rng = StdRng::seed_from_u64(seed);

    let mut in_logic = nimble_blob_stream::in_logic_front::FrontLogic::new();
    let mut out_logic = OutLogicFront::new(
        TransferId(42),
        CHUNK_SIZE,
        Duration::from_millis(31 * 3),
        blob_to_transfer.as_slice(),
    )
    .expect("should work to create logic");

    let mut now = Millis::new(0);

    for _ in 0..ITERATION_COUNT {
        let send_commands = out_logic.send(now).expect("should work");
        for send_command in send_commands {
            // Intentionally drop commands (datagrams)
            if !drop_rng.gen_bool(0.2) {
                in_logic.receive(&send_command).expect("should work");
                let commands_from_receiver = in_logic.send().expect("should work to send");
                if !drop_rng.gen_bool(0.2) {
                    out_logic
                        .receive(&commands_from_receiver)
                        .expect("should work");
                } else {
                    trace!("dropped from receiver to sender: {:?}", send_command);
                }
            } else {
                trace!("dropped from sender to receiver: {:?}", send_command);
            }
        }
        now += MillisDuration::from_millis(32);
    }

    assert_eq!(
        in_logic.blob().expect("blob should be ready"),
        blob_to_transfer
    );

    assert!(out_logic.is_received_by_remote());
}
