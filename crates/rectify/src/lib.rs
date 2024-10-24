/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod prelude;

use err_rs::{ErrorLevel, ErrorLevelProvider};
use log::trace;
use nimble_assent::{Assent, AssentCallback, UpdateState};
use nimble_seer::{Seer, SeerCallback, SeerError};
use std::fmt::{Debug, Display};
use tick_id::TickId;
use tick_queue::QueueError;

#[derive(Debug)]
pub enum RectifyError {
    WrongTickId {
        expected: TickId,
        encountered: TickId,
    },
    SeerError(SeerError),
    QueueError(QueueError),
}

impl From<SeerError> for RectifyError {
    fn from(value: SeerError) -> Self {
        Self::SeerError(value)
    }
}

impl From<QueueError> for RectifyError {
    fn from(value: QueueError) -> Self {
        Self::QueueError(value)
    }
}

impl ErrorLevelProvider for RectifyError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::WrongTickId { .. } | Self::QueueError(_) => ErrorLevel::Critical,
            Self::SeerError(err) => err.error_level(),
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
    settings: Settings,
}

impl<Game: RectifyCallbacks<StepT>, StepT: Clone + Debug + Display> Default
    for Rectify<Game, StepT>
{
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Settings {
    pub assent: nimble_assent::Settings,
    pub seer: nimble_seer::Settings,
}

impl<Game: RectifyCallbacks<StepT>, StepT: Clone + Debug + Display> Rectify<Game, StepT> {
    /// Creates a new `Rectify` instance, initializing both [`Assent`] and [`Seer`] components.
    ///
    /// # Returns
    ///
    /// A new `Rectify` instance.
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        let assent = Assent::new(settings.assent);
        let seer = Seer::new(settings.seer);

        Self {
            assent,
            seer,
            settings,
        }
    }

    #[must_use]
    pub const fn seer(&self) -> &Seer<Game, StepT> {
        &self.seer
    }

    #[must_use]
    pub const fn settings(&self) -> Settings {
        self.settings
    }

    #[must_use]
    pub const fn assent(&self) -> &Assent<Game, StepT> {
        &self.assent
    }

    /// Pushes a predicted step into the [`Seer`] component.
    ///
    /// # Arguments
    ///
    /// * `step` - The predicted step to be pushed.
    ///
    /// # Errors
    ///
    /// `RectifyError` on error // TODO:
    pub fn push_predicted(&mut self, tick_id: TickId, step: StepT) -> Result<(), RectifyError> {
        if let Some(end_tick_id) = self.assent.end_tick_id() {
            self.seer.received_authoritative(end_tick_id);
        }
        trace!("added predicted step {}", &step);
        self.seer.push(tick_id, step)?;
        Ok(())
    }

    #[must_use]
    pub const fn waiting_for_authoritative_tick_id(&self) -> TickId {
        self.assent.next_expected_tick_id()
    }

    ///
    /// # Errors
    ///
    /// `RectifyError` on error // TODO:
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
    ///
    /// # Errors
    ///
    /// `RectifyError` on error // TODO:

    #[allow(clippy::missing_panics_doc)]
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
        self.assent.push(step_for_tick_id, step)?;
        self.seer.received_authoritative(
            self.assent
                .end_tick_id()
                .expect("we know that there is an end tick, since we pushed to it previously"),
        );

        Ok(())
    }

    /// Updates the authoritative state. If all the authoritative state has been calculated
    /// it predicts from the last authoritative state.
    /// # Arguments
    ///
    /// * `game` - A mutable reference to the game implementing the necessary callback traits.
    pub fn update(&mut self, game: &mut Game) {
        let consumed_all_knowledge = self.assent.update(game);
        if consumed_all_knowledge != UpdateState::DidNotConsumeAllKnowledge {
            game.on_copy_from_authoritative();
            self.seer.update(game);
        }
    }
}
