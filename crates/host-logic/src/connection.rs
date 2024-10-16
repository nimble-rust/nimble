/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use crate::combine::HostCombinator;
use crate::session::Participant;
use crate::{GameSession, GameStateProvider, HostLogicError, Phase};
use app_version::Version;
use flood_rs::{Deserialize, Serialize};
use log::{debug, trace};
use monotonic_time_rs::Millis;
use nimble_blob_stream::out_logic_front::OutLogicFront;
use nimble_blob_stream::prelude::{ReceiverToSenderFrontCommands, TransferId};
use nimble_participant::ParticipantId;
use nimble_protocol::client_to_host::{
    ConnectRequest, DownloadGameStateRequest, JoinGameRequest, StepsRequest,
};
use nimble_protocol::host_to_client::{
    AuthoritativeStepRanges, ConnectionAccepted, DownloadGameStateResponse, GameStepResponse,
    GameStepResponseHeader, HostToClientCommands, JoinGameAccepted, JoinGameParticipant,
    JoinGameParticipants, PartyAndSessionSecret,
};
use nimble_protocol::prelude::CombinedSteps;
use nimble_protocol::SessionConnectionSecret;
use nimble_step::Step;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;
use tick_id::TickId;

#[derive(Debug)]
#[allow(clippy::new_without_default)]
pub struct Connection<StepT: Clone + Eq + Debug + Deserialize + Serialize> {
    pub participant_lookup: HashMap<ParticipantId, Rc<RefCell<Participant>>>,
    pub out_blob_stream: Option<OutLogicFront>,
    pub blob_stream_for_client_request: Option<u8>,
    last_transfer_id: u16,
    pub(crate) phase: Phase,
    #[allow(unused)]
    debug_counter: u16,
    phantom_data: PhantomData<StepT>,
}

