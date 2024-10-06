/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_participant::ParticipantId;
use seq_map::SeqMap;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct StepForParticipants<StepT> {
    pub combined_step: SeqMap<ParticipantId, StepT>,
}
impl<StepT> StepForParticipants<StepT> {
    pub fn is_empty(&self) -> bool {
        self.combined_step.is_empty()
    }
}

pub type LocalIndex = u8;
