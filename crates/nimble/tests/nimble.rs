/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::Version;
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use log::trace;
use monotonic_time_rs::Millis;
use nimble_client_logic::ClientLogic;
use nimble_host_logic::{GameStateProvider, HostConnectionId, HostLogic};
use nimble_sample_game::{SampleGame, SampleGameState, SampleStep};
use nimble_step::Step;
use std::fmt::{Debug, Display};
use tick_id::TickId;

fn communicate<
    SampleState: BufferDeserializer,
    SampleStep: Clone + Deserialize + Debug + Display + Eq + PartialEq,
>(
    host: &mut HostLogic<Step<SampleStep>>,
    state_provider: &impl GameStateProvider,
    connection_id: HostConnectionId,
    client: &mut ClientLogic<SampleState, Step<SampleStep>>,
) where
    SampleStep: Serialize,
{
    let now = Millis::new(0);

    let to_host = client.send();
    for cmd in &to_host {
        trace!("client >> host: {cmd:?}");
    }
    let to_client: Vec<_> = to_host
        .iter()
        .flat_map(|to_host| {
            host.update(connection_id, now, to_host, state_provider)
                .expect("should work in test")
        })
        .collect();

    for cmd in &to_client {
        trace!("client << host: {cmd:?}");
    }

    for to_client_cmd in to_client {
        client.receive(&to_client_cmd).expect("TODO: panic message");
    }
}

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
fn client_host_integration() {
    let game = SampleGame::default();
    let state_octets = game
        .authoritative_octets()
        .expect("expect it possible to get state");
    let game_state_provider = TestStateProvider {
        tick_id: TickId(42),
        payload: state_octets,
    };
    let simulation_version = Version::new(1, 0, 0);

    let mut host = HostLogic::<Step<SampleStep>>::new(TickId(42), simulation_version.clone());
    let connection = host.create_connection().expect("should create connection");

    let mut client = ClientLogic::<SampleGameState, Step<SampleStep>>::new(simulation_version);

    client.set_joining_player([0].into());

    communicate(&mut host, &game_state_provider, connection, &mut client);
}
