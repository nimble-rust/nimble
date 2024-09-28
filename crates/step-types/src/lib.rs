use nimble_participant::ParticipantId;
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct AuthoritativeStep<StepT> {
    pub authoritative_participants: HashMap<ParticipantId, StepT>,
}

pub type LocalIndex = u8;

#[derive(Debug, PartialEq, Clone)]
pub struct PredictedStep<StepT> {
    pub predicted_players: HashMap<LocalIndex, StepT>,
}

impl<StepT> PredictedStep<StepT> {
    pub fn is_empty(&self) -> bool {
        self.predicted_players.is_empty()
    }
}
