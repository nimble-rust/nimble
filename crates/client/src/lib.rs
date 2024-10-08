/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use app_version::VersionProvider;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use monotonic_time_rs::InstantMonotonicClock;
use nimble_client_front::{ClientFront, ClientFrontError, LocalPlayer};
use nimble_rectify::{Rectify, RectifyCallbacks, RectifyError};
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
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

impl<
        StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
        GameT: GameCallbacks<StepT> + std::fmt::Debug,
    > Default for Client<GameT, StepT>
{
    fn default() -> Self {
        Self::new()
    }
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
    tick_duration_ms: u64,
    #[allow(unused)]
    rectify: Rectify<GameT, StepForParticipants<Step<StepT>>>,
}

impl<
        StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
        GameT: GameCallbacks<StepT> + std::fmt::Debug,
    > Client<GameT, StepT>
{
    pub fn new() -> Self {
        let clock = Rc::new(RefCell::new(InstantMonotonicClock::new()));

        let deterministic_app_version = GameT::version();
        Self {
            client: ClientFront::<GameT, StepT>::new(deterministic_app_version, clock),
            tick_duration_ms: 16,
            rectify: Rectify::default(),
        }
    }

    pub fn with_tick_duration(mut self, tick_duration: u64) -> Self {
        self.tick_duration_ms = tick_duration;
        self
    }

    pub fn send(&mut self) -> Result<Vec<Vec<u8>>, ClientError> {
        Ok(self.client.send()?)
    }

    pub fn receive(&mut self, datagram: &[u8]) -> Result<(), ClientError> {
        self.client.receive(datagram)?;
        //let auth_steps = self.client.pop_all_authoritative_steps()?;
        //trace!("found auth_steps: {}", auth_steps);
        Ok(())
    }

    pub fn rectify(&self) -> &Rectify<GameT, StepForParticipants<Step<StepT>>> {
        &self.rectify
    }

    pub fn update(&mut self) -> Result<(), ClientError> {
        self.client.update();

        let (tick_id, auth_steps) = self.client.pop_all_authoritative_steps()?;

        self.rectify
            .push_authoritatives_with_check(tick_id, auth_steps.as_slice())?;
        if let Some(game_state) = self.client.game_state_mut() {
            self.rectify.update(game_state);
        }

        Ok(())
    }

    pub fn game_state(&self) -> Option<&GameT> {
        self.client.game_state()
    }

    pub fn want_predicted_step(&self) -> bool {
        self.client.can_push_predicted_step()
    }

    pub fn can_join_player(&self) -> bool {
        self.client.client.game_state().is_some()
    }

    pub fn local_players(&self) -> Vec<LocalPlayer> {
        self.client.client.local_players()
    }

    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: StepForParticipants<StepT>,
    ) -> Result<(), ClientError> {
        /*
            // create authoritative step from predicted step
            let auth = AuthoritativeStep::<Step<StepT>> {
                authoritative_participants: SeqMap::<ParticipantId, >,
            }
            self.rectify.push_predicted(step);
        */

        self.client.push_predicted_step(tick_id, step)?;

        Ok(())
    }

    pub fn latency(&self) -> Option<u16> {
        if let Some((_, x, _)) = self.client.latency() {
            Some(x as u16)
        } else {
            None
        }
    }

    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.client.server_buffer_delta_ticks()
    }

    #[allow(unused)]
    fn optimal_prediction_tick_count(&self) -> usize {
        if let Some(latency_ms) = self.latency() {
            let latency_in_ticks = (latency_ms / self.tick_duration_ms as u16) + 1;
            let tick_delta = self.server_buffer_delta_ticks().unwrap_or(0);
            const MINIMUM_DELTA_TICK: u32 = 2;
            let buffer_add = if (tick_delta as u32) < MINIMUM_DELTA_TICK {
                ((MINIMUM_DELTA_TICK as i32) - tick_delta as i32) as u32
            } else {
                0
            };

            (latency_in_ticks as u32 + buffer_add) as usize
        } else {
            2
        }
    }

    pub fn request_join_player(
        &mut self,
        local_players: Vec<LocalIndex>,
    ) -> Result<(), ClientError> {
        self.client.request_join_player(local_players)?;
        Ok(())
    }
}
