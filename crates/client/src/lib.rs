/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::VersionProvider;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use log::trace;
use metricator::MinMaxAvg;
use monotonic_time_rs::{Millis, MillisDuration};
use nimble_client_front::{
    ClientFront, ClientFrontError, ClientLogicPhase, CombinedMetrics, LocalPlayer,
};
use nimble_participant::ParticipantId;
use nimble_rectify::{Rectify, RectifyCallbacks, RectifyError};
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use seq_map::SeqMap;
use std::cmp::min;
use std::fmt::Debug;
use tick_id::TickId;
use time_tick::{RangeToFactor, TimeTick};

pub trait GameCallbacks<StepT: std::fmt::Display>:
    RectifyCallbacks<StepForParticipants<Step<StepT>>> + VersionProvider + BufferDeserializer
{
}

impl<T, StepT> GameCallbacks<StepT> for T
where
    T: RectifyCallbacks<StepForParticipants<Step<StepT>>> + VersionProvider + BufferDeserializer,
    StepT: std::fmt::Display,
{
}

#[derive(Debug)]
pub enum ClientError {
    ClientFrontError(ClientFrontError),
    IoError(std::io::Error),
    RectifyError(RectifyError),
}

impl ErrorLevelProvider for ClientError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            ClientError::ClientFrontError(err) => err.error_level(),
            ClientError::IoError(_) => ErrorLevel::Info,
            ClientError::RectifyError(err) => err.error_level(),
        }
    }
}

impl From<RectifyError> for ClientError {
    fn from(err: RectifyError) -> Self {
        ClientError::RectifyError(err)
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ClientFrontError> for ClientError {
    fn from(err: ClientFrontError) -> Self {
        Self::ClientFrontError(err)
    }
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
    client: ClientFront<GameT, StepT>,

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
        StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
        GameT: GameCallbacks<StepT> + Debug,
    > Client<GameT, StepT>
{
    pub fn new(now: Millis) -> Self {
        let deterministic_app_version = GameT::version();
        Self {
            client: ClientFront::<GameT, StepT>::new(deterministic_app_version, now),
            authoritative_range_to_tick_duration_ms: RangeToFactor::new(2, 5, 0.9, 1.0, 2.0), // 0.9 is faster, since it is a multiplier to tick_duration
            authoritative_time_tick: TimeTick::new(now, MillisDuration::from_millis(16), 4),
            prediction_range_to_tick_duration_ms: RangeToFactor::new(-2, 2, 0.9, 1.0, 2.0), // 0.9 is faster, since it is a multiplier to tick_duration
            prediction_time_tick: TimeTick::new(now, MillisDuration::from_millis(16), 4),

            rectify: Rectify::default(),
            last_need_prediction_count: 0,
            phase: ClientPhase::Normal,
            max_prediction_count: 6,
            tick_duration_ms: MillisDuration::from_millis(16),
        }
    }

    pub fn with_tick_duration(mut self, tick_duration: MillisDuration) -> Self {
        self.tick_duration_ms = tick_duration;
        self
    }

    pub fn metrics(&self) -> CombinedMetrics {
        self.client.metrics()
    }

    pub fn send(&mut self, now: Millis) -> Result<Vec<Vec<u8>>, ClientError> {
        Ok(self.client.send(now)?)
    }

    pub fn receive(&mut self, millis: Millis, datagram: &[u8]) -> Result<(), ClientError> {
        self.client.receive(millis, datagram)?;
        //let auth_steps = self.client.pop_all_authoritative_steps()?;
        //trace!("found auth_steps: {}", auth_steps);
        Ok(())
    }

    pub fn rectify(&self) -> &Rectify<GameT, StepForParticipants<Step<StepT>>> {
        &self.rectify
    }

    pub fn update(&mut self, now: Millis) -> Result<(), ClientError> {
        self.adjust_prediction_ticker();
        self.client.update(now);

        let auth_buffer_count = self
            .client
            .client
            .logic()
            .server_buffer_count()
            .unwrap_or(0);
        let factor = self
            .authoritative_range_to_tick_duration_ms
            .calculate(auth_buffer_count);
        self.authoritative_time_tick
            .set_time_period(*factor * self.tick_duration_ms);
        self.authoritative_time_tick.update(now);

        let (first_tick_id_in_vector, auth_steps) = self.client.pop_all_authoritative_steps();
        let mut current_tick_id = first_tick_id_in_vector;
        for auth_step in auth_steps {
            if current_tick_id == self.rectify.waiting_for_authoritative_tick_id() {
                self.rectify
                    .push_authoritative_with_check(current_tick_id, auth_step)?;
            }
            current_tick_id = TickId(current_tick_id.0 + 1);
        }

        match self.client.client.logic().phase() {
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

        self.last_need_prediction_count = self.prediction_time_tick.update(now);
        if self.client.client.logic().predicted_step_count_in_queue() >= self.max_prediction_count {
            self.last_need_prediction_count = 0;
            self.prediction_time_tick.reset(now);
        }

        trace!("prediction count: {}", self.last_need_prediction_count);

        if let Some(game) = self.client.game_mut() {
            self.rectify.update(game);
        }

        Ok(())
    }

    fn delta_prediction_count(&self) -> i32 {
        if self.client.client.logic().can_push_predicted_step() {
            let optimal_prediction_tick_count = self.optimal_prediction_tick_count();
            let prediction_count_in_queue =
                self.client.client.logic().predicted_step_count_in_queue();
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
            .calculate(delta_prediction);
        trace!(
            "delta-prediction: {delta_prediction} resulted in factor: {factor} for latency {}",
            self.latency().unwrap_or(MinMaxAvg::new(0, 0.0, 0))
        );

        self.prediction_time_tick
            .set_time_period(*factor * self.tick_duration_ms)
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
        self.client.game()
    }

    pub fn need_prediction_count(&self) -> usize {
        if !self.client.client.logic().can_push_predicted_step() {
            0
        } else {
            self.last_need_prediction_count as usize
        }
    }

    pub fn can_join_player(&self) -> bool {
        self.client.client.game().is_some()
    }

    pub fn local_players(&self) -> Vec<LocalPlayer> {
        self.client.client.local_players()
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
        self.prediction_time_tick.performed_tick_count(count as u16);

        self.client.push_predicted_step(tick_id, step.clone())?;

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
        self.client.latency()
    }

    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.client.server_buffer_delta_ticks()
    }

    pub fn request_join_player(
        &mut self,
        local_players: Vec<LocalIndex>,
    ) -> Result<(), ClientError> {
        self.client.request_join_player(local_players)?;
        Ok(())
    }
}
