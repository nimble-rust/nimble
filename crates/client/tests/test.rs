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
use nimble_step_types::{LocalIndex, PredictedStep};
use nimble_steps::Step;
use std::collections::HashMap;
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

    /* TODO
    let expected_steps_request_octets = &[
        EXPECTED_CONNECTION_ID,
        0x1A,
        0x93,
        0x76,
        0x47, // HASH
        0x00,
        0x01,
        0,
        0,    //?
        0x02, // Steps Request
        // Steps Ack
        0x00,
        0x00,
        0x00,
        0x00, // Waiting for this tick ID
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00, // Receive mask
        // Predicted Steps
        0x00, // Number of local participants
    ];

    assert_eq!(only_datagram, expected_steps_request_octets);
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

    let mut predicted_players = HashMap::<LocalIndex, Step<SampleStep>>::new();
    predicted_players.insert(1, Step::Custom(SampleStep::Jump));
    let predicted_step_for_local_players = PredictedStep { predicted_players };
    stream.push_predicted_step(TickId::new(0), predicted_step_for_local_players)?;

    let mut predicted_players2 = HashMap::<LocalIndex, Step<SampleStep>>::new();
    predicted_players2.insert(1, Step::Custom(SampleStep::MoveLeft(-10)));
    let predicted_step_for_local_players2 = PredictedStep {
        predicted_players: predicted_players2,
    };
    stream.push_predicted_step(TickId::new(1), predicted_step_for_local_players2)?;

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
        0x01, // number of player streams following
        0x01, // Player Index
        0x00, 0x00, 0x00, 0x00, // Start TickId

        0x02, // Predicted Step Count following

        // Predicted Steps
        0x05, // Step::Custom
        0x03, // SampleStep::Jump

        0x05, // Step::Custom
        0x01, // SampleStep::Move Left
        0xFF, 0xF6, // FFF6 = -10 (16-bit two’s complement notation)
    ];

    assert_eq_slices(&probably_one_predicted_step[0], expected_one_predicted_step);

    Ok(())
}
