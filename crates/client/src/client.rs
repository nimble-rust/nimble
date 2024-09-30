/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::client::ClientPhase::Connected;
use crate::datagram_build::{serialize_to_chunker, DatagramChunkerError};
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::prelude::OctetRefReader;
use flood_rs::{BufferDeserializer, ReadOctetStream};
use log::{debug, trace};
use nimble_client_connecting::ConnectingClient;
use nimble_client_logic::err::ClientError;
use nimble_client_logic::logic::ClientLogic;
use nimble_protocol::prelude::{HostToClientCommands, HostToClientOobCommands};
use nimble_protocol::{ClientRequestId, Version};
use nimble_step_types::PredictedStep;
use nimble_steps::StepsError;
use tick_id::TickId;

#[derive(Debug)]
pub enum ClientStreamError {
    Unexpected(String),
    IoErr(std::io::Error),
    ClientErr(ClientError),
    ClientConnectingErr(nimble_client_connecting::ClientError),
    PredictedStepsError(StepsError),
    DatagramChunkError(DatagramChunkerError),
    WrongPhase,
}

impl ErrorLevelProvider for ClientStreamError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::Unexpected(_) => ErrorLevel::Info,
            Self::IoErr(_) => ErrorLevel::Info,
            Self::ClientErr(_) => ErrorLevel::Info,
            Self::ClientConnectingErr(_) => ErrorLevel::Info,
            Self::WrongPhase => ErrorLevel::Info,
            Self::PredictedStepsError(_) => ErrorLevel::Warning,
            Self::DatagramChunkError(_) => ErrorLevel::Warning,
        }
    }
}

impl From<DatagramChunkerError> for ClientStreamError {
    fn from(value: DatagramChunkerError) -> Self {
        Self::DatagramChunkError(value)
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
pub struct ClientStream<
    StateT: BufferDeserializer,
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
> {
    phase: ClientPhase<StateT, StepT>,
}

impl<
    StateT: BufferDeserializer,
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
> ClientStream<StateT, StepT>
{
    pub fn new(application_version: &Version) -> Self {
        let nimble_protocol_version = Version {
            major: 0,
            minor: 0,
            patch: 5,
        };
        let client_request_id = ClientRequestId(0);
        Self {
            phase: ClientPhase::Connecting(ConnectingClient::new(
                client_request_id,
                *application_version,
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
            _ => Err(ClientStreamError::WrongPhase)?,
        };

        let command = HostToClientOobCommands::from_stream(in_octet_stream)
            .map_err(ClientStreamError::IoErr)?;
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
            _ => Err(ClientStreamError::WrongPhase)?,
        };
        while !in_stream.has_reached_end() {
            let cmd =
                HostToClientCommands::from_stream(in_stream).map_err(ClientStreamError::IoErr)?;
            trace!("connected_receive {cmd:?}");
            logic
                .receive_cmd(&cmd)
                .map_err(|err| ClientStreamError::ClientErr(ClientError::Single(err)))?;
        }
        Ok(())
    }

    fn connected_receive_front(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        trace!("connected receive payload length: {}", payload.len());

        let mut in_stream = OctetRefReader::new(payload);
        self.connected_receive(&mut in_stream)
    }

    pub fn receive(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        match &mut self.phase {
            ClientPhase::Connecting(_) => self.connecting_receive_front(payload),
            ClientPhase::Connected(_) => self.connected_receive_front(payload),
        }
    }

    fn connecting_send_front(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        let connecting_client = match &mut self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(ClientStreamError::WrongPhase)?,
        };
        let request = connecting_client.send();
        Ok(serialize_to_chunker([request], Self::DATAGRAM_MAX_SIZE)?)
    }
    const DATAGRAM_MAX_SIZE: usize = 1024;

    fn connected_send_front(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        let client_logic = match &mut self.phase {
            ClientPhase::Connected(ref mut client_logic) => client_logic,
            _ => Err(ClientStreamError::WrongPhase)?,
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

    pub fn state(&self) -> Option<&StateT> {
        match &self.phase {
            ClientPhase::Connected(ref client) => client.state(),
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
            Connected(ref mut client_logic) => client_logic
                .push_predicted_step(tick_id, step)
                .map_err(ClientStreamError::PredictedStepsError),
            _ => Err(ClientStreamError::WrongPhase)?,
        }
    }
}
