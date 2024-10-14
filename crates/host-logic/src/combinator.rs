/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use err_rs::{ErrorLevel, ErrorLevelProvider};
use log::trace;
use seq_map::SeqMapError;
use std::collections::HashMap;
use tick_id::TickId;

use nimble_participant::ParticipantId;
use nimble_step::Step;
use nimble_step_types::StepForParticipants;
use nimble_steps::{Steps, StepsError};

#[derive(Debug)]
pub enum CombinatorError {
    NotReadyToProduceStep {
        can_provide: usize,
        can_not_provide: usize,
    },
    OtherError,
    SeqMapError(SeqMapError),
    NoBufferForParticipant,
    StepsError(StepsError),
}

impl From<StepsError> for CombinatorError {
    fn from(e: StepsError) -> Self {
        Self::StepsError(e)
    }
}

impl ErrorLevelProvider for CombinatorError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::NotReadyToProduceStep { .. } => ErrorLevel::Info,
            Self::OtherError => ErrorLevel::Info,
            Self::SeqMapError(_) => ErrorLevel::Info,
            Self::NoBufferForParticipant => ErrorLevel::Info,
            Self::StepsError(_) => ErrorLevel::Critical,
        }
    }
}

impl From<SeqMapError> for CombinatorError {
    fn from(value: SeqMapError) -> Self {
        Self::SeqMapError(value)
    }
}

#[derive(Default)]
pub struct Combinator<T: Clone> {
    pub in_buffers: HashMap<ParticipantId, Steps<T>>,
    pub tick_id_to_produce: TickId,
}

impl<T: Clone + std::fmt::Display> Combinator<T> {
    pub fn new(tick_id_to_produce: TickId) -> Self {
        Combinator {
            in_buffers: HashMap::new(),
            tick_id_to_produce,
        }
    }

    pub fn create_buffer(&mut self, id: ParticipantId) {
        self.in_buffers.insert(id, Steps::new());
    }

    pub fn add(
        &mut self,
        id: ParticipantId,
        tick_id: TickId,
        step: T,
    ) -> Result<(), CombinatorError> {
        if let Some(buffer) = self.in_buffers.get_mut(&id) {
            buffer.push_with_check(tick_id, step)?;
            Ok(())
        } else {
            Err(CombinatorError::NoBufferForParticipant)
        }
    }

    pub fn get_mut(&mut self, id: &ParticipantId) -> Option<&mut Steps<T>> {
        self.in_buffers.get_mut(id)
    }

    pub fn participants_that_can_provide(&self) -> (usize, usize) {
        let mut participant_count_that_can_not_give_step = 0;
        let mut participant_count_that_can_provide_step = 0;
        for (_, steps) in self.in_buffers.iter() {
            if let Some(first_tick) = steps.front_tick_id() {
                if first_tick != self.tick_id_to_produce {
                    participant_count_that_can_not_give_step += 1;
                    continue;
                } else {
                    participant_count_that_can_provide_step += 1;
                }
            } else {
                participant_count_that_can_not_give_step += 1;
            }
        }

        (
            participant_count_that_can_provide_step,
            participant_count_that_can_not_give_step,
        )
    }

    pub fn produce(&mut self) -> Result<(TickId, StepForParticipants<Step<T>>), CombinatorError> {
        let (can_provide, can_not_provide) = self.participants_that_can_provide();
        if can_provide == 0 {
            trace!(
                "notice: can not produce authoritative step {}, no one can provide it",
                self.tick_id_to_produce
            );
            return Err(CombinatorError::NotReadyToProduceStep {
                can_provide,
                can_not_provide,
            });
        }
        trace!(
            "found {} that can provide steps and {} that can not",
            can_provide,
            can_not_provide
        );

        let mut combined_step = StepForParticipants::<Step<T>>::new();
        for (participant_id, steps) in self.in_buffers.iter_mut() {
            if let Some(first_tick) = steps.front_tick_id() {
                if first_tick == self.tick_id_to_produce {
                    trace!(
                        "found step from {} for {}, expecting {}",
                        first_tick,
                        participant_id,
                        steps.front_tick_id().unwrap()
                    );
                    combined_step
                        .combined_step
                        .insert(*participant_id, Step::Custom(steps.pop().unwrap().step))?;
                } else {
                    trace!(
                        "did not find step from {} for {}, setting it to forced",
                        first_tick,
                        participant_id
                    );
                    combined_step
                        .combined_step
                        .insert(*participant_id, Step::Forced)?;
                    steps.pop_up_to(self.tick_id_to_produce);
                }
            }
        }

        self.tick_id_to_produce += 1;

        Ok((self.tick_id_to_produce - 1, combined_step))
    }
}
