/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use std::fmt;
use std::io::Result;

use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};

pub mod client_to_host;
pub mod host_to_client;
pub mod prelude;
pub mod serialize;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ClientRequestId(pub u8);

impl fmt::Display for ClientRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RequestId({:X})", self.0)
    }
}

impl ClientRequestId {
    #[must_use]

    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}

impl Serialize for ClientRequestId {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> Result<()>
    where
        Self: Sized,
    {
        stream.write_u8(self.0)
    }
}

impl Deserialize for ClientRequestId {
    fn deserialize(stream: &mut impl ReadOctetStream) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(stream.read_u8()?))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    #[must_use]

    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> Result<()> {
        stream.write_u16(self.major)?;
        stream.write_u16(self.minor)?;
        stream.write_u16(self.patch)?;

        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> Result<Self> {
        Ok(Self {
            major: stream.read_u16()?,
            minor: stream.read_u16()?,
            patch: stream.read_u16()?,
        })
    }
}

pub const NIMBLE_PROTOCOL_VERSION: Version = Version::new(0, 0, 5);

#[derive(PartialEq, Copy, Clone, Eq)]
pub struct SessionConnectionSecret {
    pub value: u64,
}

impl SessionConnectionSecret {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> Result<()> {
        stream.write_u64(self.value)
    }
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> Result<Self> {
        Ok(Self {
            value: stream.read_u64()?,
        })
    }
}

impl fmt::Display for SessionConnectionSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session_secret: {:X}", self.value)
    }
}

impl fmt::Debug for SessionConnectionSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session_secret: {:X}", self.value)
    }
}
