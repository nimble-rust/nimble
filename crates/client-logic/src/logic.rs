/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::err::{ClientError, ClientErrorKind};
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::BufferDeserializer;
use flood_rs::{Deserialize, Serialize};
use log::{debug, trace};
use metricator::AggregateMetric;
use nimble_blob_stream::prelude::{FrontLogic, SenderToReceiverFrontCommands};
use nimble_participant::ParticipantId;
use nimble_protocol::client_to_host::{
    ConnectRequest, DownloadGameStateRequest, JoinGameType, JoinPlayerRequest, JoinPlayerRequests,
};
use nimble_protocol::host_to_client::{
    ConnectionAccepted, DownloadGameStateResponse, GameStepResponseHeader,
};
use nimble_protocol::prelude::*;
use nimble_protocol::{ClientRequestId, NIMBLE_PROTOCOL_VERSION};
use nimble_step::Step;
use nimble_step_types::{LocalIndex, StepForParticipants};
use nimble_steps::{Steps, StepsError};
use std::fmt::Debug;
use tick_id::TickId;

/// Represents the various phases of the client logic.
#[derive(Debug, PartialEq, Eq)]
pub enum ClientLogicPhase {
    /// Request Connect (agreeing on abilities, such as version)
    RequestConnect,

    /// Requesting a download of the game state.
    RequestDownloadState { download_state_request_id: u8 },

    /// Downloading the game state from the host.
    DownloadingState(TickId),

    /// Sending predicted steps from the client to the host.
    SendPredictedSteps,
}

#[derive(Debug, Clone)]
pub struct LocalPlayer {
    pub index: LocalIndex,
    pub participant_id: ParticipantId,
}

/// `ClientLogic` manages the client's state and communication logic
/// with the host in a multiplayer game session.
///
/// # Type Parameters
/// * `StateT`: A type implementing representing the game state.
/// * `StepT`: A type implementing representing the game steps.
#[derive(Debug)]
pub struct ClientLogic<
    StateT: BufferDeserializer,
    StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
> {
    // The deterministic simulation version
    deterministic_simulation_version: app_version::Version,

    connect_request_id: ClientRequestId,

    /// Represents the player's join game request, if available.
    joining_player: Option<Vec<LocalIndex>>,

    /// Holds the current game state.
    state: Option<StateT>,

    /// Manages the blob stream logic for the client.
    blob_stream_client: FrontLogic,

    /// Stores the outgoing predicted steps from the client.
    outgoing_predicted_steps: Steps<StepForParticipants<StepT>>,

    /// Stores the incoming authoritative steps from the host.
    incoming_authoritative_steps: Steps<StepForParticipants<Step<StepT>>>,

    /// Represents the current phase of the client's logic.
    phase: ClientLogicPhase,

    /// Tracks the delta of tick id on the server.
    server_buffer_delta_tick_id: AggregateMetric<i16>,

    /// Tracks the buffer step count on the server.
    server_buffer_count: AggregateMetric<u8>,
    joining_request_id: ClientRequestId,

    local_players: Vec<LocalPlayer>,
}

