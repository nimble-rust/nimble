/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_client_connecting::{ClientError, ConnectingClient};
use nimble_protocol::prelude::*;
use nimble_protocol::ClientRequestId;

fn create_connecting_client(
    application_version: Option<app_version::Version>,
    nimble_version: Option<Version>,
) -> ConnectingClient {
    let app_version = application_version.unwrap_or(app_version::Version {
        major: 1,
        minor: 0,
        patch: 0,
    });
    let nimble_ver = nimble_version.unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 5,
    });
    ConnectingClient::new(ClientRequestId(42), &app_version, nimble_ver)
}

#[test_log::test]
fn test_send_connect_command() {
    let mut client = create_connecting_client(None, None);
    let commands = client.send();

    let ClientToHostOobCommands::ConnectType(connect_cmd) = &commands;
    assert_eq!(
        connect_cmd.application_version,
        Version {
            major: 1,
            minor: 0,
            patch: 0
        }
    );
    assert_eq!(
        connect_cmd.nimble_version,
        Version {
            major: 0,
            minor: 0,
            patch: 5
        }
    );
    assert_eq!(connect_cmd.use_debug_stream, false);
    assert_eq!(
        connect_cmd.client_request_id,
        client.debug_client_request_id()
    );
}

#[test_log::test]
fn receive_valid_connection_accepted() {
    let mut client = create_connecting_client(None, None);
    let response_nonce = client.debug_client_request_id();

    let accepted = ConnectionAccepted {
        flags: 0,
        response_to_request: response_nonce,
    };
    let command = HostToClientOobCommands::ConnectType(accepted);

    let _ = client.send(); // Just make it send once so it can try to accept the connection accepted

    let result = client.receive(&command);

    assert!(result.is_ok());
    assert!(client.is_connected());
}

#[test_log::test]
fn receive_invalid_connection_accepted_nonce() {
    let mut client = create_connecting_client(None, None);
    let wrong_request_id = ClientRequestId(99);
    let accepted = ConnectionAccepted {
        flags: 0,
        response_to_request: wrong_request_id,
    };
    let command = HostToClientOobCommands::ConnectType(accepted);

    let _ = client.send(); // Just make it send once so it can try to accept the connection accepted

    let result = client.receive(&command);

    match result {
        Err(ClientError::WrongConnectResponseRequestId(n)) => {
            assert_eq!(n, wrong_request_id);
        }
        _ => panic!("Expected WrongConnectResponseNonce error"),
    }
}

#[test_log::test]
fn receive_response_without_request() {
    let mut client = create_connecting_client(None, None);
    let wrong_request_id = ClientRequestId(99);
    let accepted = ConnectionAccepted {
        flags: 0,
        response_to_request: wrong_request_id,
    };
    let command = HostToClientOobCommands::ConnectType(accepted);

    let result = client.receive(&command);

    match result {
        Err(ClientError::ReceivedConnectResponseWithoutRequest) => {}
        _ => panic!("Expected WrongConnectResponseNonce error"),
    }
}
