/*

pub struct Front {
            clock: InstantMonotonicClock::new(),
}


pub fn later() {
    let now = self.clock.now();
    let low_16 = datagram_header.client_time.inner() as MillisLow16;
    let earlier = now.from_lower(low_16).ok_or_else(|| ClientStreamError::Unexpected("from_lower_error".to_string()))?;
    let duration_ms = now.checked_duration_since_ms(earlier).ok_or_else(|| ClientStreamError::Unexpected("earlier".to_string()))?;
}


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

    pub fn parse(
        &mut self,
        datagram: &[u8],
    ) -> Result<(DatagramHeader, InOctetStream), DatagramOrderInError> {
        trace!("datagram. parse payload: {}", format_hex(datagram));
        let mut in_stream = InOctetStream::new(datagram);
        self.ordered_in.read_and_verify(&mut in_stream)?;
        let client_time =
            ClientTime::deserialize(&mut in_stream).map_err(DatagramOrderInError::IoError)?;

        let datagram_type = DatagramHeader {
            client_time,
            dropped_packets: 0,
        };

        Ok((datagram_type, in_stream))
    }
}


*/


