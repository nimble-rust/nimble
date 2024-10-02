/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use datagram_chunker::{serialize_to_chunker, DatagramChunkerError};
use flood_rs::prelude::InOctetStream;
use flood_rs::{Deserialize, Serialize};
use monotonic_time_rs::Millis;
use nimble_host_logic::logic::{ConnectionId, GameStateProvider, HostLogic, HostLogicError};
use nimble_protocol::prelude::ClientToHostCommands;
use std::fmt::Debug;
use std::io;
use tick_id::TickId;

pub struct HostStream<StepT: Clone> {
    host_logic: HostLogic<StepT>,
}

#[derive(Debug)]
pub enum HostStreamError {
    DatagramChunkerError(DatagramChunkerError),
    HostLogicError(HostLogicError),
    IoError(io::Error),
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

impl<StepT: Clone + Eq + Debug + Deserialize + Serialize> HostStream<StepT> {
    pub fn new(tick_id: TickId) -> Self {
        Self {
            host_logic: HostLogic::<StepT>::new(tick_id),
        }
    }

    const DATAGRAM_MAX_SIZE: usize = 1024;

    pub fn update(
        &mut self,
        connection_id: ConnectionId,
        now: Millis,
        datagram: &[u8],
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<Vec<u8>>, HostStreamError> {
        let mut in_stream = InOctetStream::new(datagram);
        let request = ClientToHostCommands::<StepT>::deserialize(&mut in_stream)?;
        let commands = self
            .host_logic
            .update(connection_id, now, &request, state_provider)?;
        Ok(serialize_to_chunker(commands, Self::DATAGRAM_MAX_SIZE)?)
    }

    pub fn create_connection(&mut self) -> Option<ConnectionId> {
        self.host_logic.create_connection()
    }
}
