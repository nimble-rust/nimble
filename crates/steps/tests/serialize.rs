/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::*;
use nimble_steps::Step;
use std::io;

mod types;

#[derive(Debug, PartialEq, Eq)] // Debug is needed for asserts in tests
pub enum SerializableGameInput {
    Jumping(bool),
    MoveHorizontal(i32),
}

impl Serialize for SerializableGameInput {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        match self {
            SerializableGameInput::Jumping(jumping) => {
                stream.write_u8(0x01)?;
                stream.write_u8(if *jumping { 0x01 } else { 0x00 })?;
                Ok(())
            }
            SerializableGameInput::MoveHorizontal(horizontal) => {
                stream.write_u8(0x02)?;
                stream.write_i32(*horizontal)?;
                Ok(())
            }
        }
    }
}

impl Deserialize for SerializableGameInput {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let type_id = stream.read_u8()?;
        let game_input = match type_id {
            0x01 => SerializableGameInput::Jumping(stream.read_u8()? != 0),
            0x02 => SerializableGameInput::MoveHorizontal(stream.read_i32()?),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid type id",
            ))?,
        };

        Ok(game_input)
    }
}

#[test_log::test]
fn serialize() -> io::Result<()> {
    let mut out_stream = OutOctetStream::new();

    let step = Step::<SerializableGameInput>::WaitingForReconnect;

    step.serialize(&mut out_stream)?;

    let mut in_stream = InOctetStream::new(out_stream.octets_ref());

    let deserialized_step = Step::deserialize(&mut in_stream)?;

    assert_eq!(step, deserialized_step);

    Ok(())
}

#[test_log::test]
fn serialize_custom() -> io::Result<()> {
    let mut out_stream = OutOctetStream::new();

    let step = Step::<SerializableGameInput>::Custom(SerializableGameInput::MoveHorizontal(-922));

    step.serialize(&mut out_stream)?;

    let mut in_stream = InOctetStream::new(out_stream.octets_ref());

    let deserialized_step = Step::deserialize(&mut in_stream)?;

    assert_eq!(step, deserialized_step);

    Ok(())
}
