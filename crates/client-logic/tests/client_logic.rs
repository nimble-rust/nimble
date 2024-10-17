/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use monotonic_time_rs::Millis;
use nimble_client_logic::err::ClientLogicError;
use nimble_client_logic::{ClientLogic, ClientLogicPhase};
use nimble_participant::ParticipantId;
use nimble_protocol::client_to_host::{ConnectRequest, DownloadGameStateRequest};
use nimble_protocol::host_to_client::{
    AuthoritativeStepRanges, ConnectionAccepted, GameStepResponse, GameStepResponseHeader,
};
use nimble_protocol::prelude::{ClientToHostCommands, CombinedSteps, HostToClientCommands};
use nimble_sample_step::{SampleState, SampleStep};
use nimble_step::Step;
use nimble_step::Step::{Custom, Forced};
use nimble_step_map::StepMap;
use std::fmt::Debug;
use tick_id::TickId;
use tick_queue::ItemInfo;

#[test_log::test]
fn basic_logic() {
    let simulation_version = app_version::Version::new(0, 1, 2);
    let mut client_logic = ClientLogic::<SampleState, Step<SampleStep>>::new(simulation_version);

    feed_connect_response(&mut client_logic);
    {
        let now = Millis::new(0);
        let commands = client_logic.send(now);
        assert_eq!(commands.len(), 2);
        if let ClientToHostCommands::DownloadGameState(DownloadGameStateRequest { request_id }) =
            &commands[1]
        {
            assert_eq!(*request_id, 153);
        } else {
            panic!("Command did not match expected structure or pattern");
        }
    }
}

fn feed_connect_response(client_logic: &mut ClientLogic<SampleState, Step<SampleStep>>) {
    let now = Millis::new(0);

    let commands = client_logic.send(now);
    assert_eq!(commands.len(), 1);

    if let ClientToHostCommands::ConnectType(ConnectRequest {
        client_request_id, ..
    }) = commands[0]
    {
        let connect_response = ConnectionAccepted {
            flags: 0,
            response_to_request: client_request_id,
        };

        client_logic
            .receive(now, &HostToClientCommands::ConnectType(connect_response))
            .expect("TODO: panic message");
    } else {
        panic!("Command should be connect request");
    }
}

fn setup_logic<
    StateT: BufferDeserializer,
    StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
>() -> ClientLogic<StateT, StepT> {
    let simulation_version = app_version::Version::new(0, 1, 2);
    ClientLogic::<StateT, StepT>::new(simulation_version)
}

#[test_log::test]
fn send_steps() -> Result<(), ClientLogicError> {
    let mut client_logic = setup_logic::<SampleState, Step<SampleStep>>();

    client_logic.push_predicted_step(
        TickId(0),
        [(ParticipantId(0), Custom(SampleStep::MoveRight(3)))]
            .as_slice()
            .into(),
    )?;

    feed_connect_response(&mut client_logic);

    {
        let now = Millis::new(0);

        let commands = client_logic.send(now);
        assert_eq!(commands.len(), 2);
        if let ClientToHostCommands::DownloadGameState(DownloadGameStateRequest { request_id }) =
            &commands[1]
        {
            assert_eq!(*request_id, 0x99);
        } else {
            panic!(
                "Command did not match expected structure or pattern {}",
                commands[0]
            );
        }
    }

    Ok(())
}

fn setup_sample_steps() -> AuthoritativeStepRanges<Step<SampleStep>> {
    let first_steps = vec![
        Custom(SampleStep::Jump),
        Custom(SampleStep::MoveLeft(-10)),
        Custom(SampleStep::MoveRight(32000)),
    ];
    let first_participant_id = ParticipantId(255);

    let second_steps = vec![
        Custom(SampleStep::MoveLeft(42)),
        Forced,
        Custom(SampleStep::Jump),
    ];
    let second_participant_id = ParticipantId(1);

    let mut auth_steps = Vec::<StepMap<Step<SampleStep>>>::new();
    for index in 0..3 {
        let mut hash_map = StepMap::<Step<SampleStep>>::new();
        hash_map
            .insert(first_participant_id, first_steps[index].clone())
            .expect("first participant should be unique");
        hash_map
            .insert(second_participant_id, second_steps[index].clone())
            .expect("second_participant should be unique");
        auth_steps.push(hash_map);
    }

    const EXPECTED_TICK_ID: TickId = TickId(0);
    let range_to_send = CombinedSteps::<Step<SampleStep>> {
        tick_id: EXPECTED_TICK_ID,
        steps: auth_steps,
    };

    let ranges_to_send = AuthoritativeStepRanges {
        ranges: vec![range_to_send],
    };

    ranges_to_send
}
#[test_log::test]
fn receive_authoritative_steps() -> Result<(), ClientLogicError> {
    let mut client_logic = setup_logic::<SampleState, SampleStep>();

    // Create a GameStep command
    let response = GameStepResponse::<Step<SampleStep>> {
        response_header: GameStepResponseHeader {
            // We ignore the response for now
            connection_buffer_count: 2,
            delta_buffer: -2,
            next_expected_tick_id: TickId(0),
        },
        authoritative_steps: setup_sample_steps(),
    };
    let command = HostToClientCommands::GameStep(response);
    let now = Millis::new(0);

    // Receive
    client_logic.receive(now, &command)?;

    // Verify
    let authoritative_steps = client_logic.debug_authoritative_steps();
    assert_eq!(
        authoritative_steps
            .back_tick_id()
            .expect("should have end_tick_id by now"),
        TickId(2)
    ); // Should have received TickId 0, 1, and 2.

    let first_participant_id = ParticipantId(255);
    let second_participant_id = ParticipantId(1);

    let mut expected_hash_map = StepMap::<Step<SampleStep>>::new();
    expected_hash_map
        .insert(first_participant_id, Custom(SampleStep::MoveLeft(-10)))
        .expect("should be unique");
    expected_hash_map
        .insert(second_participant_id, Forced)
        .expect("should be unique");

    let expected_step = expected_hash_map;

    let expected_step_with_step_info = ItemInfo::<StepMap<Step<SampleStep>>> {
        item: expected_step,
        tick_id: TickId(1),
    };

    let auth_steps = authoritative_steps
        .debug_get(1)
        .expect("index 1 should exist");
    assert_eq!(authoritative_steps.len(), 3);

    assert_eq!(*auth_steps, expected_step_with_step_info);

    // Create a GameStep command
    let response2 = GameStepResponse::<Step<SampleStep>> {
        response_header: GameStepResponseHeader {
            // We ignore the response for now
            connection_buffer_count: 2,
            delta_buffer: -3,
            next_expected_tick_id: TickId(0),
        },
        authoritative_steps: setup_sample_steps(),
    };
    let command2 = HostToClientCommands::GameStep(response2);
    // Receive
    client_logic.receive(now, &command2)?;

    assert_eq!(client_logic.server_buffer_delta_ticks(), None);

    // Create a GameStep command
    let response3 = GameStepResponse::<Step<SampleStep>> {
        response_header: GameStepResponseHeader {
            // We ignore the response for now
            connection_buffer_count: 8,
            delta_buffer: -4,
            next_expected_tick_id: TickId(0),
        },
        authoritative_steps: setup_sample_steps(),
    };
    let command3 = HostToClientCommands::GameStep(response3);
    client_logic.receive(now, &command3)?;

    assert_eq!(
        client_logic
            .server_buffer_delta_ticks()
            .expect("should work"),
        -3
    );

    Ok(())
}