impl<
        StateT: BufferDeserializer,
        StepT: Clone + Deserialize + Serialize + Debug + std::fmt::Display,
    > ClientLogic<StateT, StepT>
{
    /// Creates a new `ClientLogic` instance, initializing all fields.
    pub fn new(
        deterministic_simulation_version: app_version::Version,
    ) -> ClientLogic<StateT, StepT> {
        Self {
            joining_player: None,
            joining_request_id: ClientRequestId(0),
            blob_stream_client: FrontLogic::new(),
            outgoing_predicted_steps: Steps::new(),
            incoming_authoritative_steps: Steps::new(),
            server_buffer_delta_tick_id: AggregateMetric::new(3).unwrap(),
            server_buffer_count: AggregateMetric::new(3).unwrap(),
            state: None,
            phase: ClientLogicPhase::RequestConnect,
            local_players: Vec::new(),
            deterministic_simulation_version,
            connect_request_id: ClientRequestId::new(0),
        }
    }

    /// Returns a reference to the incoming authoritative steps.
    pub fn debug_authoritative_steps(&self) -> &Steps<StepForParticipants<Step<StepT>>> {
        &self.incoming_authoritative_steps
    }

    pub fn phase(&self) -> &ClientLogicPhase {
        &self.phase
    }

    pub fn pop_all_authoritative_steps(
        &mut self,
    ) -> (TickId, Vec<StepForParticipants<Step<StepT>>>) {
        if let Some(first_tick_id) = self.incoming_authoritative_steps.front_tick_id() {
            let vec = self.incoming_authoritative_steps.to_vec();
            self.incoming_authoritative_steps.clear();
            (first_tick_id, vec)
        } else {
            (TickId(0), vec![])
        }
    }

    /// Sets the joining player request for this client.
    ///
    /// # Arguments
    /// * `join_game_request`: The join game request to send to the host.
    pub fn set_joining_player(&mut self, local_players: Vec<LocalIndex>) {
        self.joining_player = Some(local_players);
    }

    /// Generates a download state request command to send to the host.
    ///
    /// # Arguments
    /// * `download_request_id`: The request ID for the download state.
    ///
    /// # Returns
    /// A vector of `ClientToHostCommands`.
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

    fn send_connect_request(&self) -> ClientToHostCommands<StepT> {
        let connect_request = ConnectRequest {
            nimble_version: NIMBLE_PROTOCOL_VERSION,
            use_debug_stream: false,
            application_version: Version {
                major: self.deterministic_simulation_version.major(),
                minor: self.deterministic_simulation_version.minor(),
                patch: self.deterministic_simulation_version.patch(),
            },
            client_request_id: ClientRequestId(0),
        };

        ClientToHostCommands::ConnectType(connect_request)
    }

    /// Sends the predicted steps to the host.
    ///
    /// # Returns
    /// A `ClientToHostCommands` representing the predicted steps.
    fn send_steps_request(&mut self) -> ClientToHostCommands<StepT> {
        let steps_request = StepsRequest {
            ack: StepsAck {
                waiting_for_tick_id: self.incoming_authoritative_steps.expected_write_tick_id(),
                lost_steps_mask_after_last_received: 0,
            },
            combined_predicted_steps: CombinedSteps::<StepT> {
                tick_id: self
                    .outgoing_predicted_steps
                    .front_tick_id()
                    .unwrap_or_default(),
                steps: self.outgoing_predicted_steps.to_vec(),
            },
        };

        ClientToHostCommands::Steps(steps_request)
    }

    /// Returns client commands that should be sent to the host.
    ///
    /// # Returns
    /// A vector of `ClientToHostCommands` representing all the commands to be sent to the host.
    pub fn send(&mut self) -> Vec<ClientToHostCommands<StepT>> {
        let mut commands: Vec<ClientToHostCommands<StepT>> = vec![];

        if let Some(joining_players) = &self.joining_player {
            debug!("connected. send join_game_request {:?}", joining_players);

            let player_requests = joining_players
                .iter()
                .map(|local_index| JoinPlayerRequest {
                    local_index: *local_index,
                })
                .collect();
            let join_command = ClientToHostCommands::JoinGameType(JoinGameRequest {
                client_request_id: self.joining_request_id,
                join_game_type: JoinGameType::NoSecret,
                player_requests: JoinPlayerRequests {
                    players: player_requests,
                },
            });
            trace!("send join command: {join_command:?}");
            commands.push(join_command);
        }

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
            ClientLogicPhase::RequestConnect => [self.send_connect_request()].to_vec(),
        };

        commands.extend(normal_commands);

        commands
    }

    /// Adds a predicted step to the outgoing steps queue.
    ///
    /// # Arguments
    /// * `tick_id`: The tick ID of the step.
    /// * `step`: The predicted step to add.
    ///
    /// # Errors
    /// Returns a [`StepsError`] if the step is empty or cannot be added.
    pub fn push_predicted_step(
        &mut self,
        tick_id: TickId,
        step: StepForParticipants<StepT>,
    ) -> Result<(), StepsError> {
        if step.is_empty() {
            Err(StepsError::CanNotPushEmptyPredictedSteps)?;
        }
        self.outgoing_predicted_steps.push_with_check(tick_id, step)
    }

    /// Handles the reception of the join game acceptance message from the host.
    ///
    /// # Arguments
    /// * `cmd`: The join game acceptance command.
    ///
    /// # Errors
    /// Returns a [`ClientErrorKind`] if the join game process encounters an error.
    fn on_join_game(&mut self, cmd: &JoinGameAccepted) -> Result<(), ClientErrorKind> {
        debug!("join game accepted: {:?}", cmd);

        if cmd.client_request_id != self.joining_request_id {
            Err(ClientErrorKind::WrongJoinResponseRequestId {
                encountered: cmd.client_request_id,
                expected: self.joining_request_id,
            })?;
        }

        self.joining_player = None;

        self.local_players.clear();

        for participant in &cmd.participants.0 {
            self.local_players.push(LocalPlayer {
                index: participant.local_index,
                participant_id: participant.participant_id,
            })
        }

        Ok(())
    }

    /// Returns the received game state from the host.
    ///
    /// # Returns
    /// An `Option` containing a reference to the received game state, if available.
    pub fn game_state(&self) -> Option<&StateT> {
        self.state.as_ref()
    }

    pub fn game_state_mut(&mut self) -> Option<&mut StateT> {
        self.state.as_mut()
    }

    /// Processes the game step response header received from the host.
    ///
    /// # Arguments
    /// * `header`: The game step response header.
    fn handle_game_step_header(&mut self, header: &GameStepResponseHeader) {
        let host_expected_tick_id = header.next_expected_tick_id;
        self.server_buffer_delta_tick_id
            .add(header.delta_buffer as i16);
        self.server_buffer_count.add(header.connection_buffer_count);
        trace!("removing every predicted step before {host_expected_tick_id}");
        self.outgoing_predicted_steps
            .pop_up_to(host_expected_tick_id);
    }

    fn on_connect(&mut self, cmd: &ConnectionAccepted) -> Result<(), ClientErrorKind> {
        if self.phase != ClientLogicPhase::RequestConnect {
            Err(ClientErrorKind::ReceivedConnectResponseWhenNotConnecting)?
        }

        if cmd.response_to_request != self.connect_request_id {
            Err(ClientErrorKind::WrongConnectResponseRequestId(
                cmd.response_to_request,
            ))?
        }
        self.phase = ClientLogicPhase::RequestDownloadState {
            download_state_request_id: 0x99,
        }; // TODO: proper download state request id
        debug!("set phase to connected!");
        Ok(())
    }

    /// Handles the reception of a game step response from the host.
    ///
    /// # Arguments
    /// * `cmd`: The game step response.
    ///
    /// # Errors
    /// Returns a `ClientErrorKind` if there are issues processing the game steps.
    fn on_game_step(&mut self, cmd: &GameStepResponse<Step<StepT>>) -> Result<(), ClientErrorKind> {
        trace!("game step response: {}", cmd);

        self.handle_game_step_header(&cmd.response_header);

        if cmd.authoritative_steps.ranges.is_empty() {
            return Ok(());
        }

        let mut accepted_count = 0;

        for range in &cmd.authoritative_steps.ranges {
            let mut current_authoritative_tick_id = range.tick_id;
            for combined_auth_step in &range.steps {
                if current_authoritative_tick_id
                    == self.incoming_authoritative_steps.expected_write_tick_id()
                {
                    self.incoming_authoritative_steps.push_with_check(
                        current_authoritative_tick_id,
                        combined_auth_step.clone(),
                    )?;
                    accepted_count += 1;
                }
                current_authoritative_tick_id += 1;
            }

            current_authoritative_tick_id += range.steps.len() as u32;
        }

        if accepted_count > 0 {
            trace!(
                "accepted authoritative count {accepted_count}, waiting for {}",
                self.incoming_authoritative_steps.expected_write_tick_id()
            );
        }

        Ok(())
    }

    /// Handles the reception of a download game state response.
    ///
    /// # Arguments
    /// * `download_response`: The download game state response from the host.
    ///
    /// # Errors
    /// Returns a `ClientErrorKind` if the download response is unexpected or has a mismatched request ID.
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

    /// Handles the reception of a blob stream command.
    ///
    /// # Arguments
    /// * `blob_stream_command`: The blob stream command from the host.
    ///
    /// # Errors
    /// Returns a `ClientErrorKind` if the blob stream command is unexpected.
    fn on_blob_stream(
        &mut self,
        blob_stream_command: &SenderToReceiverFrontCommands,
    ) -> Result<(), ClientErrorKind> {
        match self.phase {
            ClientLogicPhase::DownloadingState(_) => {
                self.blob_stream_client.receive(blob_stream_command)?;
                if let Some(blob_ready) = self.blob_stream_client.blob() {
                    debug!("blob stream received, phase is set to SendPredictedSteps");
                    self.phase = ClientLogicPhase::SendPredictedSteps;
                    let (deserialized, _) = StateT::deserialize(blob_ready)?;
                    self.state = Some(deserialized);
                }
            }
            _ => Err(ClientErrorKind::UnexpectedBlobChannelCommand)?,
        }
        Ok(())
    }

    /// Receives a command from the host and processes it accordingly.
    ///
    /// # Arguments
    /// * `command`: The command from the host.
    ///
    /// # Errors
    /// Returns a [`ClientErrorKind`] if the command cannot be processed.
    pub fn receive_cmd(
        &mut self,
        command: &HostToClientCommands<Step<StepT>>,
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
            HostToClientCommands::ConnectType(ref connect_accepted) => {
                self.on_connect(connect_accepted)?
            }
        }
        Ok(())
    }

    /// Processes a list of commands received from the host.
    ///
    /// # Arguments
    /// * `commands`: A slice of commands to process.
    ///
    /// # Errors
    /// Returns a `ClientError` if any command encounters an error during processing.
    pub fn receive(
        &mut self,
        commands: &[HostToClientCommands<Step<StepT>>],
    ) -> Result<(), ClientError> {
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

    /// Returns the average predicted step buffer count from the server, if available.
    ///
    /// # Returns
    /// An optional average predicted step buffer count.
    pub fn server_buffer_count(&self) -> Option<u8> {
        self.server_buffer_count
            .average()
            .map(|value| value.round() as u8)
    }

    /// Returns the average server buffer delta tick, if available.
    ///
    /// # Returns
    /// An optional average server buffer delta tick.
    pub fn server_buffer_delta_ticks(&self) -> Option<i16> {
        self.server_buffer_delta_tick_id
            .average()
            .map(|value| value.round() as i16)
    }

    pub fn local_players(&self) -> Vec<LocalPlayer> {
        self.local_players.clone()
    }
}
