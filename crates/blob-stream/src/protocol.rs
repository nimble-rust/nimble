/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{ReadOctetStream, WriteOctetStream};
use std::io;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SetChunkData {
    pub chunk_index: u32,
    pub payload: Vec<u8>,
}

impl SetChunkData {
    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u32(self.chunk_index)?;
        stream.write_u16(self.payload.len() as u16)?;
        stream.write(&self.payload[..])?;
        Ok(())
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let chunk_index = stream.read_u32()?;
        let octet_length = stream.read_u16()?;
        let mut payload = vec![0u8; octet_length as usize];
        stream.read(&mut payload)?;

        Ok(Self {
            chunk_index,
            payload,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TransferId(pub u16);

impl TransferId {
    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.

    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u16(self.0)
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self(stream.read_u16()?))
    }
}

// ---------- Receiver

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AckChunkData {
    pub waiting_for_chunk_index: u32, // first chunk index that remote has not received fully in sequence. (first gap in chunks from the start).
    pub receive_mask_after_last: u64, // receive bit mask for chunks after the `waiting_for_chunk_index`
}

impl AckChunkData {
    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u32(self.waiting_for_chunk_index)?;
        stream.write_u64(self.receive_mask_after_last)?;
        Ok(())
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            waiting_for_chunk_index: stream.read_u32()?,
            receive_mask_after_last: stream.read_u64()?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StartTransferData {
    pub transfer_id: u16, // Unique transfer_id for this session
    pub total_octet_size: u32,
    pub chunk_size: u16,
}

impl StartTransferData {
    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u16(self.transfer_id)?;
        stream.write_u32(self.total_octet_size)?;
        stream.write_u16(self.chunk_size)
    }

    /// # Errors
    ///
    /// This function will return an `io::Error` if there is an issue with writing to the stream.
    /// This could happen if the stream is closed or if there are underlying I/O errors during the write operation.
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let transfer_id = stream.read_u16()?;
        let total_octet_size = stream.read_u32()?;
        let chunk_size = stream.read_u16()?;

        Ok(Self {
            transfer_id,
            total_octet_size,
            chunk_size,
        })
    }
}
