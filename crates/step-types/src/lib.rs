/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_step_map::ParticipantId;
use seq_map::SeqMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct StepMap<StepT: Display> {
    seq_map: SeqMap<ParticipantId, StepT>,
}

impl<StepT: Display> StepMap<StepT> {
    /// Creates a new `StepMap`, returning `None` if the given [`SeqMap`] is empty.
    pub fn new(seq_map: SeqMap<ParticipantId, StepT>) -> Option<Self> {
        if seq_map.is_empty() {
            None
        } else {
            Some(Self { seq_map })
        }
    }

    /// Provides a reference to the inner [`SeqMap`] holding the steps.
    pub fn inner(&self) -> &SeqMap<ParticipantId, StepT> {
        &self.seq_map
    }
}

impl<StepT: Display> Display for StepMap<StepT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.seq_map)
    }
}
