/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod prelude;

use err_rs::{ErrorLevel, ErrorLevelProvider};
use log::trace;
use std::fmt::Debug;
use std::marker::PhantomData;

use nimble_steps::Steps;
use tick_id::TickId;

pub trait SeerCallback<CombinedStepT> {
    fn on_pre_ticks(&mut self) {}

    fn on_tick(&mut self, step: &CombinedStepT);

    fn on_post_ticks(&mut self) {}
}

#[derive(Debug)]
pub enum SeerError {
    CanNotPushAtMaximumCapacity,
}

impl ErrorLevelProvider for SeerError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            SeerError::CanNotPushAtMaximumCapacity => ErrorLevel::Warning,
        }
    }
}

// Define the Assent struct
impl<Callback, CombinedStepT: Clone + Debug> Default for Seer<Callback, CombinedStepT>
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
            max_predicted_steps_capacity: 8,
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
    CombinedStepT: Clone + Debug,
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
        trace!("seer: combined steps pre_ticks");
        callback.on_pre_ticks();

        trace!("seer: combined steps len:{}", self.predicted_steps.len());
        for combined_step_info in self.predicted_steps.iter() {
            trace!("seer tick {:?}", combined_step_info);

            callback.on_tick(&combined_step_info.step);
        }

        trace!("seer: combined steps post_ticks");
        callback.on_post_ticks();
        self.authoritative_has_changed = false;
    }

    pub fn received_authoritative(&mut self, tick: TickId) {
        self.predicted_steps.pop_up_to(tick + 1);
    }

    pub fn authoritative_has_changed(&mut self) {
        self.authoritative_has_changed = true;
    }

    pub fn push(&mut self, predicted_step: CombinedStepT) -> Result<(), SeerError> {
        if self.predicted_steps.len() >= self.settings.max_predicted_steps_capacity {
            Err(SeerError::CanNotPushAtMaximumCapacity)?;
        }
        self.predicted_steps.push(predicted_step);
        Ok(())
    }
}
