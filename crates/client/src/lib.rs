/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
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
use nimble_client_logic::{ClientLogic, ClientLogicPhase, LocalPlayer};
use nimble_layer::NimbleLayerClient;
use nimble_participant::ParticipantId;
use nimble_protocol::prelude::HostToClientCommands;
use nimble_rectify::{Rectify, RectifyCallbacks};
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use seq_map::SeqMap;
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
    RectifyCallbacks<StepForParticipants<Step<StepT>>> + VersionProvider + BufferDeserializer
{
}

impl<T, StepT> GameCallbacks<StepT> for T
where
    T: RectifyCallbacks<StepForParticipants<Step<StepT>>> + VersionProvider + BufferDeserializer,
    StepT: Display,
{
}

#[derive(Debug, PartialEq)]
pub enum ClientPhase {
    Normal,
    CanSendPredicted,
}

pub struct Client<
    GameT: GameCallbacks<StepT> + Debug,
    StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
> {
    nimble_layer: NimbleLayerClient,
    logic: ClientLogic<GameT, StepT>,
    metrics: NetworkMetrics,

    #[allow(unused)]
    rectify: Rectify<GameT, StepForParticipants<Step<StepT>>>,
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
    pub fn new(now: Millis) -> Self {
        let deterministic_app_version = GameT::version();
        Self {
            nimble_layer: NimbleLayerClient::new(now),
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

    pub fn send(&mut self, now: Millis) -> Result<Vec<Vec<u8>>, ClientError> {
        let messages = self.logic.send();
        let datagrams =
            datagram_chunker::serialize_to_datagrams(messages, Self::MAX_DATAGRAM_SIZE)?;
        self.metrics.sent_datagrams(&datagrams);

        let datagrams_with_header = self.nimble_layer.send(now, datagrams)?;

        Ok(datagrams_with_header)
    }

    pub fn receive(&mut self, millis: Millis, datagram: &[u8]) -> Result<(), ClientError> {
        self.metrics.received_datagram(datagram);
        let datagram_without_header = self.nimble_layer.receive(millis, datagram)?;
        let commands = datagram_chunker::deserialize_datagram::<HostToClientCommands<Step<StepT>>>(
            datagram_without_header,
        )?;
        for command in commands {
            self.logic.receive(&command)?;
        }

        Ok(())
    }

    pub fn rectify(&self) -> &Rectify<GameT, StepForParticipants<Step<StepT>>> {
        &self.rectify
    }

    pub fn update(&mut self, now: Millis) -> Result<(), ClientError> {
        trace!("client: update {now}");
        self.nimble_layer.update(now);
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

    #[allow(unused)]
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

    pub fn game(&self) -> Option<&GameT> {
        self.logic.game()
    }

    pub fn need_prediction_count(&self) -> usize {
        if !self.logic.can_push_predicted_step() {
            0
        } else {
            self.last_need_prediction_count as usize
        }
    }

    pub fn can_join_player(&self) -> bool {
        self.game().is_some()
    }

    pub fn local_players(&self) -> Vec<LocalPlayer> {
        self.logic.local_players()
    }

    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: StepForParticipants<StepT>,
    ) -> Result<(), ClientError> {
        let count = step.combined_step.len();
        if count > self.need_prediction_count() {
            panic!("not great")
        }
        self.prediction_time_tick.performed_ticks(count as u16);

        self.logic.push_predicted_step(tick_id, step.clone())?;

        let mut seq_map = SeqMap::<ParticipantId, Step<StepT>>::new();

        for (participant_id, step) in step.combined_step.into_iter() {
            seq_map
                .insert(*participant_id, Step::Custom(step.clone()))
                .expect("can't insert step");
        }
        self.rectify.push_predicted(
            tick_id,
            StepForParticipants::<Step<StepT>> {
                combined_step: seq_map,
            },
        )?;

        Ok(())
    }

    pub fn latency(&self) -> Option<MinMaxAvg<u16>> {
        self.nimble_layer.latency()
    }

    pub fn metrics(&self) -> CombinedMetrics {
        self.metrics.metrics()
    }

    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.logic.server_buffer_delta_ticks()
    }

    pub fn request_join_player(
        &mut self,
        local_players: Vec<LocalIndex>,
    ) -> Result<(), ClientError> {
        self.logic.set_joining_player(local_players);
        Ok(())
    }
}
