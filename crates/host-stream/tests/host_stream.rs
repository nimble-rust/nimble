/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use hexify::assert_eq_slices;
use monotonic_time_rs::{Millis, MillisDuration};
use nimble_host_logic::logic::GameStateProvider;
use nimble_host_stream::{HostStream, HostStreamError};
use nimble_sample_step::SampleStep;
use tick_id::TickId;

pub struct TestStateProvider {
    pub tick_id: TickId,
    pub payload: Vec<u8>,
}

impl GameStateProvider for TestStateProvider {
    fn state(&self, _: TickId) -> (TickId, Vec<u8>) {
        (self.tick_id, self.payload.clone())
    }
}

#[test_log::test]
fn join_game2() -> Result<(), HostStreamError> {
    let application_version = app_version::Version::new(0, 1, 2);
    let mut host = HostStream::<SampleStep>::new(application_version, TickId(0));
    let connection_id = host
        .create_connection()
        .expect("it should not be out of connections");

    let state_provider = TestStateProvider {
        tick_id: TickId(32),
        payload: vec![0xff],
    };
    let mut now = Millis::new(0);

    #[rustfmt::skip]
    let connect_datagram: &[u8] = &[
            0x05, // Connect Request: ClientToHostOobCommand::ConnectType = 0x05
            0, 0, 0, 0, 0, 5, // Nimble version
            0, // Flags (use debug stream)
            0, 0, 0, 1, 0, 2, // Application version
            0,  // Client Request Id
    ];

    let _ = host.update(connection_id, now, connect_datagram, &state_provider)?;

    #[rustfmt::skip]
    let join_datagram: &[u8] = &[
        0x01, // Join Game Command
        0x00, // RequestID
        0x00, // Join Type: No Secret
        0x01, // Number of players
        0x42, // The local player index
    ];

    now += MillisDuration::from_millis(200);

    let probably_join_responses =
        host.update(connection_id, now, join_datagram, &state_provider)?;

    assert_eq!(probably_join_responses.len(), 1);

    #[rustfmt::skip]
    let expected_response: &[u8] = &[
        0x09, // Join Game Response
        0x00, // Client Request ID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // SECRET
        0x00, // Party ID
        0x01, // Number of participants that joined
        0x42, // The index of the local player
        0x00, // The Participant ID assigned to that local player
    ];

    assert_eq_slices(&probably_join_responses[0], &expected_response);

    Ok(())
}
