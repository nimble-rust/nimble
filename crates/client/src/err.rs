/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use datagram_chunker::DatagramChunkerError;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_client_logic::err::ClientLogicError;
use nimble_layer::NimbleLayerError;
use nimble_rectify::RectifyError;
use tick_queue::QueueError;

#[derive(Debug)]
pub enum ClientError {
    IoError(std::io::Error),
    RectifyError(RectifyError),
    ClientLogicErrorKind(ClientLogicError),
    QueueError(QueueError),
    DatagramChunkerError(DatagramChunkerError),
    NimbleLayerError(NimbleLayerError),
    PredictionQueueOverflow,
}

impl From<DatagramChunkerError> for ClientError {
    fn from(value: DatagramChunkerError) -> Self {
        Self::DatagramChunkerError(value)
    }
}

impl From<NimbleLayerError> for ClientError {
    fn from(value: NimbleLayerError) -> Self {
        Self::NimbleLayerError(value)
    }
}

impl ErrorLevelProvider for ClientError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::IoError(_) => ErrorLevel::Info,
            Self::RectifyError(err) => err.error_level(),
            Self::ClientLogicErrorKind(_) => ErrorLevel::Info,
            Self::QueueError(_) => ErrorLevel::Info,
            Self::DatagramChunkerError(_) => ErrorLevel::Info,
            Self::NimbleLayerError(_) => ErrorLevel::Info,
            Self::PredictionQueueOverflow => ErrorLevel::Critical,
        }
    }
}

impl From<RectifyError> for ClientError {
    fn from(err: RectifyError) -> Self {
        ClientError::RectifyError(err)
    }
}

impl From<QueueError> for ClientError {
    fn from(err: QueueError) -> Self {
        Self::QueueError(err)
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ClientLogicError> for ClientError {
    fn from(err: ClientLogicError) -> Self {
        Self::ClientLogicErrorKind(err)
    }
}