/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::combinator::Combinator;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_participant::ParticipantId;

use nimble_step::Step;
use nimble_step_types::StepForParticipants;
use nimble_steps::Steps;
use tick_id::TickId;

#[derive(Debug)]
pub enum HostCombinatorError {
    NoBufferForParticipant,
}

impl ErrorLevelProvider for HostCombinatorError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            HostCombinatorError::NoBufferForParticipant => ErrorLevel::Warning,
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

    pub fn receive_step(
        &mut self,
        participant_id: ParticipantId,
        step: T,
    ) -> Result<(), HostCombinatorError> {
        if let Some(participant_buffer) = self.combinator.in_buffers.get_mut(&participant_id) {
            participant_buffer.push(step);
            self.produce_authoritative_steps();
            Ok(())
        } else {
            Err(HostCombinatorError::NoBufferForParticipant)
        }
    }

    pub fn authoritative_steps(&self) -> &Steps<StepForParticipants<Step<T>>> {
        &self.authoritative_steps
    }

    fn produce_authoritative_steps(&mut self) {
        for _ in 0..10 {
            if let Ok(new_combined_step) = self.combinator.produce() {
                self.authoritative_steps.push(new_combined_step);
            } else {
                break;
            }
        }
    }
}
