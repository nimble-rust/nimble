/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use flood_rs::prelude::OutOctetStream;
use flood_rs::Serialize;
use std::fmt::Debug;
use std::{io, mem};

#[derive(Debug)]
pub enum DatagramChunkerError {
    ItemSizeTooBig,
    IoError(io::Error),
}

pub struct DatagramChunker {
    datagrams: Vec<Vec<u8>>,
    current: Vec<u8>,
    max_size: usize,
}

impl DatagramChunker {
    pub fn new(max_size: usize) -> Self {
        Self {
            current: Vec::with_capacity(max_size),
            datagrams: Vec::new(),
            max_size,
        }
    }

    fn push(&mut self, data: &[u8]) -> Result<(), DatagramChunkerError> {
        if data.len() > self.max_size {
            return Err(DatagramChunkerError::ItemSizeTooBig);
        }

        if self.current.len() + data.len() > self.max_size {
            self.datagrams.push(mem::take(&mut self.current));
            self.current = data.to_vec();
        } else {
            self.current.extend_from_slice(data);
        }

        Ok(())
    }

    pub fn finalize(mut self) -> Vec<Vec<u8>> {
        if !self.current.is_empty() {
            self.datagrams.push(self.current.clone());
        }
        self.datagrams
    }
}

impl From<io::Error> for DatagramChunkerError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

pub fn serialize_to_chunker<I, T>(
    items: I,
    max_datagram_size: usize,
) -> Result<Vec<Vec<u8>>, DatagramChunkerError>
where
    T: Serialize + Debug,
    I: AsRef<[T]>,
{
    let mut chunker = DatagramChunker::new(max_datagram_size);
    for item in items.as_ref() {
        let mut temp = OutOctetStream::new();
        item.serialize(&mut temp)?;
        chunker.push(temp.octets_ref())?;
    }

    Ok(chunker.finalize())
}
