/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use err_rs::{ErrorLevel, ErrorLevelProvider};
use log::trace;
use nimble_participant::ParticipantId;
use nimble_step::Step;
use nimble_step_map::StepMap;
use seq_map::SeqMapError;
use std::collections::HashMap;
use tick_id::TickId;
use tick_queue::{Queue, QueueError};

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)] // TODO: rename CombinatorError
pub enum CombinatorError {
    NotReadyToProduceStep {
        can_provide: usize,
        can_not_provide: usize,
    },
    OtherError,
    SeqMapError(SeqMapError),
    NoBufferForParticipant,
    QueueError(QueueError),
}

impl From<QueueError> for CombinatorError {
    fn from(e: QueueError) -> Self {
        Self::QueueError(e)
    }
}

impl ErrorLevelProvider for CombinatorError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::NotReadyToProduceStep { .. }
            | Self::OtherError
            | Self::SeqMapError(_)
            | Self::NoBufferForParticipant => ErrorLevel::Info,
            Self::QueueError(_) => ErrorLevel::Critical,
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
    pub in_buffers: HashMap<ParticipantId, Queue<T>>,
    pub tick_id_to_produce: TickId,
}

impl<T: Clone + std::fmt::Display> Combinator<T> {
    #[must_use]
    pub fn new(tick_id_to_produce: TickId) -> Self {
        Self {
            in_buffers: HashMap::new(),
            tick_id_to_produce,
        }
    }

    pub fn create_buffer(&mut self, id: ParticipantId) {
        self.in_buffers.insert(id, Queue::default());
    }

    /// # Errors
    ///
    /// `CombinatorError` // TODO:
    pub fn add(
        &mut self,
        id: ParticipantId,
        tick_id: TickId,
        step: T,
    ) -> Result<(), CombinatorError> {
        if let Some(buffer) = self.in_buffers.get_mut(&id) {
            buffer.push(tick_id, step)?;
            Ok(())
        } else {
            Err(CombinatorError::NoBufferForParticipant)
        }
    }

    pub fn get_mut(&mut self, id: &ParticipantId) -> Option<&mut Queue<T>> {
        self.in_buffers.get_mut(id)
    }

    #[must_use]
    pub fn participants_that_can_provide(&self) -> (usize, usize) {
        let mut participant_count_that_can_not_give_step = 0;
        let mut participant_count_that_can_provide_step = 0;
        for steps in self.in_buffers.values() {
            if let Some(first_tick) = steps.front_tick_id() {
                if first_tick == self.tick_id_to_produce {
                    participant_count_that_can_provide_step += 1;
                } else {
                    participant_count_that_can_not_give_step += 1;
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

    /// # Errors
    ///
    /// `CombinatorError` // TODO:
    #[allow(clippy::missing_panics_doc)]
    pub fn produce(&mut self) -> Result<(TickId, StepMap<Step<T>>), CombinatorError> {
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

        let mut combined_step = StepMap::<Step<T>>::new();
        for (participant_id, steps) in &mut self.in_buffers {
            if let Some(first_tick) = steps.front_tick_id() {
                if first_tick == self.tick_id_to_produce {
                    trace!(
                        "found step from {} for {}, expecting {}",
                        first_tick,
                        participant_id,
                        steps.front_tick_id().unwrap()
                    );
                    combined_step
                        .insert(*participant_id, Step::Custom(steps.pop().unwrap().item))?;
                } else {
                    trace!(
                        "did not find step from {} for {}, setting it to forced",
                        first_tick,
                        participant_id
                    );
                    combined_step.insert(*participant_id, Step::Forced)?;
                    steps.discard_up_to(self.tick_id_to_produce);
                }
            }
        }

        self.tick_id_to_produce += 1;

        Ok((self.tick_id_to_produce - 1, combined_step))
    }
}
