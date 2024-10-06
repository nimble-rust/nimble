/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use nimble_client_logic::err::ClientError;
use nimble_client_logic::logic::ClientLogic;
use nimble_participant::ParticipantId;
use nimble_protocol::client_to_host::DownloadGameStateRequest;
use nimble_protocol::host_to_client::{
    AuthoritativeStepRanges, GameStepResponse, GameStepResponseHeader,
};
use nimble_protocol::prelude::{ClientToHostCommands, CombinedSteps, HostToClientCommands};
use nimble_sample_step::{SampleState, SampleStep};
use nimble_step_types::StepForParticipants;
use nimble_steps::Step::{Custom, Forced};
use nimble_steps::{Step, StepInfo, StepsError};
use seq_map::SeqMap;
use std::fmt::Debug;
use tick_id::TickId;

#[test_log::test]
fn basic_logic() {
    let mut client_logic = ClientLogic::<SampleState, Step<SampleStep>>::new();

    {
        let commands = client_logic.send();
        assert_eq!(commands.len(), 1);
        if let ClientToHostCommands::DownloadGameState(DownloadGameStateRequest { request_id }) =
            &commands[0]
        {
            assert_eq!(*request_id, 153);
        } else {
            panic!("Command did not match expected structure or pattern");
        }
    }
}

fn setup_logic<StateT: BufferDeserializer, StepT: Clone + Deserialize + Serialize + Debug>(
) -> ClientLogic<StateT, StepT> {
    ClientLogic::<StateT, StepT>::new()
}

#[test_log::test]
fn send_steps() -> Result<(), StepsError> {
    let mut client_logic = setup_logic::<SampleState, Step<SampleStep>>();

    client_logic.push_predicted_step(
        TickId(0),
        StepForParticipants {
            combined_step: [(ParticipantId(0), Custom(SampleStep::MoveRight(3)))]
                .as_slice()
                .into(),
        },
    )?;

    {
        let commands = client_logic.send();
        assert_eq!(commands.len(), 1);
        if let ClientToHostCommands::DownloadGameState(DownloadGameStateRequest { request_id }) =
            &commands[0]
        {
            assert_eq!(*request_id, 153);
        } else {
            panic!("Command did not match expected structure or pattern");
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

    let mut auth_steps = Vec::<StepForParticipants<Step<SampleStep>>>::new();
    for index in 0..3 {
        let mut hash_map = SeqMap::<ParticipantId, Step<SampleStep>>::new();
        hash_map
            .insert(first_participant_id, first_steps[index].clone())
            .expect("first participant should be unique");
        hash_map
            .insert(second_participant_id, second_steps[index].clone())
            .expect("second_participant should be unique");
        auth_steps.push(StepForParticipants {
            combined_step: hash_map,
        });
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
fn receive_authoritative_steps() -> Result<(), ClientError> {
    let mut client_logic = setup_logic::<SampleState, Step<SampleStep>>();

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

    // Receive
    client_logic.receive(&[command])?;

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

    let mut expected_hash_map = SeqMap::<ParticipantId, Step<SampleStep>>::new();
    expected_hash_map
        .insert(first_participant_id, Custom(SampleStep::MoveLeft(-10)))
        .expect("should be unique");
    expected_hash_map
        .insert(second_participant_id, Forced)
        .expect("should be unique");

    let expected_step = StepForParticipants::<Step<SampleStep>> {
        combined_step: expected_hash_map,
    };

    let expected_step_with_step_info = StepInfo::<StepForParticipants<Step<SampleStep>>> {
        step: expected_step,
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
    client_logic.receive(&[command2])?;

    assert_eq!(client_logic.server_buffer_count(), None);
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
    client_logic.receive(&[command3])?;

    assert_eq!(client_logic.server_buffer_count().expect("should work"), 4);
    assert_eq!(
        client_logic
            .server_buffer_delta_ticks()
            .expect("should work"),
        -3
    );

    Ok(())
}
