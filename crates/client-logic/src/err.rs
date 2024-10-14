/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_blob_stream::in_logic_front::FrontLogicError;
use nimble_blob_stream::prelude::BlobError;
use nimble_protocol::ClientRequestId;
use nimble_steps::StepsError;
use std::{fmt, io};

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
    StepsError(StepsError),
    ReceivedConnectResponseWhenNotConnecting,
}

impl From<BlobError> for ClientLogicError {
    fn from(err: BlobError) -> Self {
        Self::BlobError(err)
    }
}

impl From<StepsError> for ClientLogicError {
    fn from(err: StepsError) -> Self {
        Self::StepsError(err)
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
            Self::IoErr(_) => ErrorLevel::Critical,
            Self::WrongConnectResponseRequestId(_) => ErrorLevel::Info,
            Self::WrongDownloadRequestId => ErrorLevel::Warning,
            Self::DownloadResponseWasUnexpected => ErrorLevel::Info,
            Self::UnexpectedBlobChannelCommand => ErrorLevel::Info,
            Self::FrontLogicErr(err) => err.error_level(),
            Self::StepsError(_) => ErrorLevel::Critical,
            Self::ReceivedConnectResponseWhenNotConnecting => ErrorLevel::Info,
            Self::BlobError(_) => ErrorLevel::Warning,
            Self::WrongJoinResponseRequestId { .. } => ErrorLevel::Info,
        }
    }
}

impl fmt::Display for ClientLogicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoErr(io_err) => {
                write!(f, "io:err {:?}", io_err)
            }
            Self::WrongConnectResponseRequestId(nonce) => {
                write!(f, "wrong nonce in reply to connect {:?}", nonce)
            }
            Self::WrongDownloadRequestId => {
                write!(f, "WrongDownloadRequestId")
            }
            Self::DownloadResponseWasUnexpected => {
                write!(f, "DownloadResponseWasUnexpected")
            }
            Self::UnexpectedBlobChannelCommand => write!(f, "UnexpectedBlobChannelCommand"),
            Self::FrontLogicErr(err) => write!(f, "front logic err {err:?}"),
            Self::StepsError(err) => write!(f, "StepsError: {err:?}"),
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
        }
    }
}

impl std::error::Error for ClientLogicError {} // it implements Debug and Display
