/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_participant::ParticipantId;
use seq_map::SeqMap;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct AuthoritativeStep<StepT> {
    pub authoritative_participants: SeqMap<ParticipantId, StepT>,
}

pub type LocalIndex = u8;

#[derive(Debug, PartialEq, Clone)]
pub struct PredictedStep<StepT> {
    pub predicted_players: SeqMap<LocalIndex, StepT>,
}

impl<StepT> PredictedStep<StepT> {
    pub fn is_empty(&self) -> bool {
        self.predicted_players.is_empty()
    }
}
