/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::combinator::Combinator;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_participant::ParticipantId;

use nimble_step::Step;
use nimble_step_types::StepForParticipants;
use nimble_steps::{Steps, StepsError};
use tick_id::TickId;

#[derive(Debug)]
pub enum HostCombinatorError {
    NoBufferForParticipant,
    StepsError(StepsError),
}

impl From<StepsError> for HostCombinatorError {
    fn from(error: StepsError) -> Self {
        Self::StepsError(error)
    }
}

impl ErrorLevelProvider for HostCombinatorError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::NoBufferForParticipant => ErrorLevel::Warning,
            Self::StepsError(_) => ErrorLevel::Critical,
        }
    }
}

#[allow(unused)]
pub struct HostCombinator<T: Clone + std::fmt::Display> {
    combinator: Combinator<T>,
    authoritative_steps: Steps<StepForParticipants<Step<T>>>,
}

#[allow(unused)]
impl<T: Clone + std::fmt::Display> HostCombinator<T> {
    pub fn new(tick_id: TickId) -> Self {
        Self {
            combinator: Combinator::<T>::new(tick_id),
            authoritative_steps: Steps::new(),
        }
    }

    pub fn tick_id_to_produce(&self) -> TickId {
        self.combinator.tick_id_to_produce
    }

    pub fn create_buffer(&mut self, participant_id: ParticipantId) {
        self.combinator.create_buffer(participant_id)
    }

    pub fn get_mut(&mut self, participant_id: &ParticipantId) -> Option<&mut Steps<T>> {
        self.combinator.in_buffers.get_mut(participant_id)
    }

    pub fn authoritative_steps(&self) -> &Steps<StepForParticipants<Step<T>>> {
        &self.authoritative_steps
    }

    pub fn produce_authoritative_steps(&mut self) {
        for _ in 0..10 {
            if let Ok((produced_tick_id, new_combined_step)) = self.combinator.produce() {
                self.authoritative_steps
                    .push(produced_tick_id, new_combined_step);
            } else {
                break;
            }
        }
    }
}
