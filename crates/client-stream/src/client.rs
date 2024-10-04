/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::client::ClientPhase::Connected;
use datagram_chunker::{serialize_to_chunker, DatagramChunkerError};
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::prelude::OctetRefReader;
use flood_rs::{BufferDeserializer, Deserialize, ReadOctetStream};
use hexify::format_hex;
use log::{debug, trace};
use nimble_client_connecting::ConnectingClient;
use nimble_client_logic::err::ClientError;
use nimble_client_logic::logic::ClientLogic;
use nimble_protocol::prelude::{HostToClientCommands, HostToClientOobCommands};
use nimble_protocol::{ClientRequestId, Version};
use nimble_step_types::{AuthoritativeStep, PredictedStep};
use nimble_steps::StepsError;
use std::io;
use tick_id::TickId;

#[derive(Debug)]
pub enum ClientStreamError {
    Unexpected(String),
    IoErr(std::io::Error),
    ClientErr(ClientError),
    ClientConnectingErr(nimble_client_connecting::ClientError),
    PredictedStepsError(StepsError),
    DatagramChunkError(DatagramChunkerError),
    CommandNeedsConnectedPhase,
    CommandNeedsConnectingPhase,
}

impl ErrorLevelProvider for ClientStreamError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::Unexpected(_) => ErrorLevel::Info,
            Self::IoErr(_) => ErrorLevel::Info,
            Self::ClientErr(_) => ErrorLevel::Info,
            Self::ClientConnectingErr(_) => ErrorLevel::Info,
            Self::CommandNeedsConnectingPhase => ErrorLevel::Info,
            Self::CommandNeedsConnectedPhase => ErrorLevel::Info,
            Self::PredictedStepsError(_) => ErrorLevel::Warning,
            Self::DatagramChunkError(_) => ErrorLevel::Warning,
        }
    }
}

impl From<io::Error> for ClientStreamError {
    fn from(err: io::Error) -> Self {
        ClientStreamError::IoErr(err)
    }
}

impl From<DatagramChunkerError> for ClientStreamError {
    fn from(value: DatagramChunkerError) -> Self {
        Self::DatagramChunkError(value)
    }
}

impl From<StepsError> for ClientStreamError {
    fn from(value: StepsError) -> Self {
        Self::PredictedStepsError(value)
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ClientPhase<
    StateT: BufferDeserializer,
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
> {
    Connecting(ConnectingClient),
    Connected(ClientLogic<StateT, StepT>),
}

#[derive(Debug)]
pub struct ClientStream<
    StateT: BufferDeserializer,
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
> {
    phase: ClientPhase<StateT, StepT>,
}

impl<
        StateT: BufferDeserializer + std::fmt::Debug,
        StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
    > ClientStream<StateT, StepT>
{
    pub fn new(application_version: &app_version::Version) -> Self {
        let nimble_protocol_version = Version {
            major: 0,
            minor: 0,
            patch: 5,
        };
        let client_request_id = ClientRequestId(0);
        Self {
            phase: ClientPhase::Connecting(ConnectingClient::new(
                client_request_id,
                application_version,
                nimble_protocol_version,
            )),
        }
    }

    fn connecting_receive(
        &mut self,
        in_octet_stream: &mut impl ReadOctetStream,
    ) -> Result<(), ClientStreamError> {
        let connecting_client = match self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(ClientStreamError::CommandNeedsConnectingPhase)?,
        };

        // TODO: Do not allow empty datagrams in the future
        if in_octet_stream.has_reached_end() {
            return Ok(());
        }

        let command = HostToClientOobCommands::deserialize(in_octet_stream)?;
        connecting_client
            .receive(&command)
            .map_err(ClientStreamError::ClientConnectingErr)?;
        if connecting_client.is_connected() {
            debug!("connected!");
            self.phase = ClientPhase::Connected(ClientLogic::new());
        }
        Ok(())
    }

    fn connecting_receive_front(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        let mut in_stream = OctetRefReader::new(payload);
        self.connecting_receive(&mut in_stream)
    }

    fn connected_receive(
        &mut self,
        in_stream: &mut impl ReadOctetStream,
    ) -> Result<(), ClientStreamError> {
        let logic = match self.phase {
            ClientPhase::Connected(ref mut logic) => logic,
            _ => Err(ClientStreamError::CommandNeedsConnectedPhase)?,
        };
        while !in_stream.has_reached_end() {
            let cmd = HostToClientCommands::deserialize(in_stream)?;
            trace!("client-stream: connected_receive {cmd:?}");
            logic
                .receive_cmd(&cmd)
                .map_err(|err| ClientStreamError::ClientErr(ClientError::Single(err)))?;
        }
        Ok(())
    }

    fn connected_receive_front(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        let mut in_stream = OctetRefReader::new(payload);
        self.connected_receive(&mut in_stream)
    }

    pub fn pop_all_authoritative_steps(
        &mut self,
    ) -> Result<Vec<AuthoritativeStep<StepT>>, ClientStreamError> {
        match self.phase {
            ClientPhase::Connected(ref mut logic) => Ok(logic.pop_all_authoritative_steps()),
            _ => Err(ClientStreamError::CommandNeedsConnectedPhase)?,
        }
    }

    pub fn receive(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        trace!(
            "client-stream receive payload phase: {:?}\n{}",
            self.phase,
            format_hex(payload)
        );
        match &mut self.phase {
            ClientPhase::Connecting(_) => self.connecting_receive_front(payload),
            ClientPhase::Connected(_) => self.connected_receive_front(payload),
        }
    }

    fn connecting_send_front(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        let connecting_client = match &mut self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(ClientStreamError::CommandNeedsConnectingPhase)?,
        };
        let request = connecting_client.send();
        Ok(serialize_to_chunker([request], Self::DATAGRAM_MAX_SIZE)?)
    }
    const DATAGRAM_MAX_SIZE: usize = 1024;

    fn connected_send_front(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        let client_logic = match &mut self.phase {
            ClientPhase::Connected(ref mut client_logic) => client_logic,
            _ => Err(ClientStreamError::CommandNeedsConnectedPhase)?,
        };
        let commands = client_logic.send();
        Ok(serialize_to_chunker(commands, Self::DATAGRAM_MAX_SIZE)?)
    }

    pub fn send(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        match &mut self.phase {
            ClientPhase::Connecting(_) => Ok(self.connecting_send_front()?),
            ClientPhase::Connected(_) => self.connected_send_front(),
        }
    }

    pub fn game_state(&self) -> Option<&StateT> {
        match &self.phase {
            ClientPhase::Connected(ref client) => client.game_state(),
            _ => None,
        }
    }

    pub fn game_state_mut(&mut self) -> Option<&mut StateT> {
        match &mut self.phase {
            ClientPhase::Connected(ref mut client) => client.game_state_mut(),
            _ => None,
        }
    }

    pub fn debug_phase(&self) -> &ClientPhase<StateT, StepT> {
        &self.phase
    }

    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: PredictedStep<StepT>,
    ) -> Result<(), ClientStreamError> {
        match &mut self.phase {
            Connected(ref mut client_logic) => Ok(client_logic.push_predicted_step(tick_id, step)?),
            _ => Err(ClientStreamError::CommandNeedsConnectedPhase)?,
        }
    }

    /// Returns the average server buffer delta tick, if available.
    ///
    /// # Returns
    /// An optional average server buffer delta tick.
    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        match &self.phase {
            Connected(ref client_logic) => client_logic.server_buffer_delta_ticks(),
            _ => None,
        }
    }
}