/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::ClientRequestId;
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use std::io;
use std::io::ErrorKind;

#[repr(u8)]
pub enum HostToClientOobCommand {
    Connect = 0x0D,
}
impl TryFrom<u8> for HostToClientOobCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            0x0D => Ok(HostToClientOobCommand::Connect),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown host to client oob command {}", value),
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ConnectionAccepted {
    pub flags: u8,
    pub response_to_request: ClientRequestId,
}

#[derive(Debug)]
pub enum HostToClientOobCommands {
    ConnectType(ConnectionAccepted),
}

impl ConnectionAccepted {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.flags)?;
        self.response_to_request.serialize(stream)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            flags: stream.read_u8()?,
            response_to_request: ClientRequestId::deserialize(stream)?,
        })
    }
}

impl HostToClientOobCommands {
    pub fn to_octet(&self) -> u8 {
        match self {
            HostToClientOobCommands::ConnectType(_) => HostToClientOobCommand::Connect as u8,
        }
    }

    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            HostToClientOobCommands::ConnectType(connect_command) => {
                connect_command.to_stream(stream)
            }
        }
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let command_value = stream.read_u8()?;
        let command = HostToClientOobCommand::try_from(command_value)?;
        let x = match command {
            HostToClientOobCommand::Connect => {
                HostToClientOobCommands::ConnectType(ConnectionAccepted::from_stream(stream)?)
            }
        };
        Ok(x)
    }
}
