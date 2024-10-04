/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use monotonic_time_rs::{Millis, MillisDuration};
use nimble_blob_stream::out_stream::BlobStreamOut;
use std::time::Duration;

#[test_log::test]
fn check_last_sent_time() {
    let mut stream = BlobStreamOut::new(4, Duration::from_millis(250));

    let mut now = Millis::new(0);

    {
        let entries = stream.send(now, 2);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], 0);
        assert_eq!(entries[1], 1);
    }

    now += MillisDuration::from_millis(100);
    {
        let entries = stream.send(now, 3);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], 2);
        assert_eq!(entries[1], 3);
    }

    now += MillisDuration::from_millis(100);
    {
        let entries = stream.send(now, 3);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], 3);
    }

    now += MillisDuration::from_millis(150);
    {
        let entries = stream.send(now, 3);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], 0);
        assert_eq!(entries[1], 1);
        assert_eq!(entries[2], 2);
    }

    stream
        .set_waiting_for_chunk_index(3, 0)
        .expect("set_waiting_for_chunk_index must not fail");

    {
        let entries = stream.send(now, 3);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], 3);
    }
}
