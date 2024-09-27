/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::err::ClientErrorKind::Unexpected;
use crate::err::{ClientError, ClientErrorKind};
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::BufferDeserializer;
use flood_rs::{Deserialize, Serialize};
use log::{debug, trace};
use nimble_blob_stream::prelude::{FrontLogic, SenderToReceiverFrontCommands};
use nimble_protocol::client_to_host::{CombinedPredictedSteps, DownloadGameStateRequest};
use nimble_protocol::host_to_client::DownloadGameStateResponse;
use nimble_protocol::prelude::*;
use nimble_step_types::{AuthoritativeStep, PredictedStep};
use nimble_steps::Steps;
use std::fmt::Debug;
use tick_id::TickId;

#[derive(Debug)]
pub enum ClientLogicPhase {
    RequestDownloadState { download_state_request_id: u8 },
    DownloadingState(TickId),
    SendPredictedSteps,
}

#[derive(Debug)]
pub struct ClientLogic<StateT: BufferDeserializer, StepT: Clone + Deserialize + Serialize + Debug> {
    joining_player: Option<JoinGameRequest>,
    state: Option<StateT>,
    tick_id: u32,
    debug_tick_id_to_send: u32,
    blob_stream_client: FrontLogic,
    commands_to_send: Vec<ClientToHostCommands<StepT>>,
    outgoing_predicted_steps: Steps<PredictedStep<StepT>>,
    incoming_authoritative_steps: Steps<AuthoritativeStep<StepT>>,
    #[allow(unused)]
    phase: ClientLogicPhase,
    last_download_state_request_id: u8,
}

