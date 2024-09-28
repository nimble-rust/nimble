/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::BufferDeserializer;
use hexify::assert_eq_slices;
use log::info;
use nimble_client::client::{ClientPhase, ClientStream, ClientStreamError};
use nimble_protocol::Version;
use nimble_sample_step::{SampleState, SampleStep};
use nimble_step_types::{IndexMap, LocalIndex, PredictedStep};
use nimble_steps::Step;
use std::collections::HashSet;
use tick_id::TickId;

fn connect<
    StateT: BufferDeserializer + std::fmt::Debug,
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
>(
    stream: &mut ClientStream<StateT, Step<StepT>>,
) -> Result<(), ClientStreamError> {
    let octet_vector = stream.send()?;
    assert_eq!(octet_vector.len(), 1);

    #[rustfmt::skip]
    assert_eq!(
        octet_vector[0],
        &[
            // OOB Commands
            0x00, 0x00, // Datagram sequence
            0x00, 0x00, // Client Time

            0x05, // Connect Request: ClientToHostOobCommand::ConnectType = 0x05
            0, 0, 0, 0, 0, 5, // Nimble version
            0, // Flags (use debug stream)
            0, 0, 0, 1, 0, 2, // Application version
            0,  // Client Request Id
        ]
    );

    let phase = stream.debug_phase();

    info!("phase {phase:?}");

    assert!(matches!(phase, &ClientPhase::Connecting(_)));

    #[rustfmt::skip]

    let connect_response_from_host = [
        // Header
        0x00, 0x00, // Datagram sequence
        0x00, 0x00, // Client Time

        // OOB Commands
        0x0D, // Connect Response
        0x00, // Flags
        // Client Request ID.
        0x00,
    ];

    stream.receive(&connect_response_from_host)?;

    // Verify
    let phase = stream.debug_phase();

    info!("phase {phase:?}");

    assert!(matches!(phase, &ClientPhase::Connected(_)));

    Ok(())
}

fn download_state<
    StateT: BufferDeserializer,
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
>(
    stream: &mut ClientStream<StateT, StepT>,
) -> Result<(), ClientStreamError> {
    let datagrams_request_download_state = stream.send()?;
    assert_eq!(datagrams_request_download_state.len(), 1);
    let datagram_request_download_state = &datagrams_request_download_state[0];

    #[rustfmt::skip]
    let expected_request_download_state_octets = &[
        // Header
        0x00, 0x01, // Ordered datagram Sequence number
        0x00, 0x00,  // Client Time

        // Commands
        0x03, // Download Game State
        0x99, // Download Request id, //TODO: Hardcoded, but should not be
    ];
    assert_eq_slices(
        datagram_request_download_state,
        expected_request_download_state_octets,
    );

    #[rustfmt::skip]
    let feed_request_download_response = &[
        // Header
        0x00, 0x01, // Ordered datagram
        0x00, 0x00, // Client Time

        // Commands

        // Download Game State Response Command
        0x0B,
        0x99, // Client Request Id // TODO: Hardcoded but should not be
        0x00, 0x00, 0x00, 0x00, // TickID for state
        0x00, 0x00, // Blob Stream channel to use

        // Blob Stream Channel Command
        0x0C, // Blob Stream channel command
        0x02, // Blob Stream Start Transfer
        0x00, 0x00, // Blob Stream channel to use
        0x00, 0x00, 0x00, 0x08, // Total Octet Size
        0x00, 0x10, // Chunk Size (can not be zero)
    ];

    stream.receive(feed_request_download_response)?;

    let datagrams_request_step = stream.send()?;

    assert_eq!(datagrams_request_step.len(), 1);

    let start_transfer_octets = &datagrams_request_step[0];

    #[rustfmt::skip]
    let expected_start_transfer = &[
        // Header
        0x00, 0x02, // Datagram sequence number
        0x00, 0x00,    // Client Time

        // Commands
        0x04, // blob stream channel
        0x03, // Ack Start. Client acknowledges that the transfer has started
        0x00, 0x00, // Transfer ID
    ];
    assert_eq_slices(start_transfer_octets, expected_start_transfer);

    #[rustfmt::skip]
    let feed_complete_download = &[
        // Header
        0x00, 0x02, // Sequence
        0x00, 0x00, // Client Time

        // Commands
        0x0C, // HostToClient::BlobStreamChannel
        0x01, // Set Chunk
        0x00, 0x00, // Transfer ID
        0x00, 0x00, 0x00, 0x00, // Chunk Index
        0x00, 0x08, // Octets in this chunk. That many octets should follow
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    ];

    stream.receive(feed_complete_download)?;

    Ok(())
}

