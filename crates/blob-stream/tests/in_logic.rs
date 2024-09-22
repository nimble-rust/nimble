/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use blob_stream::in_logic::Logic;
use blob_stream::prelude::*;

#[test_log::test]
fn check_receive() {
    let mut logic = Logic::new(10, 5);

    let set_chunk_data = SetChunkData {
        chunk_index: 1,
        payload: [0x8f, 0x23, 0x98, 0xfa, 0x99].into(),
    };

    logic
        .receive(&set_chunk_data)
        .expect("should be able to receive valid SetChunk");

    let answer = logic.send();

    assert_eq!(answer.waiting_for_chunk_index, 0);
    assert_eq!(answer.receive_mask_after_last, 0b1); // Indicates that chunk_index 1 was received
}

#[test_log::test]
fn multiple_not_received() {
    let mut logic = Logic::new(11, 5);

    let set_chunk_data = SetChunkData {
        chunk_index: 2,
        payload: [0x8f].into(),
    };

    logic
        .receive(&set_chunk_data)
        .expect("should be able to receive valid SetChunk");

    let ack = logic.send();

    assert_eq!(ack.waiting_for_chunk_index, 0);
    assert_eq!(ack.receive_mask_after_last, 0b10); // Verifies that chunk_index 2 was received (bit 1 = index 2, bit 0 = index 1).
}

fn set_chunk_and_check(
    logic: &mut Logic,
    chunk_index: u32,
    payload: &[u8],
    waiting: u32,
    receive_mask: u64,
) {
    let set_chunk_data = SetChunkData {
        chunk_index,
        payload: payload.to_vec(),
    };
    logic.receive(&set_chunk_data).expect("update should work");
    let ack = logic.send();
    assert_eq!(ack.waiting_for_chunk_index, waiting);
    assert_eq!(ack.receive_mask_after_last, receive_mask);
}

#[test_log::test]
fn all_received() {
    let mut logic = Logic::new(11, 5);

    set_chunk_and_check(&mut logic, 2, &[0x8f], 0, 0b10); // Verifies that chunk_index 2 was received (bit 1 = index 2, bit 0 = index 1)
    set_chunk_and_check(&mut logic, 0, &[0x33; 5], 1, 0b1); // Verifies that chunk_index 2 was received (bit 0 = index 2)
    set_chunk_and_check(&mut logic, 1, &[0xff; 5], 3, 0b0);

    assert_eq!(
        logic.blob().expect("Blob slice should be complete"),
        &[0x33, 0x33, 0x33, 0x33, 0x33, 0xff, 0xff, 0xff, 0xff, 0xff, 0x8f]
    )
}
