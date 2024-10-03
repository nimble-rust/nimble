/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::VersionProvider;
use monotonic_time_rs::Millis;
use nimble_client::Client;
use nimble_client_front::ClientFrontError;
use nimble_host_front::HostFront;
use nimble_host_logic::logic::GameStateProvider;
use nimble_sample_game::SampleGame;
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
fn client_to_host() -> Result<(), ClientFrontError> {
    let mut client = Client::<SampleGame, SampleStep>::new();

    let to_host = client.send()?;
    assert_eq!(to_host.len(), 1);

    let application_version = SampleGame::version();

    let mut host = HostFront::<SampleStep>::new(&application_version, TickId::new(0));

    let connection_id = host.create_connection().expect("should work");
    let now = Millis::new(0);

    let state_provider = TestStateProvider {
        tick_id: TickId(0),
        payload: [0xff, 0x13].to_vec(),
    };

    let conn_before = host
        .get_stream(connection_id)
        .expect("should find connection");
    assert_eq!(
        conn_before.phase(),
        &nimble_host_stream::HostStreamConnectionPhase::Connecting
    );
    host.update(connection_id, now, to_host[0].as_slice(), &state_provider)
        .expect("should update host");

    let conn = host
        .get_stream(connection_id)
        .expect("should find connection");
    assert_eq!(
        conn.phase(),
        &nimble_host_stream::HostStreamConnectionPhase::Connected
    );

    Ok(())
}
