/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
#[derive(Debug)]
pub enum BlobError {
    InvalidChunkIndex(usize, usize),
    UnexpectedChunkSize(usize, usize, usize),
    OutOfBounds,
    RedundantContentDiffers(ChunkIndex),
}

impl fmt::Display for BlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChunkIndex(index, max) => {
                write!(f, "illegal chunk index: {index} (max: {max})")
            }
            Self::UnexpectedChunkSize(expected, found, id) => write!(
                f,
                "unexpected chunk size. expected {expected} but encountered {found} for chunk {id}"
            ),
            Self::OutOfBounds => write!(f, "calculated slice range is out of bounds"),
            Self::RedundantContentDiffers(chunk_index) => write!(f, "chunk {chunk_index} has already been received, but now received different content for that chunk. this is serious"),
        }
    }
}

impl Error for BlobError {} // it implements Debug and Display

use crate::ChunkIndex;
use core::fmt;
use std::error::Error;
use std::io;

impl From<BlobError> for io::Error {
    fn from(err: BlobError) -> Self {
        match err {
            // Map your custom error to an appropriate io::Error kind
            BlobError::InvalidChunkIndex(_, _) => {
                Self::new(io::ErrorKind::InvalidInput, err.to_string())
            }
            BlobError::OutOfBounds => Self::new(io::ErrorKind::UnexpectedEof, err.to_string()),
            BlobError::RedundantContentDiffers(_) | BlobError::UnexpectedChunkSize(_, _, _) => {
                Self::new(io::ErrorKind::InvalidData, err.to_string())
            }
        }
    }
}
