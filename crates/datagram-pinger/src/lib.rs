/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::*;

#[derive(Debug)]
pub struct ClientTime(u16);

impl ClientTime {
    pub fn new(time: u16) -> Self {
        Self(time)
    }
}

impl Serialize for ClientTime {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> std::io::Result<()>
    where
        Self: Sized,
    {
        stream.write_u16(self.0)
    }
}

impl Deserialize for ClientTime {
    fn deserialize(stream: &mut impl ReadOctetStream) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(stream.read_u16()?))
    }
}