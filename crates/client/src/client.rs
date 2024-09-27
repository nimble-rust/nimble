/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::datagram_build::NimbleDatagramBuilder;
use crate::datagram_parse::NimbleDatagramParser;
use datagram::{DatagramBuilder, DatagramError};
use datagram_builder::serialize::serialize_datagrams;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::prelude::{InOctetStream, OutOctetStream};
use flood_rs::{BufferDeserializer, ReadOctetStream};
use log::{debug, trace};
use nimble_client_connecting::{ConnectedInfo, ConnectingClient};
use nimble_client_logic::err::ClientError;
use nimble_client_logic::logic::ClientLogic;
use nimble_ordered_datagram::DatagramOrderInError;
use nimble_protocol::prelude::{HostToClientCommands, HostToClientOobCommands};
use nimble_protocol::{ClientRequestId, Version};

#[derive(Debug)]
pub enum ClientStreamError {
    Unexpected(String),
    IoErr(std::io::Error),
    ClientErr(ClientError),
    ClientConnectingErr(nimble_client_connecting::ClientError),
    DatagramError(DatagramError),
    DatagramOrderError(DatagramOrderInError),
    WrongPhase,
}

impl ErrorLevelProvider for ClientStreamError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            ClientStreamError::Unexpected(_) => ErrorLevel::Info,
            ClientStreamError::IoErr(_) => ErrorLevel::Info,
            ClientStreamError::ClientErr(_) => ErrorLevel::Info,
            ClientStreamError::ClientConnectingErr(_) => ErrorLevel::Info,
            ClientStreamError::DatagramError(_) => ErrorLevel::Info,
            ClientStreamError::DatagramOrderError(_) => ErrorLevel::Info,
            ClientStreamError::WrongPhase => ErrorLevel::Info,
        }
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
    datagram_parser: NimbleDatagramParser,
    datagram_builder: NimbleDatagramBuilder,
    phase: ClientPhase<StateT, StepT>,
    connected_info: Option<ConnectedInfo>,
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
        const DATAGRAM_MAX_SIZE: usize = 1024;
        Self {
            datagram_parser: NimbleDatagramParser::new(),
            datagram_builder: NimbleDatagramBuilder::new(DATAGRAM_MAX_SIZE),
            connected_info: None,
            phase: ClientPhase::Connecting(ConnectingClient::new(
                client_request_id,
                *application_version,
                nimble_protocol_version,
            )),
        }
    }

    fn connecting_receive(
        &mut self,
        mut in_octet_stream: InOctetStream,
    ) -> Result<(), ClientStreamError> {
        let connecting_client = match self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(ClientStreamError::WrongPhase)?,
        };

        let command = HostToClientOobCommands::from_stream(&mut in_octet_stream)
            .map_err(ClientStreamError::IoErr)?;
        connecting_client
            .receive(&command)
            .map_err(ClientStreamError::ClientConnectingErr)?;
        if let Some(connected_info) = connecting_client.connected_info() {
            debug!("connected! {connected_info:?}");
            self.connected_info = Some(*connected_info);

            self.phase = ClientPhase::Connected(ClientLogic::new());
        }
        Ok(())
    }

    fn connecting_receive_front(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        let (_, in_stream) = self
            .datagram_parser
            .parse(payload)
            .map_err(ClientStreamError::DatagramOrderError)?;
        self.connecting_receive(in_stream)
    }

    fn connected_receive(
        &mut self,
        in_stream: &mut InOctetStream,
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
        let (datagram_header, mut in_stream) = self
            .datagram_parser
            .parse(payload)
            .map_err(ClientStreamError::DatagramOrderError)?;

        // TODO: use connection_id from DatagramType::connection_id
        trace!("connection: client time {:?}", datagram_header.client_time);
        self.connected_receive(&mut in_stream)
    }

    pub fn receive(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        match &mut self.phase {
            ClientPhase::Connecting(_) => self.connecting_receive_front(payload),
            ClientPhase::Connected(_) => self.connected_receive_front(payload),
        }
    }

    fn connecting_send_front(&mut self) -> Result<Vec<u8>, ClientStreamError> {
        let connecting_client = match &mut self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(ClientStreamError::WrongPhase)?,
        };
        let request = connecting_client.send();
        let mut out_stream = OutOctetStream::new();
        request
            .to_stream(&mut out_stream)
            .map_err(ClientStreamError::IoErr)?;

        self.datagram_builder
            .clear()
            .map_err(ClientStreamError::IoErr)?;
        self.datagram_builder
            .push(out_stream.octets().as_slice())
            .map_err(ClientStreamError::DatagramError)?;
        Ok(self
            .datagram_builder
            .finalize()
            .map_err(ClientStreamError::IoErr)?
            .to_vec())
    }

    fn connected_send_front(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        let client_logic = match &mut self.phase {
            ClientPhase::Connected(ref mut client_logic) => client_logic,
            _ => Err(ClientStreamError::WrongPhase)?,
        };
        let commands = client_logic.send();
        serialize_datagrams(commands, &mut self.datagram_builder).map_err(ClientStreamError::IoErr)
    }

    pub fn send(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        match &mut self.phase {
            ClientPhase::Connecting(_) => Ok(vec![self.connecting_send_front()?]),
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

    pub fn debug_connect_info(&self) -> Option<ConnectedInfo> {
        self.connected_info
    }
}
