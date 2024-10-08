/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram_chunker::{serialize_to_chunker, DatagramChunkerError};
use flood_rs::prelude::InOctetStream;
use flood_rs::{Deserialize, ReadOctetStream, Serialize};
use hexify::format_hex;
use log::trace;
use monotonic_time_rs::Millis;
use nimble_host_logic::logic::{
    GameSession, GameStateProvider, HostConnectionId, HostLogic, HostLogicError,
};
use nimble_protocol::prelude::{ClientToHostCommands, HostToClientCommands};
use std::fmt::Debug;
use std::io;
use tick_id::TickId;

const DATAGRAM_MAX_SIZE: usize = 1024;

#[derive(Debug)]
pub enum HostStreamError {
    DatagramChunkerError(DatagramChunkerError),
    HostLogicError(HostLogicError),
    IoError(io::Error),
    ConnectionNotFound,
    WrongApplicationVersion,
    MustBeConnectedFirst,
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

pub struct HostStream<
    StepT: Clone + std::fmt::Debug + std::cmp::Eq + flood_rs::Deserialize + flood_rs::Serialize,
> {
    host_logic: HostLogic<StepT>,
}

impl<StepT: Clone + Eq + Debug + Deserialize + Serialize> HostStream<StepT> {
    pub fn new(
        required_deterministic_simulation_version: app_version::Version,
        tick_id: TickId,
    ) -> Self {
        Self {
            host_logic: HostLogic::<StepT>::new(tick_id, required_deterministic_simulation_version),
        }
    }

    fn handle_one_command(
        host_logic: &mut HostLogic<StepT>,
        connection_id: HostConnectionId,
        in_stream: &mut impl ReadOctetStream,
        state_provider: &impl GameStateProvider,
        now: Millis,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostStreamError> {
        let request = ClientToHostCommands::<StepT>::deserialize(in_stream)?;
        let commands = host_logic.update(connection_id, now, &request, state_provider)?;
        Ok(commands)
    }

    pub fn logic(&self) -> &HostLogic<StepT> {
        &self.host_logic
    }

    pub fn update(
        &mut self,
        connection_id: HostConnectionId,
        now: Millis,
        datagram: &[u8],
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<Vec<u8>>, HostStreamError> {
        trace!(
            "host-stream received from connection {}. payload:\n{}",
            connection_id.0,
            format_hex(datagram)
        );

        let mut in_stream = InOctetStream::new(datagram);
        let mut all_commands = Vec::new();
        while !in_stream.has_reached_end() {
            let commands = Self::handle_one_command(
                &mut self.host_logic,
                connection_id,
                &mut in_stream,
                state_provider,
                now,
            )?;
            all_commands.extend(commands);
        }
        Ok(serialize_to_chunker(all_commands, DATAGRAM_MAX_SIZE)?)
    }

    pub fn create_connection(&mut self) -> Option<HostConnectionId> {
        self.host_logic.create_connection()
    }

    pub fn destroy_connection(&mut self, p0: HostConnectionId) -> Result<(), HostStreamError> {
        self.host_logic.destroy_connection(p0)?;
        Ok(())
    }

    pub fn session(&self) -> &GameSession<StepT> {
        self.host_logic.session()
    }
}
