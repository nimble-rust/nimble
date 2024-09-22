/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{Deserialize, Serialize};
use log::trace;
use nimble_client_logic::logic::ClientLogic;
use nimble_host::logic::{ConnectionId, HostLogic};
use nimble_host::state::State;
use nimble_protocol::client_to_host::{AuthoritativeStep, JoinGameType};
use nimble_protocol::client_to_host::{JoinPlayerRequest, JoinPlayerRequests};
use nimble_protocol::prelude::*;
use nimble_protocol::ClientRequestId;
use nimble_sample_step::{SampleGame, SampleStep};
use nimble_steps::Step;
use std::fmt::Debug;
use std::time::Instant;
use tick_id::TickId;

mod types;

fn communicate<
    SampleGame: nimble_seer::SeerCallback<AuthoritativeStep<Step<SampleStep>>>
        + nimble_assent::AssentCallback<AuthoritativeStep<Step<SampleStep>>>
        + nimble_rectify::RectifyCallback
        + Clone
        + Deserialize
        + Serialize,
    SampleStep: Clone + Deserialize + Debug + Eq + PartialEq,
>(
    host: &mut HostLogic<Step<SampleStep>>,
    connection_id: ConnectionId,
    client: &mut ClientLogic<SampleGame, Step<SampleStep>>,
) where
    SampleStep: Serialize,
{
    let now = Instant::now();

    let to_host = client.send();
    for cmd in &to_host {
        trace!("client >> host: {cmd:?}");
    }
    let to_client: Vec<_> = to_host
        .iter()
        .flat_map(|to_host| {
            host.update(connection_id, now, to_host)
                .expect("should work in test")
        })
        .collect();

    for cmd in &to_client {
        trace!("client << host: {cmd:?}");
    }

    client
        .receive(to_client.as_slice())
        .expect("TODO: panic message");
}

#[test_log::test]
fn client_host_integration() {
    let game = SampleGame::default();
    let state_octets = game
        .authoritative_octets()
        .expect("expect it possible to get state");
    let state = State::new(TickId(42), state_octets.as_slice());
    let mut host = HostLogic::<Step<SampleStep>>::new(state);
    let connection = host.create_connection().expect("should create connection");

    let mut client = ClientLogic::<SampleGame, Step<SampleStep>>::new();
    let joining_player = JoinPlayerRequest { local_index: 0 };

    let join_game_request = JoinGameRequest {
        client_request_id: ClientRequestId(0),
        join_game_type: JoinGameType::NoSecret,
        player_requests: JoinPlayerRequests {
            players: vec![joining_player],
        },
    };
    client.set_joining_player(join_game_request);

    communicate(&mut host, connection, &mut client);
}
