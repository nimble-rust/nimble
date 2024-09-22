/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{ReadOctetStream, WriteOctetStream};
use std::fmt::Display;

#[derive(PartialEq, Eq, Copy, Hash, Clone, Debug)]
pub struct ParticipantId(pub u8);

impl ParticipantId {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> std::io::Result<()> {
        stream.write_u8(self.0)
    }
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> std::io::Result<Self> {
        Ok(Self(stream.read_u8()?))
    }
}

impl Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "participant:{}", self.0)
    }
}
