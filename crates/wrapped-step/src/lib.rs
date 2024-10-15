/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use flood_rs::prelude::*;
use std::fmt::{Debug, Display};
use std::io;

#[derive(Debug)]
pub struct GenericOctetStep {
    pub payload: Vec<u8>,
}

impl Serialize for GenericOctetStep {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        stream.write_u8(self.payload.len() as u8)?;
        stream.write(self.payload.as_slice())
    }
}

impl Deserialize for GenericOctetStep {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len = stream.read_u8()? as usize;
        let mut payload = vec![0u8; len];
        stream.read(&mut payload)?;
        Ok(Self { payload })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WrappedOctetStep<T: Serialize + Deserialize + Clone + Debug + Display + Eq> {
    pub step: T,
}

impl<T: Serialize + Deserialize + Clone + Debug + Display + Eq> Display for WrappedOctetStep<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wrapped {}", self.step)
    }
}

impl<T: Serialize + Deserialize + Clone + Debug + Display + Eq> Serialize for WrappedOctetStep<T> {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        let mut out_stream = OutOctetStream::new();
        self.step.serialize(&mut out_stream)?;
        stream.write_u8(out_stream.octets_ref().len() as u8)?;
        stream.write(out_stream.octets_ref())
    }
}

impl<T: Serialize + Deserialize + Clone + Debug + Display + Eq> Deserialize
    for WrappedOctetStep<T>
{
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len = stream.read_u8()? as usize;
        let mut payload = vec![0u8; len];
        stream.read(&mut payload)?;

        let mut in_stream = InOctetStream::new(&payload);
        Ok(Self {
            step: Deserialize::deserialize(&mut in_stream)?,
        })
    }
}
