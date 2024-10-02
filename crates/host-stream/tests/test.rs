use hexify::assert_eq_slices;
use monotonic_time_rs::Millis;
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
fn test() -> Result<(), HostStreamError> {
    let mut host = HostStream::<SampleStep>::new(TickId(0));
    let connection_id = host
        .create_connection()
        .expect("it should not be out of connections");

    let state_provider = TestStateProvider {
        tick_id: TickId(32),
        payload: vec![0xff],
    };

    #[rustfmt::skip]
    let join_datagram: &[u8] = &[
        0x01, // Join Game Command
        0x00, // RequestID
        0x00, // Join Type: No Secret
        0x01, // Number of players
        0x42, // The local player index
    ];

    let now = Millis::new(0);

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
