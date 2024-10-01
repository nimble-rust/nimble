/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use flood_rs::prelude::{InOctetStream, OutOctetStream};
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use log::info;
use metricator::AggregateMetric;
use monotonic_time_rs::{MillisLow16, MonotonicClock};
use nimble_client_stream::client::{ClientStream, ClientStreamError};
use nimble_ordered_datagram::{DatagramOrderInError, OrderedIn, OrderedOut};
use nimble_protocol::Version;
use nimble_protocol_header::ClientTime;
use std::cell::RefCell;
use std::fmt::Debug;
use std::io;
use std::rc::Rc;

#[derive(Debug)]
pub enum ClientFrontError {
    Unexpected(String),
    DatagramOrderInError(DatagramOrderInError),
    ClientStreamError(ClientStreamError),
    IoError(io::Error),
}

impl From<DatagramOrderInError> for ClientFrontError {
    fn from(err: DatagramOrderInError) -> ClientFrontError {
        ClientFrontError::DatagramOrderInError(err)
    }
}

impl From<ClientStreamError> for ClientFrontError {
    fn from(err: ClientStreamError) -> ClientFrontError {
        ClientFrontError::ClientStreamError(err)
    }
}

impl From<io::Error> for ClientFrontError {
    fn from(err: io::Error) -> ClientFrontError {
        ClientFrontError::IoError(err)
    }
}

pub struct ClientFront<StateT: BufferDeserializer, StepT: Clone + Deserialize + Serialize + Debug> {
    pub client: ClientStream<StateT, StepT>,
    clock: Rc<RefCell<dyn MonotonicClock>>, //pub clock: InstantMonotonicClock,
    ordered_datagram_out: OrderedOut,
    ordered_in: OrderedIn,
    latency: AggregateMetric<u16>,
}

impl<StateT: BufferDeserializer, StepT: Clone + Deserialize + Serialize + Debug>
    ClientFront<StateT, StepT>
{
    pub fn new(application_version: &Version, clock: Rc<RefCell<dyn MonotonicClock>>) -> Self {
        Self {
            clock,
            client: ClientStream::<StateT, StepT>::new(application_version),
            ordered_datagram_out: Default::default(),
            ordered_in: Default::default(),
            latency: AggregateMetric::<u16>::new(10).unwrap(),
        }
    }

    pub fn send(&mut self) -> Result<Vec<Vec<u8>>, ClientFrontError> {
        let mut packet = [0u8; 1200];
        let mut out_datagrams: Vec<Vec<u8>> = vec![];

        let datagrams = self.client.send()?;
        for datagram in &datagrams {
            let mut stream = OutOctetStream::new(); // TODO: implement self.stream.clear()

            // Serialize
            self.ordered_datagram_out.to_stream(&mut stream)?; // Ordered datagrams
            let now = self.clock.borrow_mut().now();
            let client_time = ClientTime::new(now.to_lower());
            client_time.serialize(&mut stream)?;

            packet[0..4].copy_from_slice(stream.octets_ref());
            packet[4..4 + datagram.len()].copy_from_slice(datagram);

            out_datagrams.push(packet[0..4 + datagram.len()].to_vec());
            self.ordered_datagram_out.commit();
        }

        Ok(out_datagrams)
    }

    pub fn receive(&mut self, datagram: &[u8]) -> Result<(), ClientFrontError> {
        let mut in_stream = InOctetStream::new(datagram);
        self.ordered_in.read_and_verify(&mut in_stream)?;
        let client_time = ClientTime::deserialize(&mut in_stream)?;

        let now = self.clock.borrow_mut().now();
        let low_16 = client_time.inner() as MillisLow16;
        let earlier = now
            .from_lower(low_16)
            .ok_or_else(|| ClientFrontError::Unexpected("from_lower_error".to_string()))?;
        let duration_ms = now
            .checked_duration_since_ms(earlier)
            .ok_or_else(|| ClientFrontError::Unexpected("earlier".to_string()))?;

        self.latency.add(duration_ms.milliseconds() as u16);

        info!("values: {:?}", self.latency.values());

        Ok(())
    }

    pub fn latency(&self) -> Option<(u16, f32, u16)> {
        self.latency.values()
    }
}