impl<StateT: BufferDeserializer, StepT: Clone + Deserialize + Serialize + Debug> Default
    for ClientLogic<StateT, StepT>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<StateT: BufferDeserializer, StepT: Clone + Deserialize + Serialize + Debug>
    ClientLogic<StateT, StepT>
{
    pub fn new() -> ClientLogic<StateT, StepT> {
        Self {
            joining_player: None,
            tick_id: 0,
            debug_tick_id_to_send: 0,
            blob_stream_client: FrontLogic::new(),
            commands_to_send: Vec::new(),
            last_download_state_request_id: 0x99,
            outgoing_predicted_steps: Steps::new(),
            incoming_authoritative_steps: Steps::new(),
            state: None,
            phase: ClientLogicPhase::RequestDownloadState {
                download_state_request_id: 0x99,
            },
        }
    }

    pub fn debug_authoritative_steps(&self) -> &Steps<AuthoritativeStep<StepT>> {
        &self.incoming_authoritative_steps
    }

    pub fn set_joining_player(&mut self, join_game_request: JoinGameRequest) {
        self.joining_player = Some(join_game_request);
    }

    pub fn debug_set_tick_id(&mut self, tick_id: u32) {
        self.tick_id = tick_id;
        self.debug_tick_id_to_send = self.tick_id;
    }

    #[allow(unused)]
    fn request_game_state(&mut self) {
        self.last_download_state_request_id += 1;
        self.phase = ClientLogicPhase::RequestDownloadState {
            download_state_request_id: self.last_download_state_request_id,
        };
    }

    fn download_state_request(
        &mut self,
        download_request_id: u8,
    ) -> Vec<ClientToHostCommands<StepT>> {
        let mut vec = vec![];
        let download_request = DownloadGameStateRequest {
            request_id: download_request_id,
        };
        vec.push(ClientToHostCommands::DownloadGameState(download_request));

        if let Some(cmd) = self.blob_stream_client.send() {
            vec.push(ClientToHostCommands::BlobStreamChannel(cmd))
        }

        vec
    }

    fn send_steps_request(&mut self) -> ClientToHostCommands<StepT> {
        let steps_request = StepsRequest {
            ack: StepsAck {
                waiting_for_tick_id: self.tick_id,
                lost_steps_mask_after_last_received: 0,
            },
            combined_predicted_steps: CombinedPredictedSteps {
                first_tick: self
                    .outgoing_predicted_steps
                    .front_tick_id()
                    .unwrap_or_default(),
                steps: self.outgoing_predicted_steps.to_vec(),
            },
        };

        ClientToHostCommands::Steps(steps_request)
    }

    pub fn send(&mut self) -> Vec<ClientToHostCommands<StepT>> {
        let mut commands: Vec<ClientToHostCommands<StepT>> = self.commands_to_send.clone();
        self.commands_to_send.clear();

        let normal_commands: Vec<ClientToHostCommands<StepT>> = match self.phase {
            ClientLogicPhase::RequestDownloadState {
                download_state_request_id,
            } => self.download_state_request(download_state_request_id),
            ClientLogicPhase::SendPredictedSteps => [self.send_steps_request()].to_vec(),
            ClientLogicPhase::DownloadingState(_) => {
                if let Some(x) = self.blob_stream_client.send() {
                    [ClientToHostCommands::BlobStreamChannel(x)].to_vec()
                } else {
                    vec![]
                }
            }
        };

        commands.extend(normal_commands);

        if let Some(joining_game) = &self.joining_player {
            debug!("connected. send join_game_request {:?}", joining_game);
            commands.push(ClientToHostCommands::JoinGameType(joining_game.clone()));
        }

        commands
    }

    pub fn add_predicted_step(&mut self, step: PredictedStep<StepT>) {
        self.outgoing_predicted_steps.push(step);
    }

    fn on_join_game(&mut self, cmd: &JoinGameAccepted) -> Result<(), ClientErrorKind> {
        debug!("join game accepted: {:?}", cmd);
        Ok(())
    }

    pub fn state(&self) -> Option<&StateT> {
        self.state.as_ref()
    }

    fn on_game_step(&mut self, cmd: &GameStepResponse<StepT>) -> Result<(), ClientErrorKind> {
        trace!("game step response: {:?}", cmd);
        if cmd.authoritative_steps.ranges.is_empty() {
            return Ok(());
        }

        for range in &cmd.authoritative_steps.ranges {
            let mut current_authoritative_tick_id = range.tick_id;
            for combined_auth_step in &range.authoritative_steps {
                if current_authoritative_tick_id
                    == self.incoming_authoritative_steps.expected_write_tick_id()
                {
                    self.incoming_authoritative_steps
                        .push_with_check(current_authoritative_tick_id, combined_auth_step.clone())
                        .map_err(Unexpected)?;
                }
                current_authoritative_tick_id += 1;
            }

            current_authoritative_tick_id += range.authoritative_steps.len() as u32;
        }

        Ok(())
    }

    fn on_download_state_response(
        &mut self,
        download_response: &DownloadGameStateResponse,
    ) -> Result<(), ClientErrorKind> {
        match self.phase {
            ClientLogicPhase::RequestDownloadState {
                download_state_request_id,
            } => {
                if download_response.client_request != download_state_request_id {
                    Err(ClientErrorKind::WrongDownloadRequestId)?;
                }
            }
            _ => Err(ClientErrorKind::DownloadResponseWasUnexpected)?,
        }

        self.phase = ClientLogicPhase::DownloadingState(download_response.tick_id);

        Ok(())
    }

    fn on_blob_stream(
        &mut self,
        blob_stream_command: &SenderToReceiverFrontCommands,
    ) -> Result<(), ClientErrorKind> {
        match self.phase {
            ClientLogicPhase::DownloadingState(_) => {
                self.blob_stream_client
                    .receive(blob_stream_command)
                    .map_err(ClientErrorKind::FrontLogicErr)?;
                if let Some(blob_ready) = self.blob_stream_client.blob() {
                    debug!("blob stream received, phase is set to SendPredictedSteps");
                    self.phase = ClientLogicPhase::SendPredictedSteps;
                    let (deserialized, _) =
                        StateT::deserialize(blob_ready).map_err(ClientErrorKind::IoErr)?;
                    self.state = Some(deserialized);
                }
            }
            _ => Err(ClientErrorKind::UnexpectedBlobChannelCommand)?,
        }
        Ok(())
    }

    pub fn receive_cmd(
        &mut self,
        command: &HostToClientCommands<StepT>,
    ) -> Result<(), ClientErrorKind> {
        match command {
            HostToClientCommands::JoinGame(ref join_game_response) => {
                self.on_join_game(join_game_response)?
            }
            HostToClientCommands::GameStep(ref game_step_response) => {
                self.on_game_step(game_step_response)?
            }
            HostToClientCommands::DownloadGameState(ref download_response) => {
                self.on_download_state_response(download_response)?
            }
            HostToClientCommands::BlobStreamChannel(ref blob_stream_command) => {
                self.on_blob_stream(blob_stream_command)?
            }
        }
        Ok(())
    }

    pub fn receive(&mut self, commands: &[HostToClientCommands<StepT>]) -> Result<(), ClientError> {
        let mut client_errors: Vec<ClientErrorKind> = Vec::new();

        for command in commands {
            if let Err(err) = self.receive_cmd(command) {
                if err.error_level() == ErrorLevel::Critical {
                    return Err(ClientError::Single(err));
                }
                client_errors.push(err);
            }
        }

        match client_errors.len() {
            0 => Ok(()),
            1 => Err(ClientError::Single(client_errors.pop().unwrap())),
            _ => Err(ClientError::Multiple(client_errors)),
        }
    }
}
