/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use std::io::Result;

/// Trait for sending datagrams.
///
/// The `DatagramSender` trait provides a standardized interface for transmitting
/// datagram-based messages, (e.g. UDP communication). Implementors
/// handle the specifics of datagram transmission, including serialization and
/// interfacing with underlying transport protocols.
pub trait DatagramSender {
    /// Sends a datagram containing the provided data.
    ///
    /// # Arguments
    ///
    /// * `buf` - An octet slice to be sent as a datagram.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the datagram was sent successfully.
    /// * `Err(io::Error)` if an error occurred during sending.
    fn send(&mut self, buf: &[u8]) -> Result<()>;
}

/// Trait for receiving datagrams.
///
/// The `DatagramReceiver` trait provides a standardized interface for receiving
/// datagram-based messages, (e.g. UDP communication). Implementors
/// of this trait handle the specifics of datagram reception, including deserialization
/// and interfacing with underlying transport protocols.
pub trait DatagramReceiver {
    /// Receives a datagram and stores it into the provided buffer.
    ///
    /// # Arguments
    ///
    /// * `buf` - A mutable byte slice where the received datagram will be stored.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written to the buffer.
    /// * `Err(io::Error)` - If an error occurred during reception.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if the datagram could not be received. Common error
    /// scenarios include network failures, invalid data formats, or issues with
    /// the underlying transport mechanism.
    fn receive(&mut self, buf: &mut [u8]) -> Result<usize>;
}

/// A trait that combines sending and receiving datagrams.
pub trait DatagramCommunicator: DatagramSender + DatagramReceiver {}

impl<T> DatagramCommunicator for T where T: DatagramSender + DatagramReceiver {}

/// Trait for encoding datagrams.
pub trait DatagramEncoder {
    fn encode(&mut self, buf: &[u8]) -> Result<Vec<u8>>;
}

/// Trait for decoding datagrams.
pub trait DatagramDecoder {
    fn decode(&mut self, buf: &[u8]) -> Result<Vec<u8>>;
}

pub trait DatagramCodec: DatagramEncoder + DatagramDecoder {}
impl<T> DatagramCodec for T where T: DatagramEncoder + DatagramDecoder {}

/// A trait for parsing and interpreting datagrams. Somewhat similar to DatagramDecoder, but instead
/// it returns part of the same buf instead of creating a new
pub trait DatagramParser {
    /// Parses the entire datagram, validating both the header and footer.
    ///
    /// # Arguments
    ///
    /// * `buf` - A slice containing the entire datagram.
    ///
    /// # Returns
    ///
    /// * `io::Result<&'a [u8]>` - The slice that is available for reading.
    fn parse<'a>(&mut self, buf: &'a [u8]) -> Result<&'a [u8]>;
}

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum DatagramError {
    BufferFull,
    ItemSizeTooBig,
    IoError(io::Error),
    OtherError(String),
}

impl fmt::Display for DatagramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatagramError::BufferFull => write!(f, "Buffer is full"),
            DatagramError::IoError(err) => write!(f, "I/O error: {}", err),
            DatagramError::OtherError(err) => write!(f, "strange error: {}", err),
            DatagramError::ItemSizeTooBig => write!(f, "Item size is too big"),
        }
    }
}

impl From<io::Error> for DatagramError {
    fn from(err: io::Error) -> Self {
        DatagramError::IoError(err)
    }
}

impl std::error::Error for DatagramError {}

/// A trait for building datagrams by adding headers and footers. Somewhat similar to
/// DatagramCodec, but uses an internal buffer instead of returning a new Vec<u8>.
///
/// Implementations of this trait are responsible for writing the necessary
/// headers and footers to an octet buffer, defining where the payload can be
/// written, and finalizing the datagram so it is ready for transmission.
pub trait DatagramBuilder {
    /// Pushes data into the current datagram.
    ///
    /// # Arguments
    ///
    /// * `data` - A slice of data to be added to the datagram.
    ///
    /// # Returns
    ///
    /// * `io::Result<DatagramError>` - Indicates success or failure.
    fn push(&mut self, data: &[u8]) -> std::result::Result<(), DatagramError>;

    /// Finalizes the datagram and returns the current octets of the datagram.
    ///
    /// # Returns
    ///
    /// * `&[u8]` - The octets of the datagram.
    fn finalize(&mut self) -> io::Result<Vec<u8>>;

    // Checks if at least one push has happened after new() or clear()
    fn is_empty(&self) -> bool;

    /// Clears the buffer and writes a new header to be able to start a new datagram.
    fn clear(&mut self) -> io::Result<()>;
}
