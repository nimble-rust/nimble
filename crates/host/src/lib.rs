/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use datagram_chunker::{DatagramChunker, DatagramChunkerError};
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::prelude::OutOctetStream;
use flood_rs::{Deserialize, Serialize};
use hexify::format_hex;
use log::{debug, trace};
use monotonic_time_rs::Millis;
use nimble_host_logic::{Connection, GameSession, GameStateProvider, HostLogic, HostLogicError};
use nimble_layer::{NimbleLayer, NimbleLayerError};
use nimble_protocol::prelude::ClientToHostCommands;
use std::collections::HashMap;
use std::fmt::Debug;
use tick_id::TickId;

#[derive(Default, Debug)]
pub struct HostConnection {
    layer: NimbleLayer,
}
impl HostConnection {
    pub fn new() -> Self {
        Self {
            layer: Default::default(),
        }
    }
}

#[derive(Debug)]
pub enum HostError {
    ConnectionNotFound(u8),
    IoError(std::io::Error),
    NimbleLayerError(NimbleLayerError),
    HostLogicError(HostLogicError),
    DatagramChunkerError(DatagramChunkerError),
}

impl ErrorLevelProvider for HostError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::IoError(_) => ErrorLevel::Warning,
            Self::ConnectionNotFound(_) => ErrorLevel::Warning,
            Self::NimbleLayerError(_) => ErrorLevel::Warning,
            Self::HostLogicError(err) => err.error_level(),
            Self::DatagramChunkerError(err) => err.error_level(),
        }
    }
}

impl From<DatagramChunkerError> for HostError {
    fn from(err: DatagramChunkerError) -> Self {
        Self::DatagramChunkerError(err)
    }
}

impl From<HostLogicError> for HostError {
    fn from(err: HostLogicError) -> Self {
        Self::HostLogicError(err)
    }
}

impl From<NimbleLayerError> for HostError {
    fn from(e: NimbleLayerError) -> Self {
        Self::NimbleLayerError(e)
    }
}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub struct ConnectionId(u8);

impl ConnectionId {
    pub fn inner(&self) -> u8 {
        self.0
    }
}

pub struct Host<StepT: Clone + Debug + Eq + Deserialize + Serialize + std::fmt::Display> {
    logic: HostLogic<StepT>,
    connections: HashMap<u8, HostConnection>,
}

impl<StepT: Clone + Deserialize + Serialize + Eq + Debug + std::fmt::Display> Host<StepT> {
    pub fn new(app_version: app_version::Version, tick_id: TickId) -> Self {
        Self {
            logic: HostLogic::<StepT>::new(tick_id, app_version),
            connections: HashMap::new(),
        }
    }

    pub fn debug_logic(&self) -> &HostLogic<StepT> {
        &self.logic
    }

    pub fn debug_get_logic(
        &self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Option<&Connection<StepT>> {
        self.logic.get(connection_id)
    }

    pub fn session(&self) -> &GameSession<StepT> {
        self.logic.session()
    }

    pub fn update(
        &mut self,
        connection_id: nimble_host_logic::HostConnectionId,
        now: Millis,
        datagram: &[u8],
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<Vec<u8>>, HostError> {
        trace!(
            "time:{now}: host received for connection:{} payload:\n{}",
            connection_id.0,
            format_hex(datagram)
        );
        //        let mut in_stream = InOctetStream::new(datagram);
        let found_connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostError::ConnectionNotFound(connection_id.0))?;

        let (datagram_without_layer, client_time) = found_connection.layer.receive(datagram)?;

        let deserialized_commands = datagram_chunker::deserialize_datagram::<
            ClientToHostCommands<StepT>,
        >(datagram_without_layer)?;

        let mut all_commands_to_send = Vec::new();
        for deserialized_command in deserialized_commands {
            let commands_to_send =
                self.logic
                    .update(connection_id, now, &deserialized_command, state_provider)?;

            all_commands_to_send.extend(commands_to_send);
        }

        let mut datagram_chunker = DatagramChunker::new(1024);
        for cmd in all_commands_to_send {
            let mut out_stream = OutOctetStream::new();
            cmd.serialize(&mut out_stream)?;
            datagram_chunker.push(out_stream.octets_ref())?;
        }

        let outgoing_datagrams = datagram_chunker.finalize();

        let out_datagrams = found_connection
            .layer
            .send(client_time, outgoing_datagrams)?;

        for (index, datagram) in out_datagrams.iter().enumerate() {
            trace!(
                "host sending index {} payload:\n{}",
                index,
                format_hex(datagram)
            );
        }

        Ok(out_datagrams)
    }

    pub fn get(
        &self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Option<&HostConnection> {
        self.connections.get(&connection_id.0)
    }

    pub fn create_connection(&mut self) -> Option<nimble_host_logic::HostConnectionId> {
        if let Some(connection_id) = self.logic.create_connection() {
            self.connections
                .insert(connection_id.0, HostConnection::new());
            debug!("Created connection {:?}", connection_id);
            Some(connection_id)
        } else {
            None
        }
    }

    pub fn destroy_connection(
        &mut self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Result<(), HostError> {
        debug!("destroying connection {:?}", connection_id);
        self.connections.remove(&connection_id.0);
        self.logic
            .destroy_connection(connection_id)
            .expect("FIX THIS"); // TODO:
        Ok(())
    }
}
