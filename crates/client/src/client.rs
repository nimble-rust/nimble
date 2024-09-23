/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::datagram_build::NimbleDatagramBuilder;
use crate::datagram_parse::NimbleDatagramParser;
use datagram::DatagramBuilder;
use datagram_builder::serialize::serialize_datagrams;
use flood_rs::prelude::{InOctetStream, OutOctetStream};
use flood_rs::ReadOctetStream;
use log::{debug, trace};
use nimble_client_connecting::{ConnectedInfo, ConnectingClient};
use nimble_client_logic::logic::ClientLogic;
use nimble_protocol::prelude::{HostToClientCommands, HostToClientOobCommands};
use nimble_protocol::{ClientRequestId, Version};
use std::io;
use std::io::{Error, ErrorKind};

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ClientPhase<StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug> {
    Connecting(ConnectingClient),
    Connected(ClientLogic<StepT>),
}
pub struct ClientStream<
    StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug,
> {
    datagram_parser: NimbleDatagramParser,
    datagram_builder: NimbleDatagramBuilder,
    phase: ClientPhase<StepT>,
    connected_info: Option<ConnectedInfo>,
}

impl<StepT: Clone + flood_rs::Deserialize + flood_rs::Serialize + std::fmt::Debug>
    ClientStream<StepT>
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

    fn connecting_receive(&mut self, mut in_octet_stream: InOctetStream) -> io::Result<()> {
        let connecting_client = match self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(Error::new(ErrorKind::InvalidData, "bad phase"))?,
        };

        let command = HostToClientOobCommands::from_stream(&mut in_octet_stream)?;
        connecting_client
            .receive(&command)
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;
        if let Some(connected_info) = connecting_client.connected_info() {
            debug!("connected! {connected_info:?}");
            self.connected_info = Some(*connected_info);

            self.phase = ClientPhase::Connected(ClientLogic::new());
        }
        Ok(())
    }

    fn connecting_receive_front(&mut self, payload: &[u8]) -> io::Result<()> {
        let (_, in_stream) = self.datagram_parser.parse(payload)?;
        self.connecting_receive(in_stream)
    }

    fn connected_receive(&mut self, in_stream: &mut InOctetStream) -> io::Result<()> {
        let logic = match self.phase {
            ClientPhase::Connected(ref mut logic) => logic,
            _ => Err(Error::new(ErrorKind::InvalidData, "bad phase"))?,
        };
        while !in_stream.has_reached_end() {
            let cmd = HostToClientCommands::from_stream(in_stream)?;
            trace!("connected_receive {cmd:?}");
            logic
                .receive_cmd(&cmd)
                .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;
        }
        Ok(())
    }

    fn connected_receive_front(&mut self, payload: &[u8]) -> io::Result<()> {
        let (datagram_header, mut in_stream) = self.datagram_parser.parse(payload)?;

        // TODO: use connection_id from DatagramType::connection_id
        trace!("connection: client time {:?}", datagram_header.client_time);
        self.connected_receive(&mut in_stream)
    }

    pub fn receive(&mut self, payload: &[u8]) -> io::Result<()> {
        match &mut self.phase {
            ClientPhase::Connecting(_) => self.connecting_receive_front(payload),
            ClientPhase::Connected(_) => self.connected_receive_front(payload),
        }
    }

    fn connecting_send_front(&mut self) -> io::Result<Vec<u8>> {
        let connecting_client = match &mut self.phase {
            ClientPhase::Connecting(ref mut connecting_client) => connecting_client,
            _ => Err(io::Error::new(ErrorKind::InvalidData, "illegal state"))?,
        };
        let request = connecting_client.send();
        let mut out_stream = OutOctetStream::new();
        request.to_stream(&mut out_stream)?;

        self.datagram_builder.clear()?;
        self.datagram_builder
            .push(out_stream.octets().as_slice())
            .map_err(|err| Error::new(ErrorKind::InvalidData, err.to_string()))?;
        Ok(self.datagram_builder.finalize()?.to_vec())
    }

    fn connected_send_front(&mut self) -> io::Result<Vec<Vec<u8>>> {
        let client_logic = match &mut self.phase {
            ClientPhase::Connected(ref mut client_logic) => client_logic,
            _ => Err(io::Error::new(ErrorKind::InvalidData, "illegal state"))?,
        };
        let commands = client_logic.send();
        serialize_datagrams(commands, &mut self.datagram_builder)
    }

    pub fn send(&mut self) -> io::Result<Vec<Vec<u8>>> {
        match &mut self.phase {
            ClientPhase::Connecting(_) => Ok(vec![self.connecting_send_front()?]),
            ClientPhase::Connected(_) => self.connected_send_front(),
        }
    }

    pub fn debug_phase(&self) -> &ClientPhase<StepT> {
        &self.phase
    }

    pub fn debug_connect_info(&self) -> Option<ConnectedInfo> {
        self.connected_info
    }
}
