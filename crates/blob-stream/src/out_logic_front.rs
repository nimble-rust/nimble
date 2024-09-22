/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::out_logic::Logic;
use crate::out_stream::OutStreamError;
use crate::prelude::{
    ReceiverToSenderFrontCommands, SenderToReceiverFrontCommands, StartTransferData, TransferId,
};
use log::{debug, trace};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum Phase {
    StartTransfer,
    Transfer,
}

#[allow(unused)]
#[derive(Debug)]
pub struct OutLogicFront {
    out_stream: Logic,
    phase: Phase,
    transfer_id: TransferId,
}

impl OutLogicFront {
    #[allow(unused)]
    pub fn new(
        transfer_id: TransferId,
        fixed_chunk_size: usize,
        resend_duration: Duration,
        blob: &[u8],
    ) -> Self {
        Self {
            out_stream: Logic::new(transfer_id, fixed_chunk_size, resend_duration, blob),
            phase: Phase::StartTransfer,
            transfer_id,
        }
    }

    pub fn receive(
        &mut self,
        command: &ReceiverToSenderFrontCommands,
    ) -> Result<(), OutStreamError> {
        match self.phase {
            Phase::StartTransfer => {
                if let ReceiverToSenderFrontCommands::AckStart(ack_transfer_id) = command {
                    if self.transfer_id.0 == *ack_transfer_id {
                        debug!("received ack for correct transfer id {ack_transfer_id}, start transfer");
                        self.phase = Phase::Transfer;
                    } else {
                        debug!(
                            "received ack for wrong transfer id {ack_transfer_id}, start transfer"
                        );
                    }
                }
            }
            Phase::Transfer => match command {
                ReceiverToSenderFrontCommands::AckChunk(ack_chunk_front) => {
                    self.out_stream.set_waiting_for_chunk_index(
                        ack_chunk_front.data.waiting_for_chunk_index as usize,
                        ack_chunk_front.data.receive_mask_after_last,
                    )?;
                    if self.out_stream.is_received_by_remote() {
                        trace!("blob stream is received by remote! {}", self.transfer_id.0);
                    }
                }
                ReceiverToSenderFrontCommands::AckStart(_) => {}
            },
        }
        Ok(())
    }

    #[allow(unused)]
    pub fn send(
        &mut self,
        now: Instant,
    ) -> Result<Vec<SenderToReceiverFrontCommands>, OutStreamError> {
        match self.phase {
            Phase::StartTransfer => {
                debug!("send start transfer {}", self.transfer_id.0);
                Ok(vec![SenderToReceiverFrontCommands::StartTransfer(
                    StartTransferData {
                        transfer_id: self.transfer_id.0,
                        total_octet_size: self.out_stream.octet_size() as u32,
                        chunk_size: self.out_stream.chunk_size() as u16,
                    },
                )])
            }

            Phase::Transfer => {
                const MAX_CHUNK_COUNT_EACH_SEND: usize = 10;
                let set_chunks: Vec<_> = self
                    .out_stream
                    .send(now, MAX_CHUNK_COUNT_EACH_SEND)
                    .iter()
                    .map(|front_data| SenderToReceiverFrontCommands::SetChunk(front_data.clone()))
                    .collect();
                for set_chunk in &set_chunks {
                    match set_chunk {
                        SenderToReceiverFrontCommands::SetChunk(front_data) => {
                            trace!(
                                "sending chunk {}  (transfer:{})",
                                front_data.data.chunk_index,
                                front_data.transfer_id.0
                            );
                        }
                        _ => panic!("Unexpected enum variant: {:?}", set_chunk),
                    }
                }
                Ok(set_chunks)
            }
        }
    }

    #[must_use]
    pub fn is_received_by_remote(&self) -> bool {
        self.out_stream.is_received_by_remote()
    }

    #[must_use]
    pub fn transfer_id(&self) -> TransferId {
        self.out_stream.transfer_id()
    }
}
