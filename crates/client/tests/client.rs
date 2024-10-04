/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::VersionProvider;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::{Deserialize, Serialize};
use log::{error, info, warn};
use monotonic_time_rs::{Millis, MillisDuration};
use nimble_client::{Client, GameCallbacks};
use nimble_client_front::ClientFrontError;
use nimble_host_front::HostFront;
use nimble_host_logic::logic::{GameStateProvider, HostConnectionId};
use nimble_sample_game::{SampleGame, SampleGameState};
use nimble_sample_step::SampleStep;
use std::fmt::Debug;
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

fn log_err<T: ErrorLevelProvider + Debug>(error: &T) {
    match error.error_level() {
        ErrorLevel::Info => info!("{:?}", error),
        ErrorLevel::Warning => warn!("{:?}", error),
        ErrorLevel::Critical => error!("{:?}", error),
    }
}

fn communicate<
    GameT: GameCallbacks<StepT> + std::fmt::Debug,
    StepT: Clone + Deserialize + Serialize + Debug + Eq,
>(
    host: &mut HostFront<StepT>,
    state_provider: &impl GameStateProvider,
    connection_id: HostConnectionId,
    client: &mut Client<GameT, StepT>,
    count: usize,
) {
    let mut now = Millis::new(0);
    for _ in 0..count {
        let to_host_datagrams = client.send().expect("send should work");

        for to_host_datagram in to_host_datagrams {
            let to_client_datagrams_result = host.update(
                connection_id,
                now,
                to_host_datagram.as_slice(),
                state_provider,
            );
            if let Ok(to_client_datagrams) = to_client_datagrams_result {
                for to_client_datagram in to_client_datagrams {
                    let result = client.receive(to_client_datagram.as_slice());
                    if let Err(err) = result {
                        log_err(&err);
                    }
                }
            } else {
                warn!("received: {:?}", to_client_datagrams_result.err());
            }
        }

        now += MillisDuration::from_millis(16);
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

    let expected_game_state = SampleGameState { x: -11, y: 42 };

    let expected_game_state_octets = expected_game_state.to_octets()?;

    let state_provider = TestStateProvider {
        tick_id: TickId(0),
        payload: expected_game_state_octets,
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

    communicate::<SampleGame, SampleStep>(
        &mut host,
        &state_provider,
        connection_id,
        &mut client,
        3,
    );

    client.update();

    assert_eq!(
        client
            .game_state()
            .expect("game state should be set")
            .authoritative,
        expected_game_state
    );

    Ok(())
}
