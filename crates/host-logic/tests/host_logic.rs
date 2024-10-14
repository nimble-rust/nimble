/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::test_types::TestStateProvider;
use app_version::Version;
use log::debug;
use monotonic_time_rs::Millis;
use nimble_blob_stream::in_logic_front::FrontLogic;
use nimble_blob_stream::prelude::{ReceiverToSenderFrontCommands, SenderToReceiverFrontCommands};
use nimble_host_logic::HostLogic;
use nimble_protocol::client_to_host::{ConnectRequest, DownloadGameStateRequest};
use nimble_protocol::prelude::{ClientToHostCommands, HostToClientCommands};
use nimble_protocol::ClientRequestId;
use nimble_sample_step::SampleStep;
use tick_id::TickId;

mod test_types;

#[test_log::test]
fn game_state_download() {
    const TICK_ID: TickId = TickId(42);
    const EXPECTED_PAYLOAD: &[u8] = &[0xff, 0x33];
    let state = TestStateProvider {
        tick_id: TICK_ID,
        payload: EXPECTED_PAYLOAD.to_vec(),
    };
    let version = Version::new(0, 1, 2);
    let mut host = HostLogic::<SampleStep>::new(TICK_ID, version);

    let connection_id = host.create_connection().expect("it should work");
    assert_eq!(connection_id.0, 0);
    let now = Millis::from(0);

    let connect_request = ConnectRequest {
        nimble_version: nimble_protocol::Version {
            major: 0,
            minor: 0,
            patch: 0,
        },
        use_debug_stream: false,
        application_version: nimble_protocol::Version {
            major: version.major(),
            minor: version.minor(),
            patch: version.patch(),
        },
        client_request_id: ClientRequestId(0),
    };

    host.update(
        connection_id,
        now,
        &ClientToHostCommands::ConnectType(connect_request),
        &state,
    )
    .expect("it should work");

    // Send a Download Game State request to the host.
    // This is usually done by the client, but we do it manually here.
    let download_request = DownloadGameStateRequest { request_id: 99 };
    let answers = host
        .update(
            connection_id,
            now,
            &ClientToHostCommands::DownloadGameState(download_request.clone()),
            &state,
        )
        .expect("Should download game state");

    debug!("{:?}", answers);

    assert_eq!(answers.len(), 2); // Download Game State Response and a Start Transfer

    debug!(
        "first answer (should be DownloadGameState response): {:?}",
        answers[0]
    );

    // Validate the DownloadGameState response
    let download_game_state_response = match &answers[0] {
        HostToClientCommands::DownloadGameState(response) => response,
        _ => panic!("Unexpected answer: expected DownloadGameState"),
    };
    assert_eq!(download_game_state_response.tick_id.0, TICK_ID.0);
    assert_eq!(
        download_game_state_response.client_request,
        download_request.request_id
    );
    assert_eq!(download_game_state_response.blob_stream_channel, 1);

    // Validate the StartTransfer response
    debug!(
        "second answer (should be StartTransfer response): {:?}",
        answers[1]
    );

    let start_transfer_data = match &answers[1] {
        HostToClientCommands::BlobStreamChannel(response) => match response {
            SenderToReceiverFrontCommands::StartTransfer(start_transfer_data) => {
                start_transfer_data
            }
            _ => panic!("Unexpected answer: expected SenderToReceiverFrontCommands"),
        },
        _ => panic!("Unexpected answer: expected BlobStreamChannel with Start Transfer"),
    };

    assert_eq!(start_transfer_data.transfer_id, 1);

    let mut in_stream = FrontLogic::new();

    // The client receives the Start Transfer from the host
    // and returns a ReceiverToSenderFrontCommands::AckStart.
    in_stream
        .receive(&SenderToReceiverFrontCommands::StartTransfer(
            start_transfer_data.clone(),
        ))
        .expect("Should start transfer");

    let probably_start_acks = in_stream.send().expect("should work to send");

    // The host receives the AckStart
    // and returns a number of BlobStreamChannel(SetChunk).
    let probably_set_chunks = host
        .update(
            connection_id,
            now,
            &ClientToHostCommands::BlobStreamChannel(probably_start_acks),
            &state,
        )
        .expect("Should download game state");

    // Extract SetChunk from BlobStreamChannel.
    let first_set_converted_chunks = probably_set_chunks
        .iter()
        .map(|x| match x {
            HostToClientCommands::BlobStreamChannel(sender_to_receiver) => match sender_to_receiver
            {
                SenderToReceiverFrontCommands::SetChunk(start_transfer_data) => start_transfer_data,
                _ => panic!(
                    "Unexpected sender to receiver {:?}",
                    &probably_set_chunks[0]
                ),
            },
            _ => panic!("Unexpected answer: expected BlobStreamChannel"),
        })
        .collect::<Vec<_>>();

    // Process SetChunks
    let last_ack = {
        let mut ack: Option<ReceiverToSenderFrontCommands> = None;

        for x in first_set_converted_chunks {
            debug!("should be SetChunkFrontData: {:?}", x);
            in_stream
                .receive(&SenderToReceiverFrontCommands::SetChunk(x.clone()))
                .expect("should handle start transfer");

            let resp = in_stream.send().expect("should work to send");
            ack = Some(resp);
        }
        ack
    };
    assert!(last_ack.is_some());

    // Ensure the in_stream ("client") has fully received the blob.
    // Verify that the host is aware the client has received the entire blob.
    assert_eq!(
        in_stream.blob().expect("blob should be ready here"),
        EXPECTED_PAYLOAD
    );

    host.update(
        connection_id,
        now,
        &ClientToHostCommands::BlobStreamChannel(last_ack.unwrap()),
        &state,
    )
    .expect("Should download game state");

    assert!(host
        .get(connection_id)
        .as_ref()
        .expect("connection should exist")
        .is_state_received_by_remote());

    host.destroy_connection(connection_id)
        .expect("Should destroy connection");
}
