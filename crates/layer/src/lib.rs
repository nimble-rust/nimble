/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::{InOctetStream, OutOctetStream};
use flood_rs::{Deserialize, Serialize};
use hexify::format_hex;
use log::{debug, trace};
use metricator::{AggregateMetric, MinMaxAvg};
use monotonic_time_rs::{Millis, MillisDuration, MillisLow16};

use nimble_ordered_datagram::{DatagramOrderInError, OrderedIn, OrderedOut};
use nimble_protocol_header::ClientTime;
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
            ordered_datagram_out: Default::default(),
            ordered_in: Default::default(),
            datagram_drops: AggregateMetric::new(16).expect("threshold should be ok"),
        }
    }
}

pub struct NimbleLayerClient {
    layer: NimbleLayer,
    latency: AggregateMetric<u16>,

    last_debug_metric_at: Millis,
    debug_metric_duration: MillisDuration,
}

#[derive(Debug)]
pub enum NimbleLayerError {
    IoError(io::Error),
    DatagramInOrderError(DatagramOrderInError),
    MillisFromLowerError,
    AbsoluteTimeError,
}

impl NimbleLayerClient {
    pub fn new(now: Millis) -> Self {
        Self {
            layer: NimbleLayer::new(),
            latency: AggregateMetric::<u16>::new(10).unwrap().with_unit("ms"),

            last_debug_metric_at: now,
            debug_metric_duration: MillisDuration::from_secs(1.0).unwrap(),
        }
    }

    pub fn receive<'a>(
        &mut self,
        now: Millis,
        datagram: &'a [u8],
    ) -> Result<&'a [u8], NimbleLayerError> {
        let (slice, client_time) = self.layer.receive(datagram)?;

        let low_16 = client_time.inner() as MillisLow16;

        let earlier = now
            .from_lower(low_16)
            .ok_or_else(|| NimbleLayerError::MillisFromLowerError)?;
        let duration_ms = now
            .checked_duration_since_ms(earlier)
            .ok_or_else(|| NimbleLayerError::AbsoluteTimeError)?;

        self.latency.add(duration_ms.as_millis() as u16);

        Ok(slice)
    }

    pub fn send(
        &mut self,
        now: Millis,
        datagrams: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, io::Error> {
        let client_time = ClientTime::new(now.to_lower());
        self.layer.send(client_time, datagrams)
    }

    pub fn update(&mut self, now: Millis) {
        if now - self.last_debug_metric_at > self.debug_metric_duration {
            self.last_debug_metric_at = now;
            debug!("metrics: {:?}", self.latency())
        }
    }

    pub fn latency(&self) -> Option<MinMaxAvg<u16>> {
        self.latency.values()
    }

    pub fn datagram_drops(&self) -> Option<MinMaxAvg<u16>> {
        self.layer.datagram_drops()
    }
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

impl NimbleLayer {
    pub fn new() -> Self {
        Self {
            ordered_datagram_out: Default::default(),
            ordered_in: Default::default(),
            datagram_drops: AggregateMetric::<u16>::new(10).unwrap(),
        }
    }

    pub fn send(
        &mut self,
        client_time: ClientTime,
        datagrams: Vec<Vec<u8>>,
    ) -> Result<Vec<Vec<u8>>, io::Error> {
        let mut packet = [0u8; 1200];
        let mut out_datagrams: Vec<Vec<u8>> = vec![];

        for datagram in &datagrams {
            let mut stream = OutOctetStream::new();

            // Serialize
            self.ordered_datagram_out.to_stream(&mut stream)?; // Ordered datagrams

            client_time.serialize(&mut stream)?;

            packet[0..4].copy_from_slice(stream.octets_ref());
            packet[4..4 + datagram.len()].copy_from_slice(datagram);

            let complete_datagram = packet[0..4 + datagram.len()].to_vec();
            out_datagrams.push(complete_datagram);
            self.ordered_datagram_out.commit();
        }

        Ok(out_datagrams)
    }

    pub fn receive<'a>(
        &mut self,
        datagram: &'a [u8],
    ) -> Result<(&'a [u8], ClientTime), NimbleLayerError> {
        trace!("client-front received\n{}", format_hex(datagram));
        let mut in_stream = InOctetStream::new(datagram);
        let dropped_packets = self.ordered_in.read_and_verify(&mut in_stream)?;
        self.datagram_drops.add(dropped_packets.inner());

        let client_time = ClientTime::deserialize(&mut in_stream)?;

        Ok((&datagram[4..], client_time))
    }

    pub fn datagram_drops(&self) -> Option<MinMaxAvg<u16>> {
        self.datagram_drops.values()
    }
}
