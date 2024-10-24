/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::protocol::{AckChunkData, SetChunkData, StartTransferData, TransferId};
use flood_rs::{ReadOctetStream, WriteOctetStream};
use std::fmt::Display;
use std::io;
use std::io::ErrorKind;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetChunkFrontData {
    pub transfer_id: TransferId,
    pub data: SetChunkData,
}

impl SetChunkFrontData {
    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.transfer_id.to_stream(stream)?;
        self.data.to_stream(stream)?;
        Ok(())
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            transfer_id: TransferId::from_stream(stream)?,
            data: SetChunkData::from_stream(stream)?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SenderToReceiverFrontCommands {
    SetChunk(SetChunkFrontData),
    StartTransfer(StartTransferData),
}

impl Display for SenderToReceiverFrontCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetChunk(set_chunk_data) => write!(
                f,
                "set_chunk transfer{} index:{} chunk_size:{}",
                set_chunk_data.transfer_id.0,
                set_chunk_data.data.chunk_index,
                set_chunk_data.data.payload.len()
            ),
            Self::StartTransfer(transfer_data) => {
                write!(f, "start transfer {transfer_data:?}")
            }
        }
    }
}

#[repr(u8)]
enum SenderToReceiverFrontCommand {
    SetChunk = 0x01,
    StartTransfer = 0x02,
}

impl TryFrom<u8> for SenderToReceiverFrontCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        Ok(match value {
            0x01 => Self::SetChunk,
            0x02 => Self::StartTransfer,
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown SenderToReceiverFrontCommand {value}"),
            ))?,
        })
    }
}

impl SenderToReceiverFrontCommands {
    #[must_use]
    pub const fn to_octet(&self) -> u8 {
        match self {
            Self::SetChunk(_) => SenderToReceiverFrontCommand::SetChunk as u8,
            Self::StartTransfer(_) => SenderToReceiverFrontCommand::StartTransfer as u8,
        }
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.

    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            Self::SetChunk(set_chunk_header) => set_chunk_header.to_stream(stream),
            Self::StartTransfer(transfer_data) => transfer_data.to_stream(stream),
        }
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let command_value = stream.read_u8()?;
        let command = SenderToReceiverFrontCommand::try_from(command_value)?;
        let x = match command {
            SenderToReceiverFrontCommand::SetChunk => {
                Self::SetChunk(SetChunkFrontData::from_stream(stream)?)
            }
            SenderToReceiverFrontCommand::StartTransfer => {
                Self::StartTransfer(StartTransferData::from_stream(stream)?)
            }
        };
        Ok(x)
    }
}

#[repr(u8)]
enum ReceiverToSenderFrontCommand {
    AckStart = 0x03,
    AckChunk = 0x04,
}

impl TryFrom<u8> for ReceiverToSenderFrontCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        Ok(match value {
            0x03 => Self::AckStart,
            0x04 => Self::AckChunk,
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown ReceiverToSenderFrontCommand {value}"),
            ))?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AckChunkFrontData {
    pub transfer_id: TransferId,
    pub data: AckChunkData,
}

impl AckChunkFrontData {
    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.transfer_id.to_stream(stream)?;
        self.data.to_stream(stream)
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            transfer_id: TransferId::from_stream(stream)?,
            data: AckChunkData::from_stream(stream)?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ReceiverToSenderFrontCommands {
    AckChunk(AckChunkFrontData),
    AckStart(u16),
}

impl ReceiverToSenderFrontCommands {
    #[must_use]
    pub const fn to_octet(&self) -> u8 {
        match self {
            Self::AckChunk(_) => ReceiverToSenderFrontCommand::AckChunk as u8,
            Self::AckStart(_) => ReceiverToSenderFrontCommand::AckStart as u8,
        }
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            Self::AckChunk(set_chunk_header) => set_chunk_header.to_stream(stream),
            Self::AckStart(transfer_id) => stream.write_u16(*transfer_id),
        }
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let command_value = stream.read_u8()?;
        let command = ReceiverToSenderFrontCommand::try_from(command_value)?;
        let x = match command {
            ReceiverToSenderFrontCommand::AckChunk => Self::AckChunk(AckChunkFrontData {
                transfer_id: TransferId::from_stream(stream)?,
                data: AckChunkData::from_stream(stream)?,
            }),
            ReceiverToSenderFrontCommand::AckStart => Self::AckStart(stream.read_u16()?),
        };
        Ok(x)
    }
}