#[test_log::test]
fn connect_stream() -> Result<(), ClientStreamError> {
    let application_version = Version {
        major: 0,
        minor: 1,
        patch: 2,
    };

    let mut stream: ClientStream<SampleState, Step<SampleStep>> =
        ClientStream::new(&application_version);

    connect(&mut stream)?;

    download_state(&mut stream)?;

    /*
            self.transfer_id.to_stream(stream)?;
    self.data.to_stream(stream)?;
     */

    Ok(())
}

#[test_log::test]
fn predicted_steps() -> Result<(), ClientStreamError> {
    let application_version = Version {
        major: 0,
        minor: 1,
        patch: 2,
    };

    let mut stream: ClientStream<SampleState, Step<SampleStep>> =
        ClientStream::new(&application_version);

    // Client must be connected and have a state before sending predicted steps
    connect(&mut stream)?;
    download_state(&mut stream)?;

    let probably_zero_predicted_steps = stream.send()?;

    #[rustfmt::skip]
    let expected_zero_predicted_steps = &[
        // Header
        0x00, 0x03, // Sequence
        0x00, 0x00, // Client Time

        // Commands
        0x02, // Send Predicted steps
        0x00, 0x00, 0x00, 0x00, // Waiting for Tick ID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Receive Mask for steps
        0x00, // number of player streams following
    ];
    assert_eq_slices(
        &probably_zero_predicted_steps[0],
        expected_zero_predicted_steps,
    );

    let array: &[(LocalIndex, &[Step<SampleStep>])] = &[
        (
            1,
            &[
                Step::Custom(SampleStep::Jump),
                Step::Custom(SampleStep::MoveLeft(-10)),
            ],
        ),
        (2, &[Step::Custom(SampleStep::MoveRight(10))]),
    ];

    let predicted_steps = create_predicted_steps(array);

    let mut tick_id = TickId::new(0);
    for predicted_step in predicted_steps {
        stream.push_predicted_step(tick_id, predicted_step)?;
        tick_id = TickId::new(tick_id.0 + 1);
    }

    let probably_one_predicted_step = stream.send()?;

    #[rustfmt::skip]
    let expected_one_predicted_step = &[
        // Header
        0x00, 0x04, // Sequence
        0x00, 0x00, // Client Time

        // Commands
        0x02, // Send Predicted steps

        // ACK
        0x00, 0x00, 0x00, 0x00, // Waiting for Tick ID
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Receive Mask for steps

        // Predicted steps Header
        0x02, // number of player streams following

        0x01, // Local Player ID
        0x00, 0x00, 0x00, 0x00, // Start TickId

        0x02, // Predicted Step Count following

        // Predicted Steps
        0x05, // Step::Custom
        0x03, // SampleStep::Jump

        0x05, // Step::Custom
        0x01, // SampleStep::Move Left
        0xFF, 0xF6, // FFF6 = -10 (signed 16-bit two’s complement notation)
    
        0x02, // Local Player ID
        0x00, 0x00, 0x00, 0x00, // Start TickId for Local Player 2 (usually the same as for player 1)
        0x01, // Predicted Step Count following
        0x05, // Step::Custom
        0x02, // SampleStep::Move Right
        0x00, 0x0A, // = +10 (signed 16-bit two’s complement notation)
    ];

    assert_eq_slices(&probably_one_predicted_step[0], expected_one_predicted_step);

    Ok(())
}

fn create_predicted_steps<StepT: Clone>(
    predicted_steps_for_all_players: &[(LocalIndex, &[StepT])],
) -> Vec<PredictedStep<StepT>> {
    let unique_indexes: HashSet<u8> = predicted_steps_for_all_players
        .iter()
        .map(|(local_index, _)| *local_index)
        .collect();
    assert_eq!(unique_indexes.len(), predicted_steps_for_all_players.len());

    let longest_steps_vector: usize =
        predicted_steps_for_all_players
            .iter()
            .fold(0, |mut acc, (_, step_vec)| {
                if step_vec.len() > acc {
                    acc = step_vec.len();
                }
                acc
            });

    let mut predicted_steps_vector = Vec::with_capacity(longest_steps_vector);
    for result_index in 0..longest_steps_vector {
        let mut predicted_players: IndexMap<LocalIndex, StepT> = IndexMap::new();
        for (local_index, steps_vector) in predicted_steps_for_all_players.iter() {
            if result_index >= steps_vector.len() {
                continue;
            }

            info!("adding {local_index:?} to predicted_steps");
            predicted_players
                .insert(*local_index as u8, steps_vector[result_index].clone())
                .expect("in the test, it should work to insert");
        }
        predicted_steps_vector.push(PredictedStep { predicted_players });
    }

    predicted_steps_vector
}
