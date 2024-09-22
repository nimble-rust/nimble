/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use hexify::assert_eq_slices;
use log::info;
use nimble_client::client::{ClientPhase, ClientStream};
use nimble_protocol::Version;
use nimble_sample_step::{SampleGame, SampleStep};
use nimble_steps::Step;
use std::io;

#[test_log::test]
#[rustfmt::skip]
fn connect_stream() -> io::Result<()> {
    let application_version = Version {
        major: 0,
        minor: 1,
        patch: 2,
    };

    let mut stream: ClientStream<SampleGame, Step<SampleStep>> =
        ClientStream::new(&application_version);

    let octet_vector = stream.send()?;
    assert_eq!(octet_vector.len(), 1);

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

    let connect_response_from_host = [
        // Header
        0x00, 0x00, // Datagram sequence
        0x00, 0x00, // Client Time

        // OOB Commands
        0x0D, // Connect Response
        0x00, // Flags
        // Client Request ID.
        0x00,
        // Connection Secret
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    ];

    stream.receive(&connect_response_from_host)?;

    // Verify
    let phase = stream.debug_phase();

    info!("phase {phase:?}");

    assert!(matches!(phase, &ClientPhase::Connected(_)));

    let connected_info = stream
        .debug_connect_info()
        .expect("connect info should be available when connected");

    assert_eq!(
        connected_info.session_connection_secret.value,
        0x0001020304050607
    );

    let datagrams_request_download_state = stream.send()?;
    assert_eq!(datagrams_request_download_state.len(), 1);
    let datagram_request_download_state = &datagrams_request_download_state[0];

    let expected_request_download_state_octets = &[
        0x00, 0x01, // Ordered datagram Sequence number
        0x00, 0x00,  // Client Time
        0x03, // Download Game State
        0x99, // Download Request id, //TODO: Hardcoded, but should not be
    ];
    assert_eq_slices(
        datagram_request_download_state,
        expected_request_download_state_octets
    );


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
    
    let hopefully_ack_blob = stream.send()?;

    let expected_ack_blob_stream = &[
        // Header
        0x00, 0x03, // Sequence
        0x00, 0x00, // Client Time
        
        // Commands
        0x04, // BlobStream client to host
        0x02, // AckChunk
        0x00, 0x00, // Transfer ID
        0x00, 0x00, 0x00, 0x01, // Waiting for this chunk index
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Receive Mask
    ];
    
    assert_eq_slices(&hopefully_ack_blob[0], expected_ack_blob_stream);
    
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
