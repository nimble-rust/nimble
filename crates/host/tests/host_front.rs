/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use hexify::assert_eq_slices;
use monotonic_time_rs::Millis;
use nimble_host::{Host, HostError};
use nimble_host_logic::{GameStateProvider, HostConnectionId};
use nimble_participant::ParticipantId;
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

fn create_and_connect<
    StepT: Clone
        + std::fmt::Debug
        + std::fmt::Display
        + std::cmp::Eq
        + flood_rs::Deserialize
        + flood_rs::Serialize,
>() -> Result<(Host<StepT>, HostConnectionId, TestStateProvider), HostError> {
    #[rustfmt::skip]
    let connect_datagram: &[u8] = &[
        // Header
        0x00, 0x00,
        0xF0, 0x0D,

        // Commands
        0x05,               // Connect Request: ClientToHostOobCommand::ConnectType = 0x05
        0, 0, 0, 0, 0, 5,   // Nimble version
        0,                  // Flags (use debug stream). Not used yet.
        0, 0, 0, 1, 0, 2,   // Application version
        0,                  // Client Request Id
    ];

    let application_version = app_version::Version::new(0, 1, 2);
    let mut host = Host::<StepT>::new(application_version, TickId(0));

    let not_used_connection_id = host
        .create_connection()
        .expect("should have connection here");
    assert_eq!(not_used_connection_id.0, 0);

    let connection_id = host
        .create_connection()
        .expect("should have connection here");
    assert_eq!(connection_id.0, 1);

    let state_provider = TestStateProvider {
        tick_id: TickId(32),
        payload: vec![0xff],
    };
    let now = Millis::new(0);

    host.update(connection_id, now, connect_datagram, &state_provider)?;

    Ok((host, connection_id, state_provider))
}

#[test_log::test]
fn join_game() -> Result<(), HostError> {
    let (mut host, connection_id, state_provider) = create_and_connect::<SampleStep>()?;

    #[rustfmt::skip]
    let join_datagram: &[u8] = &[
        // Header
        0x00, 0x01, // Datagram Sequence
        0xF0, 0x0D, // Client Time
    
        // Commands
        0x01, // Join Game Command
        0x00, // RequestID
        0x00, // Join Type: No Secret
        0x02, // Number of players
        0x42, // The local player index for first player
        0xFF, // Local player index for second player
    ];

    let now = Millis::new(0);
    assert_eq!(host.session().participants.len(), 0);

    let maybe_join_response_datagrams =
        host.update(connection_id, now, join_datagram, &state_provider)?;
    assert_eq!(maybe_join_response_datagrams.len(), 1);

    assert_eq!(host.session().participants.len(), 2);

    let expected_participant_id = ParticipantId(0);
    let participant = host
        .session()
        .participants
        .get(&expected_participant_id)
        .expect("should have participant");

    assert_eq!(participant.borrow().id.0, 0);
    assert_eq!(participant.borrow().client_local_index, 0x42);

    #[rustfmt::skip]
    let expected_join_response: &[u8] = &[
        // Header
        0x00, 0x01, // Datagram Sequence
        0xF0, 0x0D, // Client Time
    
        // Commands
        0x09, // Join Game Response
        0x00, // Client Request ID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // SECRET
        0x00, // Party ID - Only for debug purposes. Maybe should be removed?
        0x02, // Number of participants that joined
        0x42, // The index of the first local player
        0x00, // The Participant ID assigned to that first local player
        0xFF, // The index of the second local player
        0x01, // The Participant ID assigned to that second local player
    ];

    assert_eq_slices(&maybe_join_response_datagrams[0], expected_join_response);

    Ok(())
}

#[test_log::test]
fn game_step() -> Result<(), HostError> {
    let (mut host, connection_id, state_provider) = create_and_connect::<SampleStep>()?;
    #[rustfmt::skip]
    let feed_predicted_steps = &[
        // Header
        0x00, 0x01, // Datagram Sequence
        0xF0, 0x0D, // Client Time

        // Commands
        0x02, // Send Predicted steps Command
        0x00, 0x00, 0x00, 0x00, // Waiting for Tick ID
        0x00, 0x00, 0x00, 0x00, // Base tick id
        0x00, // number of player streams following
    ];

    let now = Millis::new(0);

    let maybe_step_response_datagrams =
        host.update(connection_id, now, feed_predicted_steps, &state_provider)?;

    #[rustfmt::skip]
    let expected_game_step_response = &[
        // Header
        0x00, 0x01, // Datagram Sequence
        0xF0, 0x0D, // Client Time
    
        // Commands
        0x08, // Game Step Response

        // Ack
        0x00, // Buffer count
        0x00, // Signed 8-bit delta tick
        0x00, 0x00, 0x00, 0x00, // Next Expected TickID. Signals that it has not received anything yet.

        // Authoritative Steps
        0x00, 0x00, 0x00, 0x00, // Start TickID
        0x00, // Number of ranges following
    ];

    assert_eq_slices(
        &maybe_step_response_datagrams[0],
        expected_game_step_response,
    );

    Ok(())
}
