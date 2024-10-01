/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{ReadOctetStream, WriteOctetStream};
use std::{fmt, io};

pub struct DatagramIdDiff(i32);

impl DatagramIdDiff {
    const EXPECTED_MAX_DATAGRAMS_PER_SECOND: i32 = 1000;
    const EXPECTED_MAX_LATENCY_MS: i32 = 1000;
    const ORDERED_DATAGRAM_ID_ACCEPTABLE_DIFF: i32 =
        Self::EXPECTED_MAX_DATAGRAMS_PER_SECOND * Self::EXPECTED_MAX_LATENCY_MS / 1000;
    pub fn is_successor(&self) -> bool {
        self.0 > 0 && self.0 <= Self::ORDERED_DATAGRAM_ID_ACCEPTABLE_DIFF
    }

    pub fn is_equal_or_successor(&self) -> bool {
        self.0 >= 0 && self.0 <= Self::ORDERED_DATAGRAM_ID_ACCEPTABLE_DIFF
    }

    pub fn inner(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct DatagramId(u16);

impl DatagramId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn inner(self) -> u16 {
        self.0
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    fn to_stream(self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u16(self.0)
    }

    #[allow(unused)]
    fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<DatagramId> {
        Ok(Self(stream.read_u16()?))
    }

    pub fn sub(self, after: DatagramId) -> DatagramIdDiff {
        DatagramIdDiff(after.0.wrapping_sub(self.0) as i32)
    }

    #[allow(unused)]
    pub fn is_valid_successor(self, after: DatagramId) -> bool {
        self.sub(after).is_successor()
    }

    pub fn is_equal_or_successor(self, after: DatagramId) -> bool {
        self.sub(after).is_equal_or_successor()
    }
}

impl fmt::Display for DatagramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DatagramId({:X})", self.0)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OrderedOut {
    pub sequence_to_send: DatagramId,
}

impl OrderedOut {
    pub fn new() -> Self {
        Self {
            sequence_to_send: DatagramId(0),
        }
    }
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.sequence_to_send.to_stream(stream)
    }

    pub fn commit(&mut self) {
        self.sequence_to_send = DatagramId(self.sequence_to_send.0 + 1);
    }
}

#[derive(Debug)]
pub enum DatagramOrderInError {
    IoError(io::Error),
    WrongOrder {
        expected: DatagramId,
        received: DatagramId,
    },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OrderedIn {
    expected_sequence: DatagramId,
}

impl OrderedIn {
    pub fn read_and_verify(
        &mut self,
        stream: &mut impl ReadOctetStream,
    ) -> Result<DatagramIdDiff, DatagramOrderInError> {
        let potential_expected_or_successor =
            DatagramId::from_stream(stream).map_err(DatagramOrderInError::IoError)?;

        let diff = self.expected_sequence.sub(potential_expected_or_successor);
        if diff.is_equal_or_successor() {
            self.expected_sequence = potential_expected_or_successor.next();
            Ok(diff)
        } else {
            Err(DatagramOrderInError::WrongOrder {
                received: potential_expected_or_successor,
                expected: self.expected_sequence,
            })
        }
    }
}