fn create_connecting_client(
    simulation_version: Option<app_version::Version>,
) -> ClientLogic<SampleState, SampleStep> {
    let simulation_version = simulation_version.unwrap_or(app_version::Version::new(1, 0, 0));
    let mut client = ClientLogic::<SampleState, SampleStep>::new(simulation_version);
    let now = Millis::new(0);

    let _ = client.send(now);
    client
}

#[test_log::test]
fn send_connect_command() {
    let mut client = create_connecting_client(None);
    let now = Millis::new(0);

    let commands = client.send(now);

    let ClientToHostCommands::ConnectType(connect_cmd) = &commands[0] else {
        panic!("Wrong command")
    };
    assert_eq!(
        connect_cmd.application_version,
        nimble_protocol::Version {
            major: 1,
            minor: 0,
            patch: 0
        }
    );
    assert_eq!(
        connect_cmd.nimble_version,
        nimble_protocol::Version {
            major: 0,
            minor: 0,
            patch: 5
        }
    );
    assert_eq!(connect_cmd.use_debug_stream, false);
    assert_eq!(
        connect_cmd.client_request_id,
        client
            .debug_connect_request_id()
            .expect("connect request id not set")
    );
}

#[test_log::test]
fn receive_valid_connection_accepted() {
    let mut client = create_connecting_client(None);
    let response_nonce = client
        .debug_connect_request_id()
        .expect("connect request id not set");

    let accepted = ConnectionAccepted {
        flags: 0,
        response_to_request: response_nonce,
    };
    let command = HostToClientCommands::<Step<SampleStep>>::ConnectType(accepted);

    let now = Millis::new(0);

    let _ = client.send(now); // Just make it send once so it can try to accept the connection accepted

    let result = client.receive(now, &command);

    assert!(result.is_ok());
    assert_eq!(
        client.phase(),
        &ClientLogicPhase::RequestDownloadState {
            download_state_request_id: 0x99
        }
    );
}

#[test_log::test]
fn receive_invalid_connection_accepted_nonce() {
    let mut client = create_connecting_client(None);
    let wrong_request_id = nimble_protocol::ClientRequestId(99);
    let accepted = ConnectionAccepted {
        flags: 0,
        response_to_request: wrong_request_id,
    };
    let command = HostToClientCommands::<Step<SampleStep>>::ConnectType(accepted);
    let now = Millis::new(0);

    let _ = client.send(now); // Just make it send once so it can try to accept the connection accepted

    let result = client.receive(now, &command);

    match result {
        Err(ClientLogicError::WrongConnectResponseRequestId(n)) => {
            assert_eq!(n, wrong_request_id);
        }
        _ => panic!("Expected WrongConnectResponseNonce error"),
    }
}

#[test_log::test]
fn receive_response_without_request() {
    let mut client = create_connecting_client(None);
    let wrong_request_id = nimble_protocol::ClientRequestId(99);
    let accepted = ConnectionAccepted {
        flags: 0,
        response_to_request: wrong_request_id,
    };
    let command = HostToClientCommands::<Step<SampleStep>>::ConnectType(accepted);
    let now = Millis::new(0);

    let result = client.receive(now, &command);

    match result {
        Err(ClientLogicError::WrongConnectResponseRequestId(_)) => {}
        _ => panic!("Expected WrongConnectResponseNonce error {result:?}"),
    }
}
