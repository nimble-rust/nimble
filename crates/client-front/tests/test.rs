/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use hexify::{assert_eq_slices, format_hex};
use log::info;
use monotonic_time_rs::{Millis, MonotonicClock};
use nimble_client_front::{ClientFront, ClientFrontError};
use nimble_protocol::Version;
use nimble_sample_step::{SampleState, SampleStep};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct FakeClock {
    millis: Millis,
}

impl FakeClock {
    pub fn set_time(&mut self, time: u64) {
        self.millis = Millis::new(time);
    }
}

impl MonotonicClock for FakeClock {
    fn now(&self) -> Millis {
        self.millis
    }
}

#[test_log::test]
pub fn client() -> Result<(), ClientFrontError> {
    let app_version = Version {
        major: 0,
        minor: 0,
        patch: 0,
    };

    let fake_clock = FakeClock {
        millis: Millis::new(0),
    };
    let concrete_clock: Rc<RefCell<FakeClock>> = Rc::new(RefCell::new(fake_clock));
    let monotonic_clock = Rc::clone(&concrete_clock) as Rc<RefCell<dyn MonotonicClock>>;

    let mut client =
        ClientFront::<SampleState, SampleStep>::new(&app_version, Rc::clone(&monotonic_clock));

    let datagrams = client.send()?;

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

    concrete_clock.borrow_mut().set_time(0xf00d);

    let datagrams_after = client.send()?;

    info!("datagrams_after: {}", format_hex(&datagrams_after[0]));

    let expected_after: &[u8] = &[
        // Header
        0x00, 0x01, // Datagram ID
        0xF0, 0x0D, // Client Time
        // Commands
        0x05, // Connect
        0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // Nimble Version
        0x00, // Flags
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Application Version
        0x00, // Request ID
    ];

    assert_eq_slices(&datagrams_after[0], expected_after);

    for index in 1u8..11 {
        let feed: &[u8] = &[
            // Header
            0x00, index, // Datagram ID
            0xF0, index, // Client Time
        ];
        client.receive(&feed)?;
    }

    assert_eq!(
        client.latency().expect("values should be set by now"),
        (3, 7.5, 12)
    );

    Ok(())
}
