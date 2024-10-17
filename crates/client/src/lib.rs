/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*!

# Nimble Client 🕹
**Nimble Client** is a Rust crate designed to manage networking tasks for multiplayer games.
It handles downloading the complete game state from a host, managing participants by sending
requests to the host, sending predicted inputs (steps) to the host for smoother gameplay, and
receiving authoritative steps to ensure consistent game state.

## Features

- **Game State Downloading:** Fetch the entire game state from the host. 🗂️
- **Participant Management:** Add and remove players by sending requests to the host. ➕➖
- **Input Prediction:** Send predicted inputs (steps) to the host for reduced latency. 🔮
- **Authoritative Step Handling:** Receive and apply authoritative steps from the host to
    maintain game state consistency. 📥📤
- **Metrics and Logging:** Built-in support for network metrics and logging to monitor and
    debug client operations. 📊🛠️

*/

pub mod err;
pub mod prelude;

use crate::err::ClientError;
use app_version::VersionProvider;
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use log::trace;
use metricator::MinMaxAvg;
use monotonic_time_rs::{Millis, MillisDuration};
use network_metrics::{CombinedMetrics, NetworkMetrics};
use nimble_client_logic::LocalIndex;
use nimble_client_logic::{ClientLogic, ClientLogicPhase, LocalPlayer};
use nimble_layer::NimbleLayer;
use nimble_protocol::prelude::HostToClientCommands;
use nimble_rectify::{Rectify, RectifyCallbacks};
use nimble_step::Step;
use nimble_step_map::StepMap;
use std::cmp::min;
use std::fmt::{Debug, Display};
use tick_id::TickId;
use time_tick::TimeTick;

pub type MillisDurationRange = RangeToFactor<MillisDuration, MillisDuration>;

pub struct RangeToFactor<V, F> {
    range_min: V,
    min_factor: F,
    range_max: V,
    max_factor: F,
    factor: F,
}

impl<V: PartialOrd, F> RangeToFactor<V, F> {
    pub const fn new(range_min: V, range_max: V, min_factor: F, factor: F, max_factor: F) -> Self {
        Self {
            range_min,
            min_factor,
            range_max,
            max_factor,
            factor,
        }
    }

    #[inline]
    pub fn get_factor(&self, input: V) -> &F {
        if input < self.range_min {
            &self.min_factor
        } else if input > self.range_max {
            &self.max_factor
        } else {
            &self.factor
        }
    }
}

pub trait GameCallbacks<StepT: Display>:
    RectifyCallbacks<StepMap<Step<StepT>>> + VersionProvider + BufferDeserializer
{
}

impl<T, StepT> GameCallbacks<StepT> for T
where
    T: RectifyCallbacks<StepMap<Step<StepT>>> + VersionProvider + BufferDeserializer,
    StepT: Display,
{
}

#[derive(Debug, PartialEq)]
pub enum ClientPhase {
    Normal,
    CanSendPredicted,
}

/// The main client structure handling datagram communication, participant management, and input (step) prediction.
///
/// The `Client` does not handle game logic directly but relies on external game logic
/// provided through the `GameCallbacks` trait.
pub struct Client<
    GameT: GameCallbacks<StepT> + Debug,
    StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
> {
    nimble_layer: NimbleLayer,
    logic: ClientLogic<GameT, StepT>,
    metrics: NetworkMetrics,
    rectify: Rectify<GameT, StepMap<Step<StepT>>>,
    authoritative_range_to_tick_duration_ms: RangeToFactor<u8, f32>,
    authoritative_time_tick: TimeTick,
    prediction_range_to_tick_duration_ms: RangeToFactor<i32, f32>,
    pub prediction_time_tick: TimeTick,
    max_prediction_count: usize,
    last_need_prediction_count: u16,
    phase: ClientPhase,
    tick_duration_ms: MillisDuration,
}

