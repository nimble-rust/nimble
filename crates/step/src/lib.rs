/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use flood_rs::prelude::*;
use std::fmt::{Display, Formatter};
use std::io;
use tick_id::TickId;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JoinedData {
    pub tick_id: TickId,
}

impl Serialize for JoinedData {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u32(self.tick_id.0)
    }
}

impl Deserialize for JoinedData {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            tick_id: TickId(stream.read_u32()?),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)] // Clone is needed since it can be in collections (like pending steps queue), Eq and PartialEq is to be able to use in tests, Debug for debug output.
pub enum Step<T> {
    Forced,
    WaitingForReconnect,
    Joined(JoinedData),
    Left,
    Custom(T),
}

impl<T> Step<T> {
    #[must_use]
    pub const fn to_octet(&self) -> u8 {
        match self {
            Self::Forced => 0x01,
            Self::WaitingForReconnect => 0x02,
            Self::Joined(_) => 0x03,
            Self::Left => 0x04,
            Self::Custom(_) => 0x05,
        }
    }
}

impl<T: Display> Display for Step<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forced => write!(f, "Forced"),
            Self::WaitingForReconnect => write!(f, "Forced"),
            Self::Joined(join_data) => write!(f, "joined {join_data:?}"),
            Self::Left => write!(f, "Left"),
            Self::Custom(custom) => write!(f, "Custom({})", custom),
        }
    }
}

impl<T: Serialize> Serialize for Step<T> {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            Self::Joined(join) => join.serialize(stream),
            Self::Custom(custom) => custom.serialize(stream),
            _ => Ok(()),
        }
    }
}

impl<T: Deserialize> Deserialize for Step<T> {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let step_type = stream.read_u8()?;
        let t = match step_type {
            0x01 => Self::Forced,
            0x02 => Self::WaitingForReconnect,
            0x03 => Self::Joined(JoinedData::deserialize(stream)?),
            0x04 => Self::Left,
            0x05 => Self::Custom(T::deserialize(stream)?),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid input, unknown step type",
            ))?,
        };
        Ok(t)
    }
}
