/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::VersionProvider;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use hazy_transport::{DeciderConfig, Direction, DirectionConfig};
use log::{debug, error, info, warn};
use monotonic_time_rs::{Millis, MillisDuration};
use nimble_client::{Client, ClientError, GameCallbacks};
use nimble_host_front::{HostFront, HostFrontError};
use nimble_host_logic::logic::{GameStateProvider, HostConnectionId};
use nimble_sample_game::{SampleGame, SampleGameState};
use nimble_sample_step::SampleStep;
use rand::prelude::StdRng;
use rand::SeedableRng;
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
    GameT: GameCallbacks<SampleStep> + Debug,
    //    SampleStep: Clone + Deserialize + Serialize + Debug + Eq,
>(
    host: &mut HostFront<SampleStep>,
    state_provider: &impl GameStateProvider,
    connection_id: HostConnectionId,
    client: &mut Client<GameT, SampleStep>,
    now: &mut Millis,
    count: usize,
) -> Result<(), HostFrontError> {
    let mut tick_id = TickId::default();
    let config = DirectionConfig {
        decider: DeciderConfig {
            unaffected: 90,
            drop: 3,
            tamper: 0,
            duplicate: 4,
            reorder: 3,
        },
    };
    let rng = StdRng::seed_from_u64(0x01);
    let mut to_client = Direction::new(config, rng).expect("config should be valid");

    let config2 = DirectionConfig {
        decider: DeciderConfig {
            unaffected: 90,
            drop: 3,
            tamper: 0,
            duplicate: 4,
            reorder: 3,
        },
    };
    let rng2 = StdRng::seed_from_u64(0x01);
    let mut to_host = Direction::new(config2, rng2).expect("config should be valid");

    for _ in 0..count {
        for _ in 0..client.need_prediction_count() {
            debug!("trying to push predicted step for {tick_id}");
            let mut map = SeqMap::new();
            map.insert(ParticipantId(0), SampleStep::MoveLeft(-1))
                .expect("should insert map");
            let predicted_step = StepForParticipants::<SampleStep> { combined_step: map };
            client
                .push_predicted_step(tick_id, predicted_step)
                .expect("should push predicted step");
            tick_id += 1;
        }

        if client.can_join_player() && client.local_players().is_empty() {
            debug!("join player {tick_id}");
            client
                .request_join_player([0].into())
                .expect("should request join player");
        }

        // Push to host
        let to_host_datagrams = client.send(*now).expect("send should work");
        for to_host_datagram in to_host_datagrams {
            to_host.push(now.absolute_milliseconds(), &to_host_datagram);
        }

        // Pop everything that is ready to host:
        while let Some(item) = to_host.pop_ready(now.absolute_milliseconds()) {
            let to_client_datagrams_result =
                host.update(connection_id, *now, item.data.as_slice(), state_provider);

            // Push to client
            if let Ok(to_client_datagrams) = to_client_datagrams_result {
                for to_client_datagram in to_client_datagrams {
                    to_client.push(now.absolute_milliseconds(), &to_client_datagram);
                }
            } else {
                let error = to_client_datagrams_result.err().unwrap();
                log_err(&error);
                match error.error_level() {
                    ErrorLevel::Critical => {
                        return Err(error);
                    }
                    _ => {}
                }
            }
        }

        // Pop everything that is ready for client
        while let Some(item) = to_client.pop_ready(now.absolute_milliseconds()) {
            let result = client.receive(*now, item.data.as_slice());
            if let Err(err) = result {
                log_err(&err);
            }
        }

        client.update(*now).expect("update should work");
        *now += MillisDuration::from_millis(16);
    }
    Ok(())
}

use nimble_participant::ParticipantId;
use nimble_step_types::StepForParticipants;
use seq_map::SeqMap;

fn assert_eq_with_epsilon(a: f32, b: f32, epsilon: f32) {
    assert!(
        (a - b).abs() <= epsilon,
        "Values are not equal within the given epsilon: a = {:?}, b = {:?}",
        a,
        b
    );
}
#[test_log::test] // TODO: bring this back
fn client_to_host() -> Result<(), ClientError> {
    let mut now = Millis::new(0);
    let mut client = Client::<SampleGame, SampleStep>::new(now);

    let to_host = client.send(now)?;
    assert_eq!(to_host.len(), 1);

    let application_version = SampleGame::version();

    let mut host = HostFront::<SampleStep>::new(application_version, TickId::new(0));

    let connection_id = host.create_connection().expect("should work");

    let initial_game_state = SampleGameState { x: -11, y: 42 };

    let expected_game_state_octets = initial_game_state.to_octets()?;

    let state_provider = TestStateProvider {
        tick_id: TickId(0),
        payload: expected_game_state_octets,
    };

    let conn_before = host
        .get_logic(connection_id)
        .expect("should find connection");
    assert_eq!(
        conn_before.phase(),
        &nimble_host_logic::logic::Phase::WaitingForValidConnectRequest
    );
    host.update(connection_id, now, to_host[0].as_slice(), &state_provider)
        .expect("should update host");

    let conn = host
        .get_logic(connection_id)
        .expect("should find connection");
    assert_eq!(conn.phase(), &nimble_host_logic::logic::Phase::Connected);

    communicate::<SampleGame>(
        &mut host,
        &state_provider,
        connection_id,
        &mut client,
        &mut now,
        153,
    )
    .expect("should communicate");

    // let host_connection = host.get_stream(connection_id).expect("should find connection");
    // let x = host.session().participants.get(&ParticipantId(0)).expect("should find participant");

    let expected_game_state = SampleGameState { x: 0, y: 42 };

    assert_eq!(
        client
            .game()
            .expect("game state should be set")
            .authoritative,
        expected_game_state
    );

    let expected_predicted_state_with_prediction = SampleGameState { x: 5, y: 42 };

    assert_eq!(
        client.game().expect("game state should be set").predicted,
        expected_predicted_state_with_prediction
    );

    assert_eq_with_epsilon(client.metrics().incoming.datagrams_per_second, 62.5, 0.001);
    assert_eq!(client.metrics().incoming.octets_per_second, 3312.5); // 16 kbps. (normal maximum is 120 Kbps, extreme is 575 Kbps)

    assert_eq_with_epsilon(client.metrics().outgoing.datagrams_per_second, 62.5, 0.001);
    assert_eq!(client.metrics().outgoing.octets_per_second, 2187.5); // 2.8 Kbps

    Ok(())
}
