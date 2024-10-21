/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::{InOctetStream, OutOctetStream};
use hexify::format_hex;
use log::trace;
use metricator::{AggregateMetric, MinMaxAvg};

use nimble_ordered_datagram::{DatagramOrderInError, OrderedIn, OrderedOut};
use std::io;

#[derive(Debug)]
pub struct NimbleLayer {
    ordered_datagram_out: OrderedOut,
    ordered_in: OrderedIn,
    datagram_drops: AggregateMetric<u16>,
}

impl Default for NimbleLayer {
    fn default() -> Self {
        Self {
            ordered_datagram_out: OrderedOut::default(),
            ordered_in: OrderedIn::default(),
            datagram_drops: AggregateMetric::new(16).expect("threshold should be ok"),
        }
    }
}

#[derive(Debug)]
pub enum NimbleLayerError {
    IoError(io::Error),
    DatagramInOrderError(DatagramOrderInError),
    MillisFromLowerError,
    AbsoluteTimeError,
}

impl From<DatagramOrderInError> for NimbleLayerError {
    fn from(err: DatagramOrderInError) -> Self {
        Self::DatagramInOrderError(err)
    }
}

impl From<io::Error> for NimbleLayerError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

const ORDERED_DATAGRAM_OCTETS: usize = 2;

impl NimbleLayer {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new() -> Self {
        Self {
            ordered_datagram_out: OrderedOut::default(),
            ordered_in: OrderedIn::default(),
            datagram_drops: AggregateMetric::<u16>::new(10).unwrap(),
        }
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn send(&mut self, datagrams: &Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>, io::Error> {
        let mut packet = [0u8; 1200];
        let mut out_datagrams: Vec<Vec<u8>> = vec![];

        for datagram in datagrams {
            let mut stream = OutOctetStream::new();

            self.ordered_datagram_out.to_stream(&mut stream)?;

            packet[0..ORDERED_DATAGRAM_OCTETS].copy_from_slice(stream.octets_ref());
            packet[ORDERED_DATAGRAM_OCTETS..ORDERED_DATAGRAM_OCTETS + datagram.len()]
                .copy_from_slice(datagram);

            let complete_datagram = packet[0..ORDERED_DATAGRAM_OCTETS + datagram.len()].to_vec();
            out_datagrams.push(complete_datagram);
            self.ordered_datagram_out.commit();
        }

        Ok(out_datagrams)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn receive<'a>(&mut self, datagram: &'a [u8]) -> Result<&'a [u8], NimbleLayerError> {
        let mut in_stream = InOctetStream::new(datagram);
        let dropped_packets = self.ordered_in.read_and_verify(&mut in_stream)?;
        self.datagram_drops.add(dropped_packets.inner());

        let slice = &datagram[ORDERED_DATAGRAM_OCTETS..];
        trace!(
            "nimble-layer host received without header\n{}",
            format_hex(slice)
        );
        Ok(slice)
    }

    #[must_use]
    pub fn datagram_drops(&self) -> Option<MinMaxAvg<u16>> {
        self.datagram_drops.values()
    }
}
