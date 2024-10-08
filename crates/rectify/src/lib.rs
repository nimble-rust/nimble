/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod prelude;

use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_assent::{Assent, AssentCallback, UpdateState};
use nimble_seer::{Seer, SeerCallback};
use std::fmt::Debug;
use tick_id::TickId;

#[derive(Debug)]
pub enum RectifyError {
    WrongTickId {
        expected: TickId,
        encountered: TickId,
    },
}

impl ErrorLevelProvider for RectifyError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::WrongTickId { .. } => ErrorLevel::Critical,
        }
    }
}

/// A callback trait that allows a game to handle the event when the authoritative state
pub trait RectifyCallback {
    fn on_copy_from_authoritative(&mut self);
}

pub trait RectifyCallbacks<StepT>:
    AssentCallback<StepT> + SeerCallback<StepT> + RectifyCallback
{
}

impl<T, StepT> RectifyCallbacks<StepT> for T where
    T: AssentCallback<StepT> + SeerCallback<StepT> + RectifyCallback
{
}

/// The `Rectify` struct coordinates between the [`Assent`] and [`Seer`] components, managing
/// authoritative and predicted game states.
#[derive(Debug)]
pub struct Rectify<Game: RectifyCallbacks<StepT>, StepT: Clone + Debug> {
    assent: Assent<Game, StepT>,
    seer: Seer<Game, StepT>,
}

impl<Game: RectifyCallbacks<StepT>, StepT: Clone + Debug + std::fmt::Display> Default
    for Rectify<Game, StepT>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Game: RectifyCallbacks<StepT>, StepT: Clone + std::fmt::Debug + std::fmt::Display>
    Rectify<Game, StepT>
{
    /// Creates a new `Rectify` instance, initializing both [`Assent`] and [`Seer`] components.
    ///
    /// # Returns
    ///
    /// A new `Rectify` instance.
    pub fn new() -> Self {
        let assent = Assent::new();
        let seer = Seer::new();

        Self { assent, seer }
    }

    pub fn seer(&self) -> &Seer<Game, StepT> {
        &self.seer
    }

    pub fn assent(&self) -> &Assent<Game, StepT> {
        &self.assent
    }

    /// Pushes a predicted step into the [`Seer`] component.
    ///
    /// # Arguments
    ///
    /// * `step` - The predicted step to be pushed.
    pub fn push_predicted(&mut self, step: StepT) {
        if let Some(end_tick_id) = self.assent.end_tick_id() {
            self.seer.received_authoritative(end_tick_id);
        }
        self.seer.push(step)
    }

    pub fn waiting_for_authoritative_tick_id(&self) -> Option<TickId> {
        self.assent.end_tick_id().map(|end_tick_id| end_tick_id + 1)
    }

    pub fn push_authoritatives_with_check(
        &mut self,
        step_for_tick_id: TickId,
        steps: &[StepT],
    ) -> Result<(), RectifyError> {
        let mut current_tick = step_for_tick_id;
        for step in steps {
            self.push_authoritative_with_check(current_tick, step.clone())?;
            current_tick = TickId(current_tick.0 + 1);
        }

        Ok(())
    }
    /// Pushes an authoritative step into the [`Assent`] component. This method is used to
    /// add new steps that have been determined by the authoritative host.
    ///
    /// # Arguments
    ///
    /// * `step` - The authoritative step to be pushed.
    pub fn push_authoritative_with_check(
        &mut self,
        step_for_tick_id: TickId,
        step: StepT,
    ) -> Result<(), RectifyError> {
        if let Some(end_tick_id) = self.assent.end_tick_id() {
            if end_tick_id + 1 != step_for_tick_id {
                Err(RectifyError::WrongTickId {
                    encountered: step_for_tick_id,
                    expected: end_tick_id + 1,
                })?;
            }
        }
        self.assent.push(step);
        self.seer
            .received_authoritative(self.assent.end_tick_id().unwrap());

        Ok(())
    }

    /// Updates the authoritative state. If all the authoritative state has been calculated
    /// it predicts from the last authoritative state.
    /// # Arguments
    ///
    /// * `game` - A mutable reference to the game implementing the necessary callback traits.
    pub fn update(&mut self, game: &mut Game) {
        let consumed_all_knowledge = self.assent.update(game);
        if consumed_all_knowledge == UpdateState::ConsumedAllKnowledge {
            game.on_copy_from_authoritative();
        }

        self.seer.update(game);
    }
}
