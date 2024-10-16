/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*!

# Nimble Client Logic

`nimble-client-logic` is a Rust crate designed to manage the client-side logic for multiplayer game
sessions using the Nimble protocol. It handles the creation and processing of messages,
including requests and responses:

- Download a complete game state from the host.
- Add and remove participants from the game session.
- Send predicted player inputs (steps) to the host.
- Receive authoritative combined steps from the host.

This crate ensures seamless synchronization between the client and host, maintaining the integrity
and consistency of the game state across all participants.

## Features

- **Connection Management**: Handles connecting to the host, agreeing on protocol versions,
    and managing connection states.
- **Game State Handling**: Downloads and maintains the complete game state from the host.
- **Participant Management**: Adds and removes players from the game session dynamically.
- **Step Prediction and Reconciliation**: Sends predicted player steps to the host and reconciles
    them with authoritative steps received from the host.
- **Blob Streaming**: Manages blob streaming for efficient game state transfers.

## Usage

Add `nimble-client-logic` to your `Cargo.toml`:

```toml
[dependencies]
nimble-client-logic = "0.0.14-dev"
```

*/

pub mod err;

use crate::err::ClientLogicError;
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
use nimble_step_map::StepMap;
use std::fmt::Debug;
use tick_id::TickId;
use tick_queue::Queue;

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

pub type LocalIndex = u8;

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

    connect_request_id: Option<ClientRequestId>,

    /// Represents the player's join game request, if available.
    joining_player: Option<Vec<LocalIndex>>,

    /// Holds the current game state.
    state: Option<StateT>,

    /// Manages the blob stream logic for the client.
    blob_stream_client: FrontLogic,

    /// Stores the outgoing predicted steps from the client.
    outgoing_predicted_steps: Queue<StepMap<StepT>>,

    /// Stores the incoming authoritative steps from the host.
    incoming_authoritative_steps: Queue<StepMap<Step<StepT>>>,

    /// Represents the current phase of the client's logic.
    phase: ClientLogicPhase,

    /// Tracks the delta of tick id on the server.
    server_buffer_delta_tick_id: AggregateMetric<i16>,

    /// Tracks the buffer step count on the server.
    //server_buffer_count: AggregateMetric<u8>,
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
            outgoing_predicted_steps: Queue::default(),
            incoming_authoritative_steps: Queue::default(),
            server_buffer_delta_tick_id: AggregateMetric::new(3).unwrap(),
            //server_buffer_count: AggregateMetric::new(3).unwrap(),
            state: None,
            phase: ClientLogicPhase::RequestConnect,
            local_players: Vec::new(),
            deterministic_simulation_version,
            connect_request_id: None,
        }
    }

    /// Returns a reference to the incoming authoritative steps.
    pub fn debug_authoritative_steps(&self) -> &Queue<StepMap<Step<StepT>>> {
        &self.incoming_authoritative_steps
    }

    pub fn phase(&self) -> &ClientLogicPhase {
        &self.phase
    }

    pub fn pop_all_authoritative_steps(
        &mut self,
    ) -> (TickId, Vec<StepMap<Step<StepT>>>) {
        if let Some(first_tick_id) = self.incoming_authoritative_steps.front_tick_id() {
            let vec = self.incoming_authoritative_steps.to_vec();
            self.incoming_authoritative_steps.clear(first_tick_id + 1);
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

    fn send_connect_request(&mut self) -> ClientToHostCommands<StepT> {
        if self.connect_request_id.is_none() {
            self.connect_request_id = Some(ClientRequestId(0));
        }

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

    pub fn debug_connect_request_id(&self) -> Option<ClientRequestId> {
        self.connect_request_id
    }

    /// Returns client commands that should be sent to the host.
    ///
    /// # Returns
    /// A vector of `ClientToHostCommands` representing all the commands to be sent to the host.
    #[must_use]
    pub fn send(&mut self) -> Vec<ClientToHostCommands<StepT>> {
        let mut commands: Vec<ClientToHostCommands<StepT>> = vec![];

        if self.phase != ClientLogicPhase::RequestConnect {
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

    pub fn can_push_predicted_step(&self) -> bool {
        self.is_in_game() && self.game().is_some()
    }

    pub fn is_in_game(&self) -> bool {
        self.phase == ClientLogicPhase::SendPredictedSteps
            && self.joining_player.is_none()
            && !self.local_players.is_empty()
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
        step: StepMap<StepT>,
    ) -> Result<(), ClientLogicError> {
        self.outgoing_predicted_steps.push(tick_id, step)?;
        Ok(())
    }

    pub fn predicted_step_count_in_queue(&self) -> usize {
        self.outgoing_predicted_steps.len()
    }

    /// Handles the reception of the join game acceptance message from the host.
    ///
    /// # Arguments
    /// * `cmd`: The join game acceptance command.
    ///
    /// # Errors
    /// Returns a [`ClientErrorKind`] if the join game process encounters an error.
    fn on_join_game(&mut self, cmd: &JoinGameAccepted) -> Result<(), ClientLogicError> {
        debug!("join game accepted: {:?}", cmd);

        if cmd.client_request_id != self.joining_request_id {
            Err(ClientLogicError::WrongJoinResponseRequestId {
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
    pub fn game(&self) -> Option<&StateT> {
        self.state.as_ref()
    }

    pub fn game_mut(&mut self) -> Option<&mut StateT> {
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
        //self.server_buffer_count.add(header.connection_buffer_count);
        trace!("removing every predicted step before {host_expected_tick_id}");
        self.outgoing_predicted_steps
            .discard_up_to(host_expected_tick_id);
        trace!(
            "predicted steps remaining {}",
            self.outgoing_predicted_steps.len()
        );
    }

    fn on_connect(&mut self, cmd: &ConnectionAccepted) -> Result<(), ClientLogicError> {
        if self.phase != ClientLogicPhase::RequestConnect {
            Err(ClientLogicError::ReceivedConnectResponseWhenNotConnecting)?
        }

        if self.connect_request_id.is_none() {
            Err(ClientLogicError::ReceivedConnectResponseWhenNotConnecting)?;
        }

        if cmd.response_to_request != self.connect_request_id.unwrap() {
            Err(ClientLogicError::WrongConnectResponseRequestId(
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
    fn on_game_step(
        &mut self,
        cmd: &GameStepResponse<Step<StepT>>,
    ) -> Result<(), ClientLogicError> {
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
                    self.incoming_authoritative_steps
                        .push(current_authoritative_tick_id, combined_auth_step.clone())?;
                    accepted_count += 1;
                }
                current_authoritative_tick_id += 1;
            }

            current_authoritative_tick_id += range.steps.len() as u32;
        }

        if accepted_count > 0 {
            trace!(
                "accepted {accepted_count} auth steps, waiting for {}, total count: {}",
                self.incoming_authoritative_steps.expected_write_tick_id(),
                self.incoming_authoritative_steps.len(),
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
    ) -> Result<(), ClientLogicError> {
        match self.phase {
            ClientLogicPhase::RequestDownloadState {
                download_state_request_id,
            } => {
                if download_response.client_request != download_state_request_id {
                    Err(ClientLogicError::WrongDownloadRequestId)?;
                }
            }
            _ => Err(ClientLogicError::DownloadResponseWasUnexpected)?,
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
    ) -> Result<(), ClientLogicError> {
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
            _ => Err(ClientLogicError::UnexpectedBlobChannelCommand)?,
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
    pub fn receive(
        &mut self,
        command: &HostToClientCommands<Step<StepT>>,
    ) -> Result<(), ClientLogicError> {
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
