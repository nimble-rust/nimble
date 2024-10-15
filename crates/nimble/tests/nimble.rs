/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use log::trace;
use std::fmt::{Debug, Display};

use app_version::VersionProvider;
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use monotonic_time_rs::Millis;
use tick_id::TickId;

use nimble_step::Step;

use nimble_client::prelude::{Client, GameCallbacks};
use nimble_host::prelude::{GameStateProvider, Host, HostConnectionId};

use nimble_sample_game::SampleGame;
use nimble_sample_step::SampleStep;

fn communicate<
    GameT: BufferDeserializer + VersionProvider + GameCallbacks<StepT> + Debug,
    StepT: Clone + Deserialize + Debug + Display + Eq + PartialEq,
>(
    host: &mut Host<Step<StepT>>,
    state_provider: &impl GameStateProvider,
    connection_id: HostConnectionId,
    client: &mut Client<GameT, StepT>,
) where
    StepT: Serialize,
{
    let now = Millis::new(0);

    let to_host = client.send(now).expect("should work");
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
        client.receive(now, &to_client_cmd).expect("TODO: panic message");
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
    let simulation_version = SampleGame::version();

    let mut host = Host::<Step<SampleStep>>::new(simulation_version, TickId(42));
    let connection = host.create_connection().expect("should create connection");
    let now = Millis::new(0);

    let mut client = Client::<SampleGame, SampleStep>::new(now);

    //client([0].into());

    communicate(&mut host, &game_state_provider, connection, &mut client);
}
