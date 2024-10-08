/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use hexify::{assert_eq_slices, format_hex};
use log::info;
use monotonic_time_rs::Millis;
use nimble_client_front::{ClientFront, ClientFrontError};
use nimble_sample_step::{SampleState, SampleStep};

#[test_log::test]
pub fn client() -> Result<(), ClientFrontError> {
    let app_version = app_version::Version::new(0, 0, 0);

    let mut now = Millis::new(0);
    let mut client = ClientFront::<SampleState, SampleStep>::new(app_version, now);

    let datagrams = client.send(now)?;

    let datagram = &datagrams[0];

    info!("received: {}", format_hex(datagram));
    let expected: &[u8] = &[
        // Header
        0x00, 0x00, // Datagram ID
        0x00, 0x00, // Client Time
        // Commands
        0x05, // Connect
        0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // Nimble Version
        0x00, // Flags
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Application Version
        0x00, // Request ID
    ];

    assert_eq_slices(datagram, expected);

    now = Millis::from(0xf000);

    let datagrams_after = client.send(now)?;

    info!("datagrams_after: {}", format_hex(&datagrams_after[0]));

    let expected_after: &[u8] = &[
        // Header
        0x00, 0x01, // Datagram ID
        0xF0, 0x00, // Client Time
        // Commands
        0x05, // Connect
        0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // Nimble Version
        0x00, // Flags
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Application Version
        0x00, // Request ID
    ];

    assert_eq_slices(&datagrams_after[0], expected_after);

    for index in 1u8..11 {
        let absolute_time_when_sent_lower_octet = index * 16;
        let feed: &[u8] = &[
            // Header
            0x00,
            2 + index * 3, // Datagram ID
            0xF0,
            absolute_time_when_sent_lower_octet, // Client Time
        ];

        let current_absolute_time = 0xf000 + 200u64 + index as u64 * 20;
        now = Millis::new(current_absolute_time);
        client.update(now);
        client.receive(now, &feed)?;
        if index % 2 == 0 {
            let _ = client.send(now)?;
        }
    }

    assert_eq!(client.in_datagrams_per_second(), 50.0);

    assert_eq!(
        client.latency().expect("values should be set by now"),
        (204, 222.0, 240)
    );

    assert_eq!(
        client
            .datagram_drops()
            .expect("values should be set by now"),
        (2, 2.3, 5)
    );

    assert_eq!(client.out_octets_per_second(), 380.0);

    assert_eq!(client.out_datagrams_per_second(), 20.0);

    assert_eq!(client.in_octets_per_second(), 200.0);

    assert_eq!(client.in_datagrams_per_second(), 50.0);

    Ok(())
}
