/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod prelude;

use err_rs::{ErrorLevel, ErrorLevelProvider};
use log::trace;
use nimble_steps::{Steps, StepsError};
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use tick_id::TickId;

pub trait SeerCallback<CombinedStepT> {
    fn on_pre_ticks(&mut self) {}

    fn on_tick(&mut self, step: &CombinedStepT);

    fn on_post_ticks(&mut self) {}
}

#[derive(Debug)]
pub enum SeerError {
    CanNotPushAtMaximumCapacity,
    StepsError(StepsError),
}

impl From<StepsError> for SeerError {
    fn from(error: StepsError) -> Self {
        SeerError::StepsError(error)
    }
}

impl ErrorLevelProvider for SeerError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            SeerError::CanNotPushAtMaximumCapacity => ErrorLevel::Warning,
            SeerError::StepsError(_) => ErrorLevel::Critical,
        }
    }
}

// Define the Assent struct
impl<Callback, CombinedStepT: Clone + Debug + Display> Default for Seer<Callback, CombinedStepT>
where
    Callback: SeerCallback<CombinedStepT>,
{
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Settings {
    pub max_predicted_steps_capacity: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_predicted_steps_capacity: 14,
        }
    }
}

#[derive(Debug)]
pub struct Seer<Callback, CombinedStepT>
where
    Callback: SeerCallback<CombinedStepT>,
{
    predicted_steps: Steps<CombinedStepT>,
    authoritative_has_changed: bool,
    settings: Settings,
    phantom: PhantomData<Callback>,
}

impl<Callback, CombinedStepT> Seer<Callback, CombinedStepT>
where
    Callback: SeerCallback<CombinedStepT>,
    CombinedStepT: Clone + Debug + Display,
{
    pub fn new(settings: Settings) -> Self {
        Seer {
            predicted_steps: Steps::new(),
            phantom: PhantomData,
            authoritative_has_changed: false,
            settings,
        }
    }

    pub fn predicted_steps(&self) -> &Steps<CombinedStepT> {
        &self.predicted_steps
    }

    pub fn update(&mut self, callback: &mut Callback) {
        if self.predicted_steps.is_empty() {
            return;
        }

        trace!("pre_ticks");
        callback.on_pre_ticks();

        trace!("{} predicted steps in queue.", self.predicted_steps.len());

        for combined_step_info in self.predicted_steps.iter() {
            trace!("tick {}", combined_step_info);

            callback.on_tick(&combined_step_info.step);
        }

        trace!("post_ticks");
        callback.on_post_ticks();
        self.authoritative_has_changed = false;
    }

    pub fn received_authoritative(&mut self, tick: TickId) {
        trace!("received_authoritative discarding predicted steps before {tick}");
        self.predicted_steps.discard_up_to(tick + 1);
        trace!("predicted steps remaining {}", self.predicted_steps.len());
    }

    pub fn authoritative_has_changed(&mut self) {
        self.authoritative_has_changed = true;
    }

    pub fn push(
        &mut self,
        tick_id: TickId,
        predicted_step: CombinedStepT,
    ) -> Result<(), SeerError> {
        if self.predicted_steps.len() >= self.settings.max_predicted_steps_capacity {
            Err(SeerError::CanNotPushAtMaximumCapacity)?;
        }
        self.predicted_steps
            .push(tick_id, predicted_step)?;
        Ok(())
    }
}
