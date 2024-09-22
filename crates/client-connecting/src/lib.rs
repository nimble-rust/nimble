/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use log::info;
use nimble_protocol::prelude::*;
use nimble_protocol::ClientRequestId;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum ClientError {
    WrongConnectResponseRequestId(ClientRequestId),
    ReceivedConnectResponseWithoutRequest,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "client_error {:?}", self)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ConnectedInfo {
    pub session_connection_secret: SessionConnectionSecret,
}

#[derive(Debug, PartialEq)]
pub struct ConnectingClient {
    client_request_id: ClientRequestId,
    application_version: Version,
    nimble_version: Version,
    connected_info: Option<ConnectedInfo>,
    sent_at_least_once: bool,
}

impl ConnectingClient {
    #[must_use]
    pub const fn new(
        client_request_id: ClientRequestId,
        application_version: Version,
        nimble_version: Version,
    ) -> Self {
        Self {
            application_version,
            nimble_version,
            client_request_id,
            connected_info: None,
            sent_at_least_once: false,
        }
    }

    #[must_use]
    pub fn send(&mut self) -> ClientToHostOobCommands {
        let connect_cmd = ConnectRequest {
            nimble_version: self.nimble_version,
            use_debug_stream: false,
            application_version: self.application_version,
            client_request_id: self.client_request_id,
        };

        self.sent_at_least_once = true;

        ClientToHostOobCommands::ConnectType(connect_cmd)
    }

    fn on_connect(&mut self, cmd: &ConnectionAccepted) -> Result<(), ClientError> {
        if !self.sent_at_least_once {
            Err(ClientError::ReceivedConnectResponseWithoutRequest)?
        }

        if cmd.response_to_request != self.client_request_id {
            Err(ClientError::WrongConnectResponseRequestId(
                cmd.response_to_request,
            ))?
        }
        self.connected_info = Some(ConnectedInfo {
            session_connection_secret: cmd.host_assigned_connection_secret,
        });
        info!(
            "connected: session_secret: {:?}",
            self.connected_info.unwrap()
        );
        Ok(())
    }

    pub fn receive(&mut self, command: &HostToClientOobCommands) -> Result<(), ClientError> {
        match command {
            HostToClientOobCommands::ConnectType(connect_command) => {
                self.on_connect(connect_command)
            }
        }
    }

    pub fn debug_client_request_id(&self) -> ClientRequestId {
        self.client_request_id
    }

    pub fn connected_info(&self) -> &Option<ConnectedInfo> {
        &self.connected_info
    }
}
