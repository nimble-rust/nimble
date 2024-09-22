/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::combinator::Combinator;
use nimble_participant::ParticipantId;
use nimble_participant_steps::ParticipantSteps;
use nimble_steps::Steps;
use tick_id::TickId;

#[allow(unused)]
pub struct HostCombinator<T> {
    combinator: Combinator<T>,
    authoritative_steps: Steps<ParticipantSteps<T>>,
}

#[allow(unused)]
impl<T> HostCombinator<T> {
    pub fn new() -> Self {
        Self {
            combinator: Combinator::<T>::new(TickId(0)),
            authoritative_steps: Steps::<ParticipantSteps<T>>::new(),
        }
    }

    pub fn receive_step(&mut self, participant_id: ParticipantId, step: T) {
        if let Some(participant_buffer) = self.combinator.in_buffers.get_mut(&participant_id) {
            participant_buffer.push(step);
            self.produce_authoritative_steps();
        }
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

    pub fn get_steps_from(&self, _: TickId) -> Vec<ParticipantSteps<T>> {
        todo!()
    }
}
