/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use blob_stream::in_logic_front::FrontLogicError;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_protocol::ClientRequestId;
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

#[derive(Debug)]
pub enum ClientErrorKind {
    Unexpected(String),
    IoErr(io::Error),
    WrongConnectResponseRequestId(ClientRequestId),
    WrongDownloadRequestId,
    DownloadResponseWasUnexpected,
    UnexpectedBlobChannelCommand,
    FrontLogicErr(FrontLogicError),
}

impl ErrorLevelProvider for ClientErrorKind {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::IoErr(_) => ErrorLevel::Critical,
            Self::WrongConnectResponseRequestId(_) => ErrorLevel::Info,
            Self::WrongDownloadRequestId => ErrorLevel::Warning,
            Self::DownloadResponseWasUnexpected => ErrorLevel::Info,
            Self::Unexpected(_) => ErrorLevel::Critical,
            Self::UnexpectedBlobChannelCommand => ErrorLevel::Info,
            Self::FrontLogicErr(err) => err.error_level(),
        }
    }
}

impl fmt::Display for ClientErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected(description) => {
                write!(f, "Unexpected {}", description)
            }
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
        }
    }
}

impl std::error::Error for ClientErrorKind {} // it implements Debug and Display
impl std::error::Error for ClientError {} // it implements Debug and Display
