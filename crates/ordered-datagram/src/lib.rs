/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::{ReadOctetStream, WriteOctetStream};
use std::ops::Sub;
use std::{fmt, io};

pub struct DatagramIdDiff(u16);

impl DatagramIdDiff {
    const EXPECTED_MAX_DATAGRAMS_PER_SECOND: u16 = 1000;
    const EXPECTED_MAX_LATENCY_MS: u16 = 1000;
    const ORDERED_DATAGRAM_ID_ACCEPTABLE_DIFF: u16 =
        Self::EXPECTED_MAX_DATAGRAMS_PER_SECOND * (Self::EXPECTED_MAX_LATENCY_MS / 1000);
    #[must_use]
    pub const fn is_successor(&self) -> bool {
        self.0 > 0 && self.0 <= Self::ORDERED_DATAGRAM_ID_ACCEPTABLE_DIFF
    }

    #[must_use]
    pub const fn is_equal_or_successor(&self) -> bool {
        self.0 <= Self::ORDERED_DATAGRAM_ID_ACCEPTABLE_DIFF
    }

    #[must_use]
    pub const fn inner(&self) -> u16 {
        self.0
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct DatagramId(u16);

impl DatagramId {
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn inner(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }

    pub fn next_mut(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    fn to_stream(self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u16(self.0)
    }

    #[allow(unused)]
    fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self(stream.read_u16()?))
    }

    #[allow(unused)]
    #[must_use]
    pub fn is_valid_successor(self, after: Self) -> bool {
        (self - after).is_successor()
    }

    #[must_use]
    pub fn is_equal_or_successor(self, after: Self) -> bool {
        (self - after).is_equal_or_successor()
    }
}

impl Sub for DatagramId {
    type Output = DatagramIdDiff;
    fn sub(self, rhs: Self) -> DatagramIdDiff {
        DatagramIdDiff(rhs.0.wrapping_sub(self.0))
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
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sequence_to_send: DatagramId(0),
        }
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.sequence_to_send.to_stream(stream)
    }

    pub fn commit(&mut self) {
        self.sequence_to_send.next_mut();
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

impl From<io::Error> for DatagramOrderInError {
    fn from(e: io::Error) -> Self {
        Self::IoError(e)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OrderedIn {
    expected_sequence: DatagramId,
}

impl OrderedIn {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn read_and_verify(
        &mut self,
        stream: &mut impl ReadOctetStream,
    ) -> Result<DatagramIdDiff, DatagramOrderInError> {
        let potential_expected_or_successor = DatagramId::from_stream(stream)?;

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
