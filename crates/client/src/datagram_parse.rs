/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram_pinger::ClientTime;
use flood_rs::prelude::InOctetStream;
use flood_rs::Deserialize;
use nimble_ordered_datagram::OrderedIn;
use std::io;

pub struct NimbleDatagramParser {
    ordered_in: OrderedIn,
}

pub struct DatagramHeader {
    pub client_time: ClientTime,
    #[allow(unused)]
    pub dropped_packets: usize,
}

impl NimbleDatagramParser {
    pub fn new() -> Self {
        Self {
            ordered_in: OrderedIn::default(),
        }
    }

    pub fn parse(&mut self, datagram: &[u8]) -> io::Result<(DatagramHeader, InOctetStream)> {
        let mut in_stream = InOctetStream::new(datagram);
        self.ordered_in.read_and_verify(&mut in_stream)?;
        let client_time = ClientTime::deserialize(&mut in_stream)?;

        let datagram_type = DatagramHeader {
            client_time,
            dropped_packets: 0,
        };

        Ok((datagram_type, in_stream))
    }
}
