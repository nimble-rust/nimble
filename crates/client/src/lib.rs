/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::VersionProvider;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use log::debug;
use metricator::MinMaxAvg;
use monotonic_time_rs::Millis;
use nimble_client_front::{ClientFront, ClientFrontError, CombinedMetrics, LocalPlayer};
use nimble_rectify::{Rectify, RectifyCallbacks, RectifyError};
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use std::fmt::Debug;
use tick_id::TickId;

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

pub struct Client<
    GameT: GameCallbacks<StepT> + std::fmt::Debug,
    StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
> {
    client: ClientFront<GameT, StepT>,

    #[allow(unused)]
    rectify: Rectify<GameT, StepForParticipants<Step<StepT>>>,
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

            rectify: Rectify::default(),
        }
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
        self.client.update(now);

        let (first_tick_id_in_vector, auth_steps) = self.client.pop_all_authoritative_steps();
        let mut current_tick_id = first_tick_id_in_vector;
        for auth_step in auth_steps {
            if current_tick_id == self.rectify.waiting_for_authoritative_tick_id() {
                self.rectify
                    .push_authoritative_with_check(current_tick_id, auth_step)?;
            }
            current_tick_id = TickId(current_tick_id.0 + 1);
        }

        if let Some(game) = self.client.game_mut() {
            self.rectify.update(game);
        }

        Ok(())
    }

    pub fn game(&self) -> Option<&GameT> {
        self.client.game()
    }

    pub fn need_prediction_count(&self) -> usize {
        let v = self.client.need_prediction_count();
        debug!("optimal count: {v}");
        v
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
        self.client.push_predicted_step(tick_id, step)?;

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
