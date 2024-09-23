/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram::{DatagramBuilder, DatagramError};
use datagram_pinger::ClientTime;
use flood_rs::prelude::OutOctetStream;
use flood_rs::{Serialize, WriteOctetStream};
use hexify::format_hex;
use log::trace;
use nimble_ordered_datagram::OrderedOut;
use std::io;

pub struct NimbleDatagramBuilder {
    ordered_datagram_out: OrderedOut,
    max_size: usize,
    stream: OutOctetStream,
    is_empty: bool, // TODO: Add on OutOctetStream
}

impl NimbleDatagramBuilder {
    pub fn new(max_size: usize) -> Self {
        Self {
            ordered_datagram_out: Default::default(),
            stream: OutOctetStream::new(),
            max_size,
            is_empty: true,
        }
    }

    fn finalize_header(&mut self) -> io::Result<Vec<u8>> {
        let payload = self.stream.octets();
        trace!(
            "datagram. finalize: total    payload: {}",
            format_hex(&payload)
        );
        Ok(payload)
    }
}

impl DatagramBuilder for NimbleDatagramBuilder {
    fn push(&mut self, data: &[u8]) -> Result<(), DatagramError> {
        const FOOTER_SIZE: usize = 1;

        if data.len() > self.max_size - FOOTER_SIZE {
            return Err(DatagramError::ItemSizeTooBig);
        }

        if self.stream.octets().len() + data.len() > self.max_size - FOOTER_SIZE {
            return Err(DatagramError::BufferFull);
        }

        self.stream.write(data)?;
        Ok(())
    }

    fn finalize(&mut self) -> io::Result<Vec<u8>> {
        // Finalize header
        self.finalize_header()
    }

    fn is_empty(&self) -> bool {
        self.is_empty // self.stream.is_empty()
    }

    fn clear(&mut self) -> io::Result<()> {
        self.stream = OutOctetStream::new(); // TODO: implement self.stream.clear()

        self.ordered_datagram_out.to_stream(&mut self.stream)?; // Ordered datagrams

        trace!(
            "datagram header. sequence:{}",
            self.ordered_datagram_out.sequence_to_send
        );

        let client_time = ClientTime::new(0);
        client_time.serialize(&mut self.stream)?;
        self.is_empty = false;

        self.ordered_datagram_out.commit();

        Ok(())
    }
}
