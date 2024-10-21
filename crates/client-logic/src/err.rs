/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_blob_stream::in_logic_front::FrontLogicError;
use nimble_blob_stream::prelude::BlobError;
use nimble_protocol::ClientRequestId;
use std::{fmt, io};
use tick_queue::QueueError;

#[derive(Debug)]
pub enum ClientLogicError {
    IoErr(io::Error),
    WrongJoinResponseRequestId {
        expected: ClientRequestId,
        encountered: ClientRequestId,
    },
    WrongConnectResponseRequestId(ClientRequestId),
    WrongDownloadRequestId,
    DownloadResponseWasUnexpected,
    UnexpectedBlobChannelCommand,
    BlobError(BlobError),
    FrontLogicErr(FrontLogicError),
    QueueError(QueueError),
    ReceivedConnectResponseWhenNotConnecting,
    CanNotPushEmptyPredictedSteps,
    MillisFromLowerError,
    AbsoluteTimeError,
    TooManyAuthoritativeSteps,
    LatencyIsTooBig,
    TooManyStepsInRange,
}

impl From<BlobError> for ClientLogicError {
    fn from(err: BlobError) -> Self {
        Self::BlobError(err)
    }
}

impl From<QueueError> for ClientLogicError {
    fn from(err: QueueError) -> Self {
        Self::QueueError(err)
    }
}

impl From<FrontLogicError> for ClientLogicError {
    fn from(err: FrontLogicError) -> Self {
        Self::FrontLogicErr(err)
    }
}

impl From<io::Error> for ClientLogicError {
    fn from(err: io::Error) -> Self {
        Self::IoErr(err)
    }
}

impl ErrorLevelProvider for ClientLogicError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::IoErr(_)
            | Self::QueueError(_)
            | Self::CanNotPushEmptyPredictedSteps
            | Self::TooManyAuthoritativeSteps
            | Self::TooManyStepsInRange => ErrorLevel::Critical,
            Self::WrongConnectResponseRequestId(_)
            | Self::DownloadResponseWasUnexpected
            | Self::UnexpectedBlobChannelCommand
            | Self::ReceivedConnectResponseWhenNotConnecting
            | Self::WrongJoinResponseRequestId { .. } => ErrorLevel::Info,
            Self::WrongDownloadRequestId
            | Self::BlobError(_)
            | Self::MillisFromLowerError
            | Self::AbsoluteTimeError
            | Self::LatencyIsTooBig => ErrorLevel::Warning,
            Self::FrontLogicErr(err) => err.error_level(),
        }
    }
}

impl fmt::Display for ClientLogicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoErr(io_err) => {
                write!(f, "io:err {io_err:?}")
            }
            Self::WrongConnectResponseRequestId(nonce) => {
                write!(f, "wrong nonce in reply to connect {nonce:?}")
            }
            Self::WrongDownloadRequestId => {
                write!(f, "WrongDownloadRequestId")
            }
            Self::DownloadResponseWasUnexpected => {
                write!(f, "DownloadResponseWasUnexpected")
            }
            Self::UnexpectedBlobChannelCommand => write!(f, "UnexpectedBlobChannelCommand"),
            Self::FrontLogicErr(err) => write!(f, "front logic err {err:?}"),
            Self::QueueError(err) => write!(f, "Steps queue err: {err:?}"),
            Self::ReceivedConnectResponseWhenNotConnecting => {
                write!(f, "ReceivedConnectResponseWhenNotConnecting")
            }
            Self::BlobError(_) => write!(f, "BlobError"),
            Self::WrongJoinResponseRequestId {
                expected,
                encountered,
            } => write!(
                f,
                "wrong join response, expected {expected:?}, encountered: {encountered:?}"
            ),
            Self::CanNotPushEmptyPredictedSteps => write!(f, "CanNotPushEmptyPredictedSteps"),
            Self::MillisFromLowerError => write!(f, "millis from lower"),
            Self::AbsoluteTimeError => write!(f, "absolute time"),
            Self::TooManyAuthoritativeSteps => write!(f, "TooManyAuthoritativeSteps"),
            Self::LatencyIsTooBig => write!(f, "Latency Is Too Big"),
            Self::TooManyStepsInRange => write!(f, "Too ManySteps"),
        }
    }
}

impl std::error::Error for ClientLogicError {} // it implements Debug and Display
