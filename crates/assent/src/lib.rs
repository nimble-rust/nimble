/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*!

`nimble-assent` is a library designed for deterministic simulation of game logic based on player input.
It operates on the concept of "steps" (or actions) taken by players and ensures these steps are applied
in a specific, predictable order. This library integrates smoothly with deterministic simulations, ensuring
that all participants in a networked game that receive and process the same steps in the same order, yield
identical results.

## Why "Assent"?

The name "Assent" was chosen because it reflects the concept of agreement or concurrence.
In a deterministic simulation, especially for multiplayer games, it is crucial that all parties
(the players and the host) are in complete agreement on the sequence of steps or actions taken.
In this context, "assent" represents the system's role in
enforcing an authoritative and agreed-upon sequence of events, ensuring that everyone shares
the same view of the game state at every step.

## Overview

The main structure in this crate is the `Assent` struct, which handles the simulation of player input
(called "steps") over a series of game ticks. The crate is designed to:

- Queue player inputs (steps) with associated tick IDs.
- Apply these inputs consistently across all participants in the simulation.
- Limit the number of ticks processed per update to avoid overloading the system.

The crate also provides a customizable callback mechanism ([`AssentCallback`]) that allows developers
to hook into different stages of the update cycle, enabling detailed control over how steps are processed.

*/

pub mod prelude;

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use log::trace;
use nimble_steps::{Steps, StepsError};
use tick_id::TickId;

/// A trait representing callbacks for the `Assent` simulation.
///
/// This trait defines hooks for handling steps at different stages of a game update cycle.
pub trait AssentCallback<CombinedStepT> {
    /// Called before any ticks are processed.
    fn on_pre_ticks(&mut self) {}

    /// Called for each tick with the corresponding step.
    fn on_tick(&mut self, step: &CombinedStepT);

    /// Called after all ticks have been processed.
    fn on_post_ticks(&mut self) {}
}

/// Enum representing the state of an update cycle in the `Assent` simulation.
#[derive(Debug, PartialEq)]
pub enum UpdateState {
    ConsumedAllKnowledge,
    DidNotConsumeAllKnowledge,
    NoKnowledge,
}

/// Configuration settings for controlling the behavior of the `Assent` simulation.
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

/// Main struct for managing and processing player steps (actions) in a deterministic simulation.
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
    CombinedStepT: Clone + Debug + Display,
{
    fn default() -> Self {
        Assent::new(Settings::default())
    }
}

impl<C, CombinedStepT> Assent<C, CombinedStepT>
where
    C: AssentCallback<CombinedStepT>,
    CombinedStepT: Clone + Debug + Display,
{
    pub fn new(settings: Settings) -> Self {
        Assent {
            phantom: PhantomData {},
            steps: Steps::new(),
            settings,
        }
    }

    /// Adds a new step to be processed for the given `tick_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the step cannot be added.
    pub fn push(&mut self, tick_id: TickId, steps: CombinedStepT) -> Result<(), StepsError> {
        self.steps.push(tick_id, steps)
    }

    /// Returns the next expected `TickId` for inserting new steps.
    pub fn next_expected_tick_id(&self) -> TickId {
        self.steps.expected_write_tick_id()
    }

    /// Returns the most recent `TickId`, or `None` if no steps have been added.
    pub fn end_tick_id(&self) -> Option<TickId> {
        self.steps.back_tick_id()
    }

    /// Returns a reference to the underlying steps for debugging purposes.
    pub fn debug_steps(&self) -> &Steps<CombinedStepT> {
        &self.steps
    }

    /// Processes available steps, invoking the provided callback for each step.
    ///
    /// This method processes up to `max_ticks_per_update` steps (ticks) and returns an
    /// `UpdateState` indicating whether all steps were processed or if some remain.
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
