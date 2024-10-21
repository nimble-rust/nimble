/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::OutOctetStream;
use flood_rs::Serialize;
use hexify::{assert_eq_slices, format_hex};
use log::info;
use metricator::MinMaxAvg;
use monotonic_time_rs::Millis;
use nimble_client_logic::err::ClientLogicError;
use nimble_client_logic::ClientLogic;
use nimble_layer::{NimbleLayer, NimbleLayerError};
use nimble_sample_step::{SampleState, SampleStep};

fn send(
    now: Millis,
    logic: &mut ClientLogic<SampleState, SampleStep>,
    layer: &mut NimbleLayer,
) -> Result<Vec<Vec<u8>>, ClientLogicError> {
    let commands = logic.send(now);
    let mut chunker = datagram_chunker::DatagramChunker::new(1024);
    for command in commands {
        let mut out_stream = OutOctetStream::new();
        command.serialize(&mut out_stream)?;
        chunker
            .push(out_stream.octets_ref())
            .expect("TODO: panic message");
    }

    let datagrams = layer.send(&chunker.finalize())?;
    Ok(datagrams)
}

#[derive(Debug)]
pub enum TestError {
    ClientLogicError(ClientLogicError),
    NimbleLayerError(NimbleLayerError),
}

impl From<NimbleLayerError> for TestError {
    fn from(error: NimbleLayerError) -> Self {
        Self::NimbleLayerError(error)
    }
}

impl From<ClientLogicError> for TestError {
    fn from(kind: ClientLogicError) -> TestError {
        Self::ClientLogicError(kind)
    }
}

#[test_log::test]
pub fn client_connect_with_layer() -> Result<(), TestError> {
    let app_version = app_version::Version::new(0, 0, 0);

    let mut now = Millis::new(0);
    let mut layer = NimbleLayer::new();
    let mut logic = ClientLogic::<SampleState, SampleStep>::new(app_version);

    let datagrams = send(now, &mut logic, &mut layer)?;

    let datagram = &datagrams[0];

    info!("received: {}", format_hex(datagram));
    let expected: &[u8] = &[
        // Header
        0x00, 0x00, // Datagram ID
        // Commands
        0x05, // Connect
        0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // Nimble Version
        0x00, // Flags
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Application Version
        0x00, // Request ID
    ];

    assert_eq_slices(datagram, expected);

    now = Millis::from(0xf000);

    let datagrams_after = send(now, &mut logic, &mut layer)?;

    info!("datagrams_after: {}", format_hex(&datagrams_after[0]));

    let expected_after: &[u8] = &[
        // Header
        0x00, 0x01, // Datagram ID
        // Commands
        0x05, // Connect
        0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // Nimble Version
        0x00, // Flags
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Application Version
        0x00, // Request ID
    ];

    assert_eq_slices(&datagrams_after[0], expected_after);

    for index in 1u8..11 {
        let absolute_time_when_sent_lower_octet = index * 16;
        let feed: &[u8] = &[
            // Header
            0x00,
            2 + index * 3, // Datagram ID
            0xF0,
            absolute_time_when_sent_lower_octet, // Client Time
        ];

        now = Millis::from(0xf000 + 200u64 + index as u64 * 20);
        layer.receive(&feed)?;
        if index % 2 == 0 {
            let _ = send(now, &mut logic, &mut layer)?;
        }
    }

    // assert_eq!(header.in_datagrams_per_second(), 50.0);

    assert_eq!(
        layer.datagram_drops().expect("values should be set by now"),
        MinMaxAvg::new(2, 2.3, 5)
    );

    // assert_eq!(metrics.out_octets_per_second(), 380.0);
    //
    // assert_eq!(metrics.out_datagrams_per_second(), 20.0);
    //
    // assert_eq!(metrics.in_octets_per_second(), 200.0);
    //
    // assert_eq!(metrics.in_datagrams_per_second(), 50.0);

    Ok(())
}
