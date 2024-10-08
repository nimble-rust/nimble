/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use err_rs::{most_severe_error, ErrorLevel, ErrorLevelProvider};
use nimble_blob_stream::in_logic_front::FrontLogicError;
use nimble_blob_stream::prelude::BlobError;
use nimble_protocol::ClientRequestId;
use nimble_steps::StepsError;
use std::{fmt, io};

#[derive(Debug)]
pub enum ClientError {
    Single(ClientErrorKind),
    Multiple(Vec<ClientErrorKind>),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(error) => std::fmt::Display::fmt(&error, f),
            Self::Multiple(errors) => {
                writeln!(f, "Multiple errors occurred:")?;

                for (index, error) in errors.iter().enumerate() {
                    writeln!(f, "{}: {}", index + 1, error)?;
                }

                Ok(())
            }
        }
    }
}

impl ErrorLevelProvider for ClientError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            ClientError::Single(err) => err.error_level(),
            ClientError::Multiple(errors) => most_severe_error(errors).expect("REASON"),
        }
    }
}

#[derive(Debug)]
pub enum ClientErrorKind {
    IoErr(io::Error),
    WrongConnectResponseRequestId(ClientRequestId),
    WrongDownloadRequestId,
    DownloadResponseWasUnexpected,
    UnexpectedBlobChannelCommand,
    BlobError(BlobError),
    FrontLogicErr(FrontLogicError),
    StepsError(StepsError),
    ReceivedConnectResponseWhenNotConnecting,
}

impl From<BlobError> for ClientErrorKind {
    fn from(err: BlobError) -> Self {
        Self::BlobError(err)
    }
}

impl From<StepsError> for ClientErrorKind {
    fn from(err: StepsError) -> Self {
        Self::StepsError(err)
    }
}

impl From<FrontLogicError> for ClientErrorKind {
    fn from(err: FrontLogicError) -> Self {
        Self::FrontLogicErr(err)
    }
}

impl From<io::Error> for ClientErrorKind {
    fn from(err: io::Error) -> Self {
        Self::IoErr(err)
    }
}

impl ErrorLevelProvider for ClientErrorKind {
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
        }
    }
}

impl fmt::Display for ClientErrorKind {
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
        }
    }
}

impl std::error::Error for ClientErrorKind {} // it implements Debug and Display
impl std::error::Error for ClientError {} // it implements Debug and Display
