/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use log::debug;
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

#[derive(Debug, PartialEq)]
pub struct ConnectingClient {
    client_request_id: ClientRequestId,
    application_version: app_version::Version,
    nimble_version: Version,
    sent_at_least_once: bool,
    is_connected: bool,
}

impl ConnectingClient {
    #[must_use]
    pub const fn new(
        client_request_id: ClientRequestId,
        application_version: &app_version::Version,
        nimble_version: Version,
    ) -> Self {
        Self {
            application_version: *application_version,
            nimble_version,
            client_request_id,
            is_connected: false,
            sent_at_least_once: false,
        }
    }

    #[must_use]
    pub fn send(&mut self) -> ClientToHostOobCommands {
        let connect_cmd = ConnectRequest {
            nimble_version: self.nimble_version,
            use_debug_stream: false,
            application_version: Version {
                major: self.application_version.major,
                minor: self.application_version.minor,
                patch: self.application_version.patch,
            },
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
        self.is_connected = true;
        debug!("set phase to connected!");
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

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}
