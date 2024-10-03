use hexify::assert_eq_slices;
use monotonic_time_rs::Millis;
use nimble_host_front::{HostFront, HostFrontError};
use nimble_host_logic::logic::{GameStateProvider, HostConnectionId};
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
    StepT: Clone + std::fmt::Debug + std::cmp::Eq + flood_rs::Deserialize + flood_rs::Serialize,
>() -> Result<(HostFront<StepT>, HostConnectionId, TestStateProvider), HostFrontError> {
    #[rustfmt::skip]
    let connect_datagram: &[u8] = &[
        // Header
        0x00, 0x00,
        0xF0, 0x0D,

        // Commands

        0x05, // Connect Request: ClientToHostOobCommand::ConnectType = 0x05
        0, 0, 0, 0, 0, 5, // Nimble version
        0, // Flags (use debug stream)
        0, 0, 0, 1, 0, 2, // Application version
        0,  // Client Request Id
    ];

    let application_version = app_version::Version {
        major: 0,
        minor: 1,
        patch: 2,
    };
    let mut host = HostFront::<StepT>::new(&application_version, TickId(0));

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
fn join_game() -> Result<(), HostFrontError> {
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
        0x01, // Number of players
        0x42, // The local player index
    ];

    let now = Millis::new(0);
    assert_eq!(host.session().participants.len(), 0);

    let maybe_join_response_datagrams =
        host.update(connection_id, now, join_datagram, &state_provider)?;
    assert_eq!(maybe_join_response_datagrams.len(), 1);

    assert_eq!(host.session().participants.len(), 1);

    #[rustfmt::skip]
    let expected_join_response: &[u8] = &[
        // Header
        0x00, 0x00, // Datagram Sequence
        0xF0, 0x0D, // Client Time
    
        // Commands
        0x09, // Join Game Response
        0x00, // Client Request ID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // SECRET
        0x00, // Party ID
        0x01, // Number of participants that joined
        0x42, // The index of the local player
        0x00, // The Participant ID assigned to that local player
    ];

    assert_eq_slices(&maybe_join_response_datagrams[0], expected_join_response);

    Ok(())
}

#[test_log::test]
fn game_step() -> Result<(), HostFrontError> {
    let (mut host, connection_id, state_provider) = create_and_connect::<SampleStep>()?;
    #[rustfmt::skip]
    let feed_predicted_steps = &[
        // Header
        0x00, 0x01, // Datagram Sequence
        0xF0, 0x0D, // Client Time

        // Commands
        0x02, // Send Predicted steps
        0x00, 0x00, 0x00, 0x00, // Waiting for Tick ID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Receive Mask for steps
        0x00, // number of player streams following
    ];

    let now = Millis::new(0);

    let maybe_step_response_datagrams =
        host.update(connection_id, now, feed_predicted_steps, &state_provider)?;

    #[rustfmt::skip]
    let expected_game_step_response = &[
        // Header
        0x00, 0x00, // Datagram Sequence
        0xF0, 0x0D, // Client Time
    
        // Commands
        0x08, // Game Step Response

        // Ack
        0x00, // Buffer count
        0x00, // Signed 8-bit delta buffer
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
