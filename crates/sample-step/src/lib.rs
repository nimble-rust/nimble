/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::*;
use std::io;

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
            SampleStep::Nothing => stream.write_u8(0x00),
            SampleStep::MoveLeft(amount) => {
                stream.write_u8(0x01)?;
                stream.write_i16(*amount)
            }
            SampleStep::MoveRight(amount) => {
                stream.write_u8(0x02)?;
                stream.write_i16(*amount)
            }
            SampleStep::Jump => stream.write_u8(0x03),
        }
    }
}

impl Deserialize for SampleStep {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let octet = stream.read_u8()?;
        let value = match octet {
            0x00 => SampleStep::Nothing,
            0x01 => SampleStep::MoveLeft(stream.read_i16()?),
            0x02 => SampleStep::MoveRight(stream.read_i16()?),
            0x03 => SampleStep::Jump,
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid input"))?,
        };
        Ok(value)
    }
}