impl<
        StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display + Eq,
        GameT: GameCallbacks<StepT> + Debug,
    > Client<GameT, StepT>
{
    /// Creates a new `Client` instance with the given current time.
    ///
    /// # Arguments
    ///
    /// * `now` - The current time in milliseconds.
    pub fn new(now: Millis) -> Self {
        let deterministic_app_version = GameT::version();
        Self {
            nimble_layer: NimbleLayer::new(),
            logic: ClientLogic::<GameT, StepT>::new(deterministic_app_version),
            metrics: NetworkMetrics::new(now),

            authoritative_range_to_tick_duration_ms: RangeToFactor::new(2, 5, 0.9, 1.0, 2.0), // 0.9 is faster, since it is a multiplier to tick_duration
            authoritative_time_tick: TimeTick::new(now, MillisDuration::from_millis(16), 4),

            prediction_range_to_tick_duration_ms: RangeToFactor::new(-1, 3, 0.85, 1.0, 2.0), // 0.9 is faster, since it is a multiplier to tick_duration
            prediction_time_tick: TimeTick::new(now, MillisDuration::from_millis(16), 4),

            rectify: Rectify::default(),
            last_need_prediction_count: 0,
            phase: ClientPhase::Normal,
            max_prediction_count: 10, // TODO: Settings
            tick_duration_ms: MillisDuration::from_millis(16),
        }
    }

    pub fn with_tick_duration(mut self, tick_duration: MillisDuration) -> Self {
        self.tick_duration_ms = tick_duration;
        self
    }

    const MAX_DATAGRAM_SIZE: usize = 1024;

    /// Creates outgoing messages and returns the serialized datagrams.
    ///
    /// This method collects messages prepared by the client logic, serializes them into datagrams,
    /// updates network metrics, and returns the datagrams. They are usually sent over some datagram transport.
    ///
    /// # Arguments
    ///
    /// * `now` - The current time in milliseconds.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of serialized datagrams or a `ClientError`.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if serialization or sending fails.
    pub fn send(&mut self, now: Millis) -> Result<Vec<Vec<u8>>, ClientError> {
        let messages = self.logic.send(now);
        let datagrams =
            datagram_chunker::serialize_to_datagrams(messages, Self::MAX_DATAGRAM_SIZE)?;
        self.metrics.sent_datagrams(&datagrams);

        let datagrams_with_header = self.nimble_layer.send(datagrams)?;

        Ok(datagrams_with_header)
    }

    /// Receives and processes an incoming datagram.
    ///
    /// This method handles incoming datagrams by updating metrics, deserializing the datagram,
    /// and passing the contained commands to the client logic for further processing.
    ///
    /// # Arguments
    ///
    /// * `millis` - The current time in milliseconds.
    /// * `datagram` - The received datagram bytes.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or containing a `ClientError`.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if deserialization or processing fails.
    pub fn receive(&mut self, now: Millis, datagram: &[u8]) -> Result<(), ClientError> {
        self.metrics.received_datagram(datagram);
        let datagram_without_header = self.nimble_layer.receive(datagram)?;
        let commands = datagram_chunker::deserialize_datagram::<HostToClientCommands<Step<StepT>>>(
            datagram_without_header,
        )?;
        for command in commands {
            self.logic.receive(now, &command)?;
        }

        Ok(())
    }

    pub fn debug_rectify(&self) -> &Rectify<GameT, StepMap<Step<StepT>>> {
        &self.rectify
    }

    /// Updates the client's phase and handles synchronization tasks based on the current time.
    ///
    /// This includes updating the network layer, metrics, tick durations, processing authoritative steps,
    /// and managing prediction phases.
    ///
    /// # Arguments
    ///
    /// * `now` - The current time in milliseconds.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or containing a `ClientError`.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if any internal operations fail.
    pub fn update(&mut self, now: Millis) -> Result<(), ClientError> {
        trace!("client: update {now}");
        self.metrics.update_metrics(now);

        let factor = self
            .authoritative_range_to_tick_duration_ms
            .get_factor(self.logic.debug_authoritative_steps().len() as u8);
        self.authoritative_time_tick
            .set_tick_duration(*factor * self.tick_duration_ms);
        self.authoritative_time_tick.calculate_ticks(now);

        let (first_tick_id_in_vector, auth_steps) = self.logic.pop_all_authoritative_steps();
        let mut current_tick_id = first_tick_id_in_vector;
        for auth_step in auth_steps {
            if current_tick_id == self.rectify.waiting_for_authoritative_tick_id() {
                self.rectify
                    .push_authoritative_with_check(current_tick_id, auth_step)?;
            }
            current_tick_id = TickId(current_tick_id.0 + 1);
        }

        match self.logic.phase() {
            ClientLogicPhase::RequestConnect => {}
            ClientLogicPhase::RequestDownloadState { .. } => {}
            ClientLogicPhase::DownloadingState(_) => {}
            ClientLogicPhase::SendPredictedSteps => {
                if self.phase != ClientPhase::CanSendPredicted {
                    self.prediction_time_tick.reset(now);
                    self.phase = ClientPhase::CanSendPredicted;
                }
            }
        }

        match self.phase {
            ClientPhase::Normal => {}
            ClientPhase::CanSendPredicted => {
                self.adjust_prediction_ticker();
                self.last_need_prediction_count = self.prediction_time_tick.calculate_ticks(now);
                if self.logic.predicted_step_count_in_queue() >= self.max_prediction_count {
                    trace!(
                        "prediction queue is maxed out: {}",
                        self.max_prediction_count
                    );
                    self.last_need_prediction_count = 0;
                    self.prediction_time_tick.reset(now);
                }

                trace!("prediction count: {}", self.last_need_prediction_count);
                if let Some(game) = self.logic.game_mut() {
                    self.rectify.update(game);
                }
            }
        }

        Ok(())
    }

    /// Calculates the difference between the current prediction count and the optimal count.
    ///
    /// # Returns
    ///
    /// The difference as an `i32`. A positive value indicates excess predictions, while a negative
    /// value indicates a deficit.
    fn delta_prediction_count(&self) -> i32 {
        if self.logic.can_push_predicted_step() {
            let optimal_prediction_tick_count = self.optimal_prediction_tick_count();
            let prediction_count_in_queue = self.logic.predicted_step_count_in_queue();
            trace!("optimal according to latency {optimal_prediction_tick_count}, outgoing queue {prediction_count_in_queue}");

            prediction_count_in_queue as i32 - optimal_prediction_tick_count as i32
        } else {
            0
        }
    }

    /// Adjusts the prediction ticker based on the current delta prediction count.
    fn adjust_prediction_ticker(&mut self) {
        let delta_prediction = self.delta_prediction_count();
        let factor = self
            .prediction_range_to_tick_duration_ms
            .get_factor(delta_prediction);
        trace!(
            "delta-prediction: {delta_prediction} resulted in factor: {factor} for latency {}",
            self.latency().unwrap_or(MinMaxAvg::new(0, 0.0, 0))
        );

        self.prediction_time_tick
            .set_tick_duration(*factor * self.tick_duration_ms)
    }

    /// Determines the optimal number of prediction ticks based on current average latency.
    ///
    /// # Returns
    ///
    /// The optimal prediction tick count as a `usize`.
    ///
    /// # Notes
    ///
    /// This function ensures that the prediction count does not exceed a predefined maximum.
    fn optimal_prediction_tick_count(&self) -> usize {
        if let Some(latency_ms) = self.latency() {
            let latency_in_ticks =
                (latency_ms.avg as u16 / self.tick_duration_ms.as_millis() as u16) + 1;
            let tick_delta = self.server_buffer_delta_ticks().unwrap_or(0);
            const MINIMUM_DELTA_TICK: u32 = 2;
            let buffer_add = if (tick_delta as u32) < MINIMUM_DELTA_TICK {
                ((MINIMUM_DELTA_TICK as i32) - tick_delta as i32) as u32
            } else {
                0
            };

            let count = (latency_in_ticks as u32 + buffer_add) as usize;

            const MAXIMUM_PREDICTION_COUNT: usize = 10; // TODO: Setting
            min(count, MAXIMUM_PREDICTION_COUNT)
        } else {
            2
        }
    }

    /// Retrieves a reference to the current game instance, if available.
    ///
    /// Note: The `Client` does not manage game logic directly. This method provides access to the
    /// game state managed externally via callbacks.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to `CallbacksT` or `None` if no game is active.
    pub fn game(&self) -> Option<&GameT> {
        self.logic.game()
    }

    /// Determines the number of predictions needed based on the current state.
    ///
    /// # Returns
    ///
    /// The number of predictions needed as a `usize`.
    pub fn required_prediction_count(&self) -> usize {
        if !self.logic.can_push_predicted_step() {
            0
        } else {
            self.last_need_prediction_count as usize
        }
    }

    /// Checks if a new player can join the game session.
    ///
    /// # Returns
    ///
    /// `true` if a player can join, `false` otherwise.
    pub fn can_join_player(&self) -> bool {
        self.game().is_some()
    }

    /// Retrieves a list of local players currently managed by the client.
    ///
    /// # Returns
    ///
    /// A vector of `LocalPlayer` instances.
    pub fn local_players(&self) -> Vec<LocalPlayer> {
        self.logic.local_players()
    }

    /// Adds a predicted input (step) to the client's logic and rectification system.
    ///
    /// This method serializes predicted steps into datagrams (in the future) in upcoming send() function calls.
    ///
    /// # Arguments
    ///
    /// * `tick_id` - The tick identifier for the predicted step.
    /// * `step` - The predicted step data.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or containing a `ClientError`.
    ///
    /// # Errors
    ///
    /// Returns `ClientError` if the prediction queue is full or if processing fails.
    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: StepMap<StepT>,
    ) -> Result<(), ClientError> {
        let count = step.len();
        if count > self.required_prediction_count() {
            return Err(ClientError::PredictionQueueOverflow);
        }
        self.prediction_time_tick.performed_ticks(count as u16);

        self.logic.push_predicted_step(tick_id, step.clone())?;

        let mut seq_map = StepMap::<Step<StepT>>::new();

        for (participant_id, step) in &step {
            seq_map
                .insert(*participant_id, Step::Custom(step.clone()))
                .expect("can't insert step");
        }
        self.rectify.push_predicted(tick_id, seq_map)?;

        Ok(())
    }

    /// Retrieves the current transmission round trip latency metrics.
    ///
    /// # Returns
    ///
    /// An `Option` containing `MinMaxAvg<u16>` representing latency metrics, or `None` if unavailable.
    pub fn latency(&self) -> Option<MinMaxAvg<u16>> {
        self.logic.latency()
    }

    /// Retrieves the combined network metrics.
    ///
    /// # Returns
    ///
    /// A `CombinedMetrics` instance containing various network metrics.
    pub fn metrics(&self) -> CombinedMetrics {
        self.metrics.metrics()
    }

    /// Retrieves the delta ticks on the host for the incoming predicted steps
    /// A negative means that the incoming buffer is too low, a larger positive number
    /// means that the buffer is too big, and the prediction should slow down.
    ///
    /// # Returns
    ///
    /// An `Option` containing the delta ticks as `i16`, or `None` if unavailable.
    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.logic.server_buffer_delta_ticks()
    }

    /// Requests to join a new player with the specified local indices.
    ///
    /// This method sends a request to the host to add new participants to the game session.
    ///
    /// # Arguments
    ///
    /// * `local_players` - A vector of `LocalIndex` representing the local players to join.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or containing a `ClientError`.
    pub fn request_join_player(
        &mut self,
        local_players: Vec<LocalIndex>,
    ) -> Result<(), ClientError> {
        self.logic.set_joining_player(local_players);
        Ok(())
    }
}
