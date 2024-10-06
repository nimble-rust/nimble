/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use seq_map::SeqMapError;
use std::collections::HashMap;
use tick_id::TickId;

use nimble_participant::ParticipantId;
use nimble_step_types::StepForParticipants;
use nimble_steps::{Step, Steps};

#[derive(Debug)]
pub enum CombinatorError {
    NotReadyToProduceStep {
        can_provide: usize,
        can_not_provide: usize,
    },
    OtherError,
    SeqMapError(SeqMapError),
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

impl<T: std::clone::Clone> Combinator<T> {
    pub fn new(tick_id_to_produce: TickId) -> Self {
        Combinator {
            in_buffers: HashMap::new(),
            tick_id_to_produce,
        }
    }

    pub fn create_buffer(&mut self, id: ParticipantId) {
        self.in_buffers.insert(id, Steps::new());
    }

    pub fn add(&mut self, id: ParticipantId, step: T) {
        if let Some(buffer) = self.in_buffers.get_mut(&id) {
            buffer.push(step);
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

    pub fn produce(&mut self) -> Result<StepForParticipants<Step<T>>, CombinatorError> {
        let (can_provide, can_not_provide) = self.participants_that_can_provide();
        if can_provide == 0 {
            return Err(CombinatorError::NotReadyToProduceStep {
                can_provide,
                can_not_provide,
            });
        }

        let mut combined_step = StepForParticipants::<Step<T>>::new();
        for (participant_id, steps) in self.in_buffers.iter_mut() {
            if let Some(first_tick) = steps.front_tick_id() {
                if first_tick == self.tick_id_to_produce {
                    combined_step
                        .combined_step
                        .insert(*participant_id, Step::Custom(steps.pop().unwrap().step))?;
                } else {
                    combined_step
                        .combined_step
                        .insert(*participant_id, Step::Forced)?;
                    steps.pop_up_to(self.tick_id_to_produce);
                }
            }
        }

        self.tick_id_to_produce += 1;

        Ok(combined_step)
    }
}
