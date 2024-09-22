/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram::{DatagramBuilder, DatagramError, DatagramParser};
use datagram_builder::deserialize::deserialize_datagrams;
use datagram_builder::serialize::serialize_datagrams;
use flood_rs::prelude::*;
use std::io;

#[derive(Debug, PartialEq)]
enum TestItem {
    Name { input: String },
    Amount(i16),
}

impl Serialize for TestItem {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        match self {
            TestItem::Name { input } => {
                stream.write_u8(0x01)?;
                stream.write_u8(input.len() as u8)?;
                stream.write(input.as_bytes())?;
            }
            TestItem::Amount(amount) => {
                stream.write_u8(0x02)?;
                stream.write_i16(*amount)?;
            }
        }
        Ok(())
    }
}

impl Deserialize for TestItem {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let type_id = stream.read_u8()?;
        let cmd = match type_id {
            0x01 => {
                let len = stream.read_u8()? as usize;
                let mut vec = vec![0u8; len];
                stream.read(&mut vec)?;
                Self::Name {
                    input: String::from_utf8(vec)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                }
            }

            0x02 => {
                let amount = stream.read_i16()?;
                Self::Amount(amount)
            }

            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unknown type"))?,
        };
        Ok(cmd)
    }
}

pub struct ExampleDatagramBuilder {
    buffer: Vec<u8>,
    max_size: usize,
}

impl ExampleDatagramBuilder {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(max_size),
            max_size,
        }
    }
}

impl DatagramBuilder for ExampleDatagramBuilder {
    fn push(&mut self, data: &[u8]) -> Result<(), DatagramError> {
        const FOOTER_SIZE: usize = 1;

        if data.len() > self.max_size - FOOTER_SIZE {
            return Err(DatagramError::ItemSizeTooBig);
        }

        if self.buffer.len() + data.len() > self.max_size - FOOTER_SIZE {
            return Err(DatagramError::BufferFull);
        }

        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn finalize(&mut self) -> io::Result<Vec<u8>> {
        // Finalize header
        self.buffer.push(0x00); // Signals end of datagram
        self.buffer[0] = 0x02;
        Ok(self.buffer.clone())
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn clear(&mut self) -> io::Result<()> {
        self.buffer.clear();
        self.buffer.extend_from_slice(&[0x01, 0x0ff]);
        Ok(())
    }
}

struct ExampleDatagramParser;

impl DatagramParser for ExampleDatagramParser {
    fn parse<'a>(&mut self, datagram: &'a [u8]) -> io::Result<&'a [u8]> {
        assert_eq!(datagram[0], 0x02);
        assert_eq!(datagram[1], 0xff);

        Ok(datagram[2..datagram.len() - 1].as_ref())
    }
}

#[test_log::test]
fn serialize_single_datagram() {
    let items = [
        TestItem::Name {
            input: "foo".into(),
        },
        TestItem::Amount(42),
        TestItem::Name {
            input: "bar".into(),
        },
    ];
    const EXPECTED_DATAGRAM: &[u8] = &[
        0x02, 0xff, // Header
        0x01, 3, b'f', b'o', b'o', 0x02, 0x00, 42, 0x01, 3, b'b', b'a', b'r', 0x00,
    ];
    let mut builder = ExampleDatagramBuilder::new(1024);
    let datagrams = serialize_datagrams(&items, &mut builder).expect("serialization failed");
    assert_eq!(datagrams.len(), 1);
    assert_eq!(datagrams[0], EXPECTED_DATAGRAM);

    let mut parser = ExampleDatagramParser;
    let deserialized_items: Vec<TestItem> =
        deserialize_datagrams(datagrams, &mut parser).expect("deserialization failed");
    assert_eq!(items.len(), deserialized_items.len());
    assert_eq!(*deserialized_items, items);
}

#[test_log::test]
fn serialize_and_deserialize_multiple_datagrams() {
    const MAX_PACKET_SIZE: usize = 8;
    let items = [
        TestItem::Name {
            input: "foo".into(),
        },
        TestItem::Amount(42),
        TestItem::Name {
            input: "bar".into(),
        },
    ];

    const EXPECTED_DATAGRAMS: [&[u8]; 3] = [
        &[
            0x02, 0xff, // header
            0x01, 3, 'f' as u8, 'o' as u8, 'o' as u8, 0x00, // end of datagram
        ],
        &[
            0x02, 0xff, // header
            0x02, 0x00, 42, 0x00, // end of datagram
        ],
        &[
            0x02, 0xff, // header
            0x01, 3, 'b' as u8, 'a' as u8, 'r' as u8, 0x00, // end of datagram
        ],
    ];

    const EXPECTED_DATAGRAM_COUNT: usize = 3;

    let mut builder = ExampleDatagramBuilder::new(MAX_PACKET_SIZE);
    let datagrams = serialize_datagrams(&items, &mut builder).expect("serialization failed");
    assert_eq!(datagrams.len(), EXPECTED_DATAGRAM_COUNT);

    for i in 0..EXPECTED_DATAGRAM_COUNT {
        assert_eq!(datagrams[i].len(), EXPECTED_DATAGRAMS[i].len());
        assert_eq!(datagrams[i], EXPECTED_DATAGRAMS[i]);
    }

    let mut parser = ExampleDatagramParser;
    let deserialized_items: Vec<TestItem> =
        deserialize_datagrams(datagrams, &mut parser).expect("deserialization failed");
    assert_eq!(items.len(), deserialized_items.len());
}

#[test_log::test]
fn too_big_item_size() {
    const MAX_PACKET_SIZE: usize = 4;
    let items = [
        TestItem::Name {
            input: "foo".into(),
        },
        TestItem::Amount(42),
        TestItem::Name {
            input: "bar".into(),
        },
    ];

    let mut builder = ExampleDatagramBuilder::new(MAX_PACKET_SIZE);
    let result = serialize_datagrams(&items, &mut builder);
    if let Err(ref err) = result {
        println!("{}", err);
    }
    assert!(
        matches!(result, Err(e) if e.kind() == io::ErrorKind::InvalidData && e.to_string() == "Item size is too big")
    );
}
