/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_blob_stream::in_stream::BlobStreamIn;
pub mod helper;

#[test_log::test]
fn chunks_out_of_order() {
    const CHUNK_SIZE: u16 = 4;
    const CHUNK_COUNT: u32 = 3;

    let mut stream = BlobStreamIn::new(
        (CHUNK_COUNT - 1) as usize * CHUNK_SIZE as usize + 1,
        CHUNK_SIZE,
    );

    assert!(
        !stream.is_complete(),
        "Stream should not be complete initially"
    );

    stream
        .set_chunk(1, &[0xff, 0xfe, 0xfd, 0xfc])
        .expect("Setting chunk 1 should work");

    stream
        .set_chunk(0, &[0x31, 0x32, 0x33, 0x34])
        .expect("Setting chunk 0 should work");

    assert!(!stream.is_complete());

    stream
        .set_chunk(2, &[0x42])
        .expect("Setting chunk 2 should work");

    assert!(
        stream.is_complete(),
        "Stream should be complete after setting all chunks"
    );
    assert_eq!(
        stream.blob().expect("Blob slice should be complete"),
        &[0x31, 0x32, 0x33, 0x34, 0xff, 0xfe, 0xfd, 0xfc, 0x42]
    );
}
