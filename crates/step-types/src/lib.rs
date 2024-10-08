/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_participant::ParticipantId;
use seq_map::SeqMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct StepForParticipants<StepT: Display> {
    pub combined_step: SeqMap<ParticipantId, StepT>,
}
impl<StepT: Display> StepForParticipants<StepT> {
    pub fn new() -> Self {
        Self {
            combined_step: SeqMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.combined_step.is_empty()
    }
}

impl<StepT: Display> Display for StepForParticipants<StepT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.combined_step)
    }
}

pub type LocalIndex = u8;
