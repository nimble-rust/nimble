/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use flood_rs::{BufferDeserializer, Deserialize, Serialize};
use monotonic_time_rs::InstantMonotonicClock;
use nimble_assent::{Assent, DeterministicVersionProvider};
use nimble_client_front::{ClientFront, ClientFrontError};
use nimble_protocol::Version;
use nimble_rectify::RectifyCallbacks;
use nimble_step_types::AuthoritativeStep;
use nimble_steps::Step;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

pub trait GameCallbacks<StepT>:
    RectifyCallbacks<AuthoritativeStep<Step<StepT>>> + DeterministicVersionProvider + BufferDeserializer
{
}

impl<T, StepT> GameCallbacks<StepT> for T where
    T: RectifyCallbacks<AuthoritativeStep<Step<StepT>>>
        + DeterministicVersionProvider
        + BufferDeserializer
{
}

impl<StepT: Clone + Deserialize + Serialize + Debug, GameT: GameCallbacks<StepT>> Default
    for Client<GameT, StepT>
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct Client<GameT: GameCallbacks<StepT>, StepT: Clone + Deserialize + Serialize + Debug> {
    client: ClientFront<GameT, StepT>,
    tick_duration_ms: u64,
    #[allow(unused)]
    assent: Assent<GameT, AuthoritativeStep<Step<StepT>>>,
}

impl<StepT: Clone + Deserialize + Serialize + Debug, GameT: GameCallbacks<StepT>>
    Client<GameT, StepT>
{
    pub fn new() -> Self {
        let clock = Rc::new(RefCell::new(InstantMonotonicClock::new()));

        let deterministic_app_version = GameT::deterministic_version();
        let application_version = Version {
            major: deterministic_app_version.major,
            minor: deterministic_app_version.minor,
            patch: deterministic_app_version.patch,
        };
        Self {
            client: ClientFront::<GameT, StepT>::new(&application_version, clock),
            tick_duration_ms: 16,
            assent: Assent::default(),
        }
    }

    pub fn with_tick_duration(mut self, tick_duration: u64) -> Self {
        self.tick_duration_ms = tick_duration;
        self
    }

    pub fn send(&mut self) -> Result<Vec<Vec<u8>>, ClientFrontError> {
        self.client.send()
    }

    pub fn receive(&mut self, datagram: &[u8]) -> Result<(), ClientFrontError> {
        self.client.receive(datagram)
    }

    pub fn update(&mut self) {
        self.client.update()
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
}
