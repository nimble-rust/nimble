/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod prelude;

use std::marker::PhantomData;

use log::trace;
use nimble_steps::{Steps, StepsError};
use tick_id::TickId;

pub trait AssentCallback<CombinedStepT> {
    fn on_pre_ticks(&mut self) {}

    fn on_tick(&mut self, step: &CombinedStepT);

    fn on_post_ticks(&mut self) {}
}

#[derive(Debug, PartialEq)]
pub enum UpdateState {
    ConsumedAllKnowledge,
    DidNotConsumeAllKnowledge,
    NoKnowledge,
}

#[derive(Debug, Copy, Clone)]
pub struct Settings {
    pub max_tick_count_per_update: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_tick_count_per_update: 5,
        }
    }
}

// Define the Assent struct
#[derive(Debug)]
pub struct Assent<C, CombinedStepT>
where
    C: AssentCallback<CombinedStepT>,
{
    phantom: PhantomData<C>,
    settings: Settings,
    steps: Steps<CombinedStepT>,
}

impl<C, CombinedStepT> Default for Assent<C, CombinedStepT>
where
    C: AssentCallback<CombinedStepT>,
    CombinedStepT: Clone + std::fmt::Debug + std::fmt::Display,
{
    fn default() -> Self {
        Assent::new(Settings::default())
    }
}

impl<C, CombinedStepT> Assent<C, CombinedStepT>
where
    C: AssentCallback<CombinedStepT>,
    CombinedStepT: Clone + std::fmt::Debug + std::fmt::Display,
{
    pub fn new(settings: Settings) -> Self {
        Assent {
            phantom: PhantomData {},
            steps: Steps::new(),
            settings,
        }
    }

    pub fn push(&mut self, tick_id: TickId, steps: CombinedStepT) -> Result<(), StepsError> {
        self.steps.push_with_check(tick_id, steps)
    }

    pub fn expecting_tick_id(&self) -> TickId {
        self.steps.expected_write_tick_id()
    }

    pub fn end_tick_id(&self) -> Option<TickId> {
        self.steps.back_tick_id()
    }

    pub fn debug_steps(&self) -> &Steps<CombinedStepT> {
        &self.steps
    }

    pub fn update(&mut self, callback: &mut C) -> UpdateState {
        if self.steps.is_empty() {
            trace!("notice: assent steps are empty");
            return UpdateState::NoKnowledge;
        }

        callback.on_pre_ticks();
        trace!("tick start. {} steps in queue.", self.steps.len());
        let mut count = 0;
        while let Some(combined_step_info) = self.steps.pop() {
            trace!("tick: {}", &combined_step_info);
            callback.on_tick(&combined_step_info.step);
            count += 1;
            if count >= self.settings.max_tick_count_per_update {
                trace!("encountered threshold, not simulating all ticks");
                return UpdateState::DidNotConsumeAllKnowledge;
            }
        }

        trace!("consumed all knowledge");
        UpdateState::ConsumedAllKnowledge
    }
}
