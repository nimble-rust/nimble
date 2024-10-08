/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::*;
use flood_rs::BufferDeserializer;
use std::io;

#[derive(Debug)]
pub struct SampleState {
    pub buf: Vec<u8>,
}

impl BufferDeserializer for SampleState {
    fn deserialize(buf: &[u8]) -> io::Result<(Self, usize)>
    where
        Self: Sized,
    {
        Ok((Self { buf: buf.into() }, buf.len()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampleStep {
    MoveLeft(i16),
    MoveRight(i16),
    Jump,
    Nothing,
}

impl Serialize for SampleStep {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        match self {
            Self::Nothing => stream.write_u8(0x00),
            Self::MoveLeft(amount) => {
                stream.write_u8(0x01)?;
                stream.write_i16(*amount)
            }
            Self::MoveRight(amount) => {
                stream.write_u8(0x02)?;
                stream.write_i16(*amount)
            }
            Self::Jump => stream.write_u8(0x03),
        }
    }
}

impl Deserialize for SampleStep {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let octet = stream.read_u8()?;
        Ok(match octet {
            0x00 => Self::Nothing,
            0x01 => Self::MoveLeft(stream.read_i16()?),
            0x02 => Self::MoveRight(stream.read_i16()?),
            0x03 => Self::Jump,
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid input"))?,
        })
    }
}
