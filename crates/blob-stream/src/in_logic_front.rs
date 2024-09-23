/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::in_logic::Logic;
use crate::prelude::BlobError;
use crate::protocol::TransferId;
use crate::protocol_front::{
    AckChunkFrontData, ReceiverToSenderFrontCommands, SenderToReceiverFrontCommands,
};
use crate::ChunkIndex;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use log::{debug, trace};
use std::io;

#[derive(Debug)]
pub enum FrontLogicError {
    IoError(io::Error),
    BlobError(BlobError),
    UnknownTransferId(TransferId),
    ChunkSizeCanNotBeZero,
}

impl ErrorLevelProvider for FrontLogicError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            FrontLogicError::IoError(_) => ErrorLevel::Info,
            FrontLogicError::BlobError(_) => ErrorLevel::Info,
            FrontLogicError::UnknownTransferId(_) => ErrorLevel::Info,
            FrontLogicError::ChunkSizeCanNotBeZero => ErrorLevel::Info,
        }
    }
}

pub struct Info {
    pub transfer_id: TransferId,
    pub fixed_chunk_size: usize,
    pub octet_count: usize,
    pub chunk_count_received: usize,
    pub waiting_for_chunk_index: ChunkIndex,
}

#[derive(Debug)]
pub struct State {
    transfer_id: TransferId,
    logic: Logic,
}

/// `Logic` handles the logic for receiving and processing chunks of data
/// in a streaming context. It manages the internal state and interactions
/// between the sender and receiver commands.
#[derive(Debug, Default)]
pub struct FrontLogic {
    state: Option<State>,
    should_reply_ack: bool,
}

impl FrontLogic {
    /// Creates a new `InLogicFront` instance with the specified `octet_count` and `chunk_size`.
    ///
    /// # Arguments
    ///
    /// * `octet_count` - The total number of octets (bytes) expected in the stream.
    /// * `chunk_size` - The size of each chunk in the stream.
    ///
    /// # Returns
    ///
    /// A new `InLogicFront` instance.
    ///
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: None,
            should_reply_ack: false,
        }
    }

    /// Updates the internal state based on a `SenderToReceiverFrontCommands` command.
    ///
    /// This method processes either a `StartTransfer` or `SetChunk` command sent by the sender.
    /// If a `StartTransfer` command is received, the current state (including `transfer_id` and
    /// `logic`) is reinitialized if necessary. If a `SetChunk` command is received, it applies
    /// the chunk of data to the current logic.
    ///
    /// # Arguments
    ///
    /// * `command` - A command sent by the sender to either start a new transfer or update
    ///               an existing one with a chunk of data.
    ///
    /// # Returns
    ///
    /// On success, this method returns a corresponding response:
    /// * If a `StartTransfer` command is processed, it returns `AckStart` with the `transfer_id`.
    /// * If a `SetChunk` command is processed successfully, it returns `AckChunk` with information
    ///   on the last chunk received in order as well as a receive-mask for up to 64 chunks
    ///   after that.
    ///
    /// # Errors
    ///
    /// This function returns an `io::Error` in the following cases:
    /// * If a `SetChunk` command is received and the transfer state has not been initialized
    ///   (i.e., no `StartTransfer` has been processed), it returns an `io::Error` with
    ///   `ErrorKind::InvalidData` and a message indicating that the `transfer_id` is unknown.
    ///
    /// * Any I/O error encountered during the update of the logic will be propagated.
    ///
    /// # Example
    ///
    /// ```
    /// use nimble_blob_stream::in_logic_front::FrontLogic;
    /// use nimble_blob_stream::protocol::StartTransferData;
    /// use nimble_blob_stream::protocol_front::SenderToReceiverFrontCommands;
    ///
    /// let mut logic_front = FrontLogic::new();
    ///
    /// let start_command = SenderToReceiverFrontCommands::StartTransfer(StartTransferData {
    ///     transfer_id: 1234,
    ///     total_octet_size: 1024,
    ///     chunk_size: 256,
    /// });
    ///
    /// let response = logic_front.receive(&start_command);
    /// assert!(response.is_ok());
    /// ```
    pub fn receive(
        &mut self,
        command: &SenderToReceiverFrontCommands,
    ) -> Result<(), FrontLogicError> {
        match command {
            SenderToReceiverFrontCommands::StartTransfer(start_transfer_data) => {
                if self
                    .state
                    .as_ref()
                    .map_or(true, |s| s.transfer_id.0 != start_transfer_data.transfer_id)
                {
                    debug!(
                        "received a start transfer for {}. sending ack.",
                        start_transfer_data.transfer_id
                    );

                    if start_transfer_data.chunk_size == 0 {
                        Err(FrontLogicError::ChunkSizeCanNotBeZero)?;
                    }
                    // Either logic is not set or the transfer_id is different, so we start with a fresh InLogic.
                    self.state = Some(State {
                        transfer_id: TransferId(start_transfer_data.transfer_id),
                        logic: Logic::new(
                            start_transfer_data.total_octet_size as usize,
                            start_transfer_data.chunk_size as usize,
                        ),
                    });
                    self.should_reply_ack = true;
                }
                Ok(())
            }
            SenderToReceiverFrontCommands::SetChunk(chunk_data) => {
                if let Some(ref mut state) = self.state {
                    trace!(
                        "received chunk {}  (transfer:{})",
                        chunk_data.data.chunk_index,
                        chunk_data.transfer_id.0
                    );
                    state
                        .logic
                        .receive(&chunk_data.data)
                        .map_err(FrontLogicError::BlobError)?;
                    if state.logic.is_complete() {
                        trace!("received all chunks!")
                    }
                    Ok(())
                } else {
                    Err(FrontLogicError::UnknownTransferId(chunk_data.transfer_id))
                }
            }
        }
    }

    pub fn send(&mut self) -> Option<ReceiverToSenderFrontCommands> {
        if self.should_reply_ack {
            self.should_reply_ack = false;
            let transfer_id = self.state.as_ref().unwrap().transfer_id.0;
            Some(ReceiverToSenderFrontCommands::AckStart(transfer_id))
        } else if let Some(state) = self.state.as_mut() {
            let ack = &state.logic.send();
            Some(ReceiverToSenderFrontCommands::AckChunk(AckChunkFrontData {
                transfer_id: state.transfer_id,
                data: *ack,
            }))
        } else {
            None
        }
    }

    /// Retrieves the full blob data if all chunks have been received.
    ///
    /// # Returns
    ///
    /// An `Some(&[u8])` containing the full blob data if all chunks have been received,
    /// or `None` if the blob is incomplete.
    #[must_use]
    pub fn blob(&self) -> Option<&[u8]> {
        self.state.as_ref().and_then(|state| state.logic.blob())
    }

    #[must_use]
    pub fn info(&self) -> Option<Info> {
        self.state.as_ref().map(|s| {
            let info = s.logic.info();
            Info {
                transfer_id: s.transfer_id,
                fixed_chunk_size: info.chunk_octet_size,
                octet_count: info.total_octet_size,
                chunk_count_received: info.chunk_count_received,
                waiting_for_chunk_index: info.waiting_for_chunk_index,
            }
        })
    }
}