#[allow(clippy::new_without_default)]
impl<StepT: Clone + Eq + Debug + Deserialize + Serialize + std::fmt::Display> Connection<StepT> {
    pub fn new() -> Self {
        Self {
            participant_lookup: Default::default(),
            out_blob_stream: None,
            blob_stream_for_client_request: None,
            last_transfer_id: 0,
            debug_counter: 0,
            phase: Phase::WaitingForValidConnectRequest,
            phantom_data: PhantomData,
        }
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn on_connect(
        &mut self,
        connect_request: &ConnectRequest,
        required_deterministic_simulation_version: &Version,
    ) -> Result<Vec<HostToClientCommands<Step<StepT>>>, HostLogicError> {
        self.phase = Phase::Connected;

        let connect_version = Version::new(
            connect_request.application_version.major,
            connect_request.application_version.minor,
            connect_request.application_version.patch,
        );

        if connect_version != *required_deterministic_simulation_version {
            return Err(HostLogicError::WrongApplicationVersion);
        }

        let response = ConnectionAccepted {
            flags: 0,
            response_to_request: connect_request.client_request_id,
        };
        debug!(
            "host-stream received connect request {:?} and responding:\n{:?}",
            connect_request, response
        );
        Ok([HostToClientCommands::ConnectType(response)].into())
    }

    pub fn is_state_received_by_remote(&self) -> bool {
        self.out_blob_stream
            .as_ref()
            .map_or(false, |stream| stream.is_received_by_remote())
    }

    pub(crate) fn on_blob_stream(
        &mut self,
        now: Millis,
        blob_stream_command: &ReceiverToSenderFrontCommands,
    ) -> Result<Vec<HostToClientCommands<Step<StepT>>>, HostLogicError> {
        let blob_stream = self
            .out_blob_stream
            .as_mut()
            .ok_or(HostLogicError::NoDownloadNow)?;
        blob_stream.receive(blob_stream_command)?;
        let blob_commands = blob_stream.send(now)?;

        let converted_commands: Vec<_> = blob_commands
            .into_iter()
            .map(HostToClientCommands::BlobStreamChannel)
            .collect();

        Ok(converted_commands)
    }

    pub(crate) fn on_join(
        &mut self,
        session: &mut GameSession<StepT>,
        request: &JoinGameRequest,
    ) -> Result<HostToClientCommands<Step<StepT>>, HostLogicError> {
        debug!("on_join {:?}", request);

        if request.player_requests.players.is_empty() {
            return Err(HostLogicError::NoFreeParticipantIds);
        }

        let local_indices: Vec<_> = request
            .player_requests
            .players
            .iter()
            .map(|p| p.local_index)
            .collect();

        let participants = session
            .create_participants(local_indices.as_slice())
            .ok_or(HostLogicError::NoFreeParticipantIds)?;

        for participant in &participants {
            self.participant_lookup
                .insert(participant.borrow().id, participant.clone());
            session.combinator.create_buffer(participant.borrow().id);
        }

        let join_game_participants = participants
            .iter()
            .map(|found_participant| JoinGameParticipant {
                local_index: found_participant.borrow().client_local_index,
                participant_id: found_participant.borrow().id,
            })
            .collect();

        let join_accepted = JoinGameAccepted {
            client_request_id: request.client_request_id,
            party_and_session_secret: PartyAndSessionSecret {
                session_secret: SessionConnectionSecret { value: 0 },
                party_id: 0,
            },
            participants: JoinGameParticipants(join_game_participants),
        };

        Ok(HostToClientCommands::JoinGame(join_accepted))
    }

    pub(crate) fn on_download(
        &mut self,
        tick_id_to_be_produced: TickId,
        now: Millis,
        request: &DownloadGameStateRequest,
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<HostToClientCommands<Step<StepT>>>, HostLogicError> {
        debug!("client requested download {:?}", request);
        let (state_tick_id, state_vec) = state_provider.state(tick_id_to_be_produced);

        const FIXED_CHUNK_SIZE: usize = 1024;
        const RESEND_DURATION: Duration = Duration::from_millis(32 * 3);

        let is_new_request = if let Some(x) = self.blob_stream_for_client_request {
            x == request.request_id
        } else {
            true
        };
        if is_new_request {
            self.last_transfer_id += 1;
            let transfer_id = TransferId(self.last_transfer_id);
            self.out_blob_stream = Some(OutLogicFront::new(
                transfer_id,
                FIXED_CHUNK_SIZE,
                RESEND_DURATION,
                state_vec.as_slice(),
            ));
        }

        let response = DownloadGameStateResponse {
            client_request: request.request_id,
            tick_id: state_tick_id,
            blob_stream_channel: self.out_blob_stream.as_ref().unwrap().transfer_id().0,
        };
        let mut commands = vec![];
        commands.push(HostToClientCommands::DownloadGameState(response));

        // Since most datagram transports have a very low packet drop rate,
        // this implementation is optimized for the high likelihood of datagram delivery.
        // So we start including the first blob commands right away
        let blob_commands = self.out_blob_stream.as_mut().unwrap().send(now)?;
        let converted_blob_commands: Vec<_> = blob_commands
            .into_iter()
            .map(HostToClientCommands::BlobStreamChannel)
            .collect();
        commands.extend(converted_blob_commands);

        Ok(commands)
    }

    pub(crate) fn on_steps(
        &mut self,
        combinator: &mut HostCombinator<StepT>,
        request: &StepsRequest<StepT>,
    ) -> Result<HostToClientCommands<Step<StepT>>, HostLogicError> {
        trace!("on incoming predicted steps {}", request);

        /*
                               let mut tick = add_steps_request.combined_predicted_steps.tick_id;
                       for combined_step in &add_steps_request.combined_predicted_steps.steps {
                           for (participant_id, step) in combined_step.combined_step.into_iter() {
                               if !connection.participant_lookup.contains_key(participant_id) {
                                   Err(HostLogicError::UnknownPartyMember(*participant_id))?;
                               }
                               self.session
                                   .combinator
                                   .receive_step(*participant_id, tick, step.clone())?;
                           }
                           tick += 1;
                       }
        */

        let mut current_tick = request.combined_predicted_steps.tick_id;
        for combined_predicted_step in &request.combined_predicted_steps.steps {
            for participant_id in combined_predicted_step.keys() {
                // TODO:
                if self.participant_lookup.contains_key(participant_id) {
                    let part = combined_predicted_step.get(participant_id).unwrap();

                    let buffer = combinator
                        .get_mut(participant_id)
                        .expect("since the participant lookup worked, there should be a buffer");
                    if buffer.expected_write_tick_id() != current_tick {
                        continue;
                    }
                    buffer.push(current_tick, part.clone())?;
                } else {
                    return Err(HostLogicError::UnknownPartyMember(*participant_id));
                }
            }
            current_tick += 1;
        }

        let authoritative_steps = combinator.authoritative_steps();

        let combined_steps_vec =
            if let Some(found_first_tick_id) = authoritative_steps.front_tick_id() {
                let combined_steps = CombinedSteps::<Step<StepT>> {
                    tick_id: found_first_tick_id,
                    steps: authoritative_steps.to_vec(),
                };
                vec![combined_steps]
            } else {
                vec![]
            };

        let game_step_response = GameStepResponse {
            response_header: GameStepResponseHeader {
                connection_buffer_count: 0,
                delta_buffer: 0,
                next_expected_tick_id: combinator.tick_id_to_produce(),
            },
            authoritative_steps: AuthoritativeStepRanges {
                ranges: combined_steps_vec,
            },
        };

        trace!("sending auth steps: {}", game_step_response);
        Ok(HostToClientCommands::GameStep(game_step_response))
    }
}
