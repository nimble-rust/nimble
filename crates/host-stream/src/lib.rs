/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram_chunker::{serialize_to_chunker, DatagramChunkerError};
use flood_rs::prelude::InOctetStream;
use flood_rs::{Deserialize, Serialize};
use monotonic_time_rs::Millis;
use nimble_host_logic::logic::{
    GameSession, GameStateProvider, HostConnectionId, HostLogic, HostLogicError,
};
use nimble_protocol::host_to_client_oob::ConnectionAccepted;
use nimble_protocol::prelude::{
    ClientToHostCommands, ClientToHostOobCommands, HostToClientOobCommands,
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io;
use tick_id::TickId;

#[derive(Debug)]
pub enum HostStreamError {
    DatagramChunkerError(DatagramChunkerError),
    HostLogicError(HostLogicError),
    IoError(io::Error),
    ConnectionNotFound,
    WrongApplicationVersion,
}

impl From<DatagramChunkerError> for HostStreamError {
    fn from(e: DatagramChunkerError) -> Self {
        Self::DatagramChunkerError(e)
    }
}

impl From<HostLogicError> for HostStreamError {
    fn from(e: HostLogicError) -> Self {
        Self::HostLogicError(e)
    }
}

impl From<io::Error> for HostStreamError {
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}

#[derive(Debug, PartialEq)]
pub enum HostStreamConnectionPhase {
    Connecting,
    Connected,
}

pub struct HostStreamConnection {
    phase: HostStreamConnectionPhase,
}

impl HostStreamConnection {
    pub fn new() -> Self {
        Self {
            phase: HostStreamConnectionPhase::Connecting,
        }
    }

    pub fn phase(&self) -> &HostStreamConnectionPhase {
        &self.phase
    }
}

pub struct HostStream<
    StepT: Clone + std::fmt::Debug + std::cmp::Eq + flood_rs::Deserialize + flood_rs::Serialize,
> {
    host_logic: HostLogic<StepT>,
    connections: HashMap<u8, HostStreamConnection>,
    application_version: app_version::Version,
}

impl<StepT: Clone + Eq + Debug + Deserialize + Serialize> HostStream<StepT> {
    pub fn new(version: &app_version::Version, tick_id: TickId) -> Self {
        Self {
            host_logic: HostLogic::<StepT>::new(tick_id),
            connections: Default::default(),
            application_version: *version,
        }
    }

    pub fn get(&self, connection_id: HostConnectionId) -> Option<&HostStreamConnection> {
        self.connections.get(&connection_id.0)
    }

    const DATAGRAM_MAX_SIZE: usize = 1024;

    pub fn update(
        &mut self,
        connection_id: HostConnectionId,
        now: Millis,
        datagram: &[u8],
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<Vec<u8>>, HostStreamError> {
        let mut in_stream = InOctetStream::new(datagram);

        if let Some(ref mut connection) = self.connections.get_mut(&connection_id.0) {
            match connection.phase {
                HostStreamConnectionPhase::Connecting => {
                    let request = ClientToHostOobCommands::deserialize(&mut in_stream)?;
                    match request {
                        ClientToHostOobCommands::ConnectType(connect_request) => {
                            let connect_version = app_version::Version {
                                major: connect_request.application_version.major,
                                minor: connect_request.application_version.minor,
                                patch: connect_request.application_version.patch,
                            };
                            if connect_version != self.application_version {
                                return Err(HostStreamError::WrongApplicationVersion);
                            }
                            connection.phase = HostStreamConnectionPhase::Connected;

                            let response = ConnectionAccepted {
                                flags: 0,
                                response_to_request: connect_request.client_request_id,
                            };

                            let commands = [HostToClientOobCommands::ConnectType(response)];
                            Ok(serialize_to_chunker(commands, Self::DATAGRAM_MAX_SIZE)?)
                        }
                    }
                }
                HostStreamConnectionPhase::Connected => {
                    let request = ClientToHostCommands::<StepT>::deserialize(&mut in_stream)?;
                    let commands =
                        self.host_logic
                            .update(connection_id, now, &request, state_provider)?;
                    Ok(serialize_to_chunker(commands, Self::DATAGRAM_MAX_SIZE)?)
                }
            }
        } else {
            Err(HostStreamError::ConnectionNotFound)
        }
    }

    pub fn create_connection(&mut self) -> Option<HostConnectionId> {
        if let Some(connection_id) = self.host_logic.create_connection() {
            self.connections
                .insert(connection_id.0, HostStreamConnection::new());
            Some(connection_id)
        } else {
            None
        }
    }

    pub fn session(&self) -> &GameSession {
        self.host_logic.session()
    }
}
