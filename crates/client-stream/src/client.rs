/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use datagram_chunker::{serialize_to_chunker, DatagramChunkerError};
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::prelude::OctetRefReader;
use flood_rs::{BufferDeserializer, Deserialize, ReadOctetStream, Serialize};
use log::trace;
use nimble_client_logic::err::ClientLogicErrorKind;
use nimble_client_logic::logic::{ClientLogic, ClientLogicPhase, LocalPlayer};
use nimble_protocol::prelude::HostToClientCommands;
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use nimble_steps::StepsError;
use std::fmt::Debug;
use std::io;
use tick_id::TickId;

pub type AuthStep<StepT> = StepForParticipants<Step<StepT>>;
pub type AuthStepVec<StepT> = Vec<AuthStep<StepT>>;

#[derive(Debug)]
pub enum ClientStreamError {
    Unexpected(String),
    IoErr(io::Error),
    ClientErr(ClientLogicErrorKind),
    PredictedStepsError(StepsError),
    DatagramChunkError(DatagramChunkerError),
    CommandNeedsConnectedPhase,
    CommandNeedsConnectingPhase,
    CanOnlyPushPredictedStepsIfConnected,
}

impl ErrorLevelProvider for ClientStreamError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::Unexpected(_) => ErrorLevel::Info,
            Self::IoErr(_) => ErrorLevel::Info,
            Self::ClientErr(err) => err.error_level(),
            Self::CommandNeedsConnectingPhase => ErrorLevel::Info,
            Self::CommandNeedsConnectedPhase => ErrorLevel::Info,
            Self::PredictedStepsError(_) => ErrorLevel::Warning,
            Self::DatagramChunkError(_) => ErrorLevel::Warning,
            Self::CanOnlyPushPredictedStepsIfConnected => ErrorLevel::Warning,
        }
    }
}

impl From<io::Error> for ClientStreamError {
    fn from(err: io::Error) -> Self {
        ClientStreamError::IoErr(err)
    }
}

impl From<DatagramChunkerError> for ClientStreamError {
    fn from(value: DatagramChunkerError) -> Self {
        Self::DatagramChunkError(value)
    }
}

impl From<StepsError> for ClientStreamError {
    fn from(value: StepsError) -> Self {
        Self::PredictedStepsError(value)
    }
}

impl From<ClientLogicErrorKind> for ClientStreamError {
    fn from(err: ClientLogicErrorKind) -> Self {
        Self::ClientErr(err)
    }
}

#[derive(Debug)]
pub struct ClientStream<
    StateT: BufferDeserializer,
    StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
> {
    logic: ClientLogic<StateT, StepT>,
}

impl<
        StateT: BufferDeserializer + Debug,
        StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
    > ClientStream<StateT, StepT>
{
    pub fn new(application_version: app_version::Version) -> Self {
        Self {
            logic: ClientLogic::new(application_version),
        }
    }

    pub fn debug_phase(&self) -> &ClientLogicPhase {
        self.logic.phase()
    }

    fn receive_stream(
        &mut self,
        in_stream: &mut impl ReadOctetStream,
    ) -> Result<(), ClientStreamError> {
        while !in_stream.has_reached_end() {
            let cmd = HostToClientCommands::<Step<StepT>>::deserialize(in_stream)?;
            trace!("client-stream: connected_receive {cmd}");
            self.logic.receive(&cmd)?;
        }
        Ok(())
    }

    pub fn pop_all_authoritative_steps(
        &mut self,
    ) -> Result<(TickId, AuthStepVec<StepT>), ClientStreamError> {
        Ok(self.logic.pop_all_authoritative_steps())
    }

    pub fn receive(&mut self, payload: &[u8]) -> Result<(), ClientStreamError> {
        let mut in_stream = OctetRefReader::new(payload);
        self.receive_stream(&mut in_stream)
    }

    const DATAGRAM_MAX_SIZE: usize = 1024;

    pub fn send(&mut self) -> Result<Vec<Vec<u8>>, ClientStreamError> {
        let commands = self.logic.send();
        Ok(serialize_to_chunker(commands, Self::DATAGRAM_MAX_SIZE)?)
    }

    pub fn game_state(&self) -> Option<&StateT> {
        self.logic.game_state()
    }

    pub fn game_state_mut(&mut self) -> Option<&mut StateT> {
        self.logic.game_state_mut()
    }

    pub fn is_in_game(&self) -> bool {
        *self.logic.phase() == ClientLogicPhase::SendPredictedSteps
    }

    pub fn can_push_predicted_step(&self) -> bool {
        self.is_in_game() && self.game_state().is_some()
    }

    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: StepForParticipants<StepT>,
    ) -> Result<(), ClientStreamError> {
        self.logic.push_predicted_step(tick_id, step)?;
        Ok(())
    }

    /// Returns the average server buffer delta tick, if available.
    ///
    /// # Returns
    /// An optional average server buffer delta tick.
    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.logic.server_buffer_delta_ticks()
    }

    pub fn request_join_player(
        &mut self,
        local_players: Vec<LocalIndex>,
    ) -> Result<(), ClientStreamError> {
        self.logic.set_joining_player(local_players);
        Ok(())
    }

    pub fn local_players(&self) -> Vec<LocalPlayer> {
        self.logic.local_players()
    }
}
