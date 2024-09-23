/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::combinator::Combinator;
use crate::state::State;
use flood_rs::{Deserialize, Serialize};
use freelist_rs::FreeList;
use log::{debug, info, trace};
use nimble_blob_stream::prelude::*;
use nimble_participant::ParticipantId;
use nimble_protocol::client_to_host::{
    ClientToHostCommands, DownloadGameStateRequest, StepsRequest,
};
use nimble_protocol::host_to_client::{
    AuthoritativeStepRanges, DownloadGameStateResponse, GameStepResponseHeader,
    HostToClientCommands, JoinGameAccepted, JoinGameParticipants, PartyAndSessionSecret,
};
use nimble_protocol::prelude::{GameStepResponse, JoinGameRequest};
use nimble_protocol::SessionConnectionSecret;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tick_id::TickId;

#[derive(Copy, Clone, Debug)]
pub struct ConnectionId(pub u8);

#[derive(Copy, Clone, Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub client_local_index: u8,
}

pub struct GameSession {
    pub state: State,
    pub participants: HashMap<ParticipantId, Rc<RefCell<Participant>>>,
    pub participant_ids: FreeList,
}

impl GameSession {
    pub fn new(state: State) -> Self {
        Self {
            state,
            participants: HashMap::new(),
            participant_ids: FreeList::new(0xff),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub(crate) fn create_participant(
        &mut self,
        client_local_index: u8,
    ) -> Option<Rc<RefCell<Participant>>> {
        let participant_id_value = self.participant_ids.allocate();
        if let Some(id_value) = participant_id_value {
            let participant_id = ParticipantId(id_value);
            let participant = Rc::new(RefCell::new(Participant {
                client_local_index,
                id: participant_id,
            }));

            self.participants
                .insert(participant_id, participant.clone());
            Some(participant)
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[allow(clippy::new_without_default)]
pub struct Connection {
    pub participant_lookup: HashMap<u8, Rc<RefCell<Participant>>>,
    pub out_blob_stream: Option<OutLogicFront>,
    pub blob_stream_for_client_request: Option<u8>,
    last_transfer_id: u16,
    #[allow(unused)]
    debug_counter: u16,
}

#[allow(clippy::new_without_default)]
impl Connection {
    pub fn new() -> Self {
        Self {
            participant_lookup: Default::default(),
            out_blob_stream: None,
            blob_stream_for_client_request: None,
            last_transfer_id: 0,
            debug_counter: 0,
        }
    }

    pub fn is_state_received_by_remote(&self) -> bool {
        self.out_blob_stream
            .as_ref()
            .map_or(false, |stream| stream.is_received_by_remote())
    }
}

#[derive(Debug)]
pub enum HostLogicError {
    UnknownConnectionId(ConnectionId),
    FreeListError {
        connection_id: ConnectionId,
        message: String,
    },
    UnknownPartyMemberIndex(u8),
    NoFreeParticipantIds,
    BlobStreamErr(OutStreamError),
}

pub struct HostLogic<StepT> {
    #[allow(unused)]
    combinator: Combinator<StepT>,
    connections: HashMap<u8, Connection>,
    session: GameSession,
    free_list: FreeList,
}

impl<StepT: std::clone::Clone + Eq + Debug + Deserialize + Serialize> HostLogic<StepT> {
    pub fn new(last_known_state: State) -> Self {
        Self {
            combinator: Combinator::<StepT>::new(last_known_state.tick_id),
            connections: HashMap::new(),
            session: GameSession::new(last_known_state),
            free_list: FreeList::new(0xff),
        }
    }

    pub fn create_connection(&mut self) -> Option<ConnectionId> {
        let new_connection_id = self.free_list.allocate();
        if let Some(id) = new_connection_id {
            self.connections.insert(id, Connection::new());
            Some(ConnectionId(id))
        } else {
            None
        }
    }

    pub fn get(&self, connection_id: ConnectionId) -> Option<&Connection> {
        self.connections.get(&connection_id.0)
    }

    pub fn destroy_connection(
        &mut self,
        connection_id: ConnectionId,
    ) -> Result<(), HostLogicError> {
        self.free_list
            .free(connection_id.0)
            .map_err(|err| HostLogicError::FreeListError {
                connection_id,
                message: err,
            })?;

        if self.connections.remove(&connection_id.0).is_some() {
            Ok(())
        } else {
            Err(HostLogicError::UnknownConnectionId(connection_id))
        }
    }

    fn on_join(
        &mut self,
        connection_id: ConnectionId,
        request: &JoinGameRequest,
    ) -> Result<HostToClientCommands<StepT>, HostLogicError> {
        debug!("on_join {:?}", request);

        let join_accepted = JoinGameAccepted {
            client_request_id: request.client_request_id,
            party_and_session_secret: PartyAndSessionSecret {
                session_secret: SessionConnectionSecret { value: 0 },
                party_id: 0,
            },
            participants: JoinGameParticipants(vec![]),
        };

        let participant = self
            .session
            .create_participant(request.player_requests.players[0].local_index)
            .ok_or(HostLogicError::NoFreeParticipantIds)?;
        let connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostLogicError::UnknownConnectionId(connection_id))?;
        connection
            .participant_lookup
            .insert(request.player_requests.players[0].local_index, participant);
        Ok(HostToClientCommands::JoinGame(join_accepted))
    }

    fn on_steps(
        &mut self,
        connection_id: ConnectionId,
        request: &StepsRequest<StepT>,
    ) -> Result<HostToClientCommands<StepT>, HostLogicError> {
        trace!("on_step {:?}", request);
        /*
               for participant in request.combined_predicted_steps.predicted_steps_for_players {
                   self.combinator.receive_step(participant.participant_party_index)

               }

        */

        let connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostLogicError::UnknownConnectionId(connection_id))?;

        for combined_predicted_step in &request.combined_predicted_steps.steps {
            for local_index in combined_predicted_step.predicted_players.keys() {
                // TODO:
                if let Some(participant) = connection.participant_lookup.get(local_index) {
                    // TODO: ADD to participant queue
                    info!("participant {participant:?}");
                } else {
                    return Err(HostLogicError::UnknownPartyMemberIndex(*local_index));
                }
            }
        }

        let game_step_response = GameStepResponse {
            response_header: GameStepResponseHeader {
                connection_buffer_count: 0,
                delta_buffer: 0,
                last_step_received_from_client: 0,
            },
            authoritative_steps: AuthoritativeStepRanges { ranges: vec![] },
        };
        Ok(HostToClientCommands::GameStep(game_step_response))
    }

    fn on_download(
        &mut self,
        connection_id: ConnectionId,
        now: Instant,
        request: &DownloadGameStateRequest,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
        debug!("client requested download {:?}", request);
        let state = self.session.state();
        let connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostLogicError::UnknownConnectionId(connection_id))?;

        const FIXED_CHUNK_SIZE: usize = 1024;
        const RESEND_DURATION: Duration = Duration::from_millis(32 * 3);

        let is_new_request = if let Some(x) = connection.blob_stream_for_client_request {
            x == request.request_id
        } else {
            true
        };
        if is_new_request {
            connection.last_transfer_id += 1;
            let transfer_id = TransferId(connection.last_transfer_id);
            connection.out_blob_stream = Some(OutLogicFront::new(
                transfer_id,
                FIXED_CHUNK_SIZE,
                RESEND_DURATION,
                self.session.state().data.as_slice(),
            ));
        }

        let response = DownloadGameStateResponse {
            client_request: request.request_id,
            tick_id: TickId(state.tick_id.0),
            blob_stream_channel: connection.out_blob_stream.as_ref().unwrap().transfer_id().0,
        };
        let mut commands = vec![];
        commands.push(HostToClientCommands::DownloadGameState(response));

        // Since most datagram transports have a very low packet drop rate,
        // this implementation is optimized for the high likelihood of datagram delivery.
        // So we start including the first blob commands right away
        let blob_commands = connection
            .out_blob_stream
            .as_mut()
            .unwrap()
            .send(now)
            .map_err(HostLogicError::BlobStreamErr)?;
        let converted_blob_commands: Vec<_> = blob_commands
            .into_iter()
            .map(HostToClientCommands::BlobStreamChannel)
            .collect();
        commands.extend(converted_blob_commands);

        Ok(commands)
    }

    fn on_blob_stream(
        &mut self,
        connection_id: ConnectionId,
        now: Instant,
        blob_stream_command: &ReceiverToSenderFrontCommands,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
        let connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostLogicError::UnknownConnectionId(connection_id))?;

        let blob_stream = connection
            .out_blob_stream
            .as_mut()
            .ok_or(HostLogicError::UnknownConnectionId(connection_id))?;
        blob_stream
            .receive(blob_stream_command)
            .map_err(HostLogicError::BlobStreamErr)?;
        let blob_commands = blob_stream
            .send(now)
            .map_err(HostLogicError::BlobStreamErr)?;

        let converted_commands: Vec<_> = blob_commands
            .into_iter()
            .map(HostToClientCommands::BlobStreamChannel)
            .collect();

        Ok(converted_commands)
    }

    pub fn update(
        &mut self,
        connection_id: ConnectionId,
        now: Instant,
        request: &ClientToHostCommands<StepT>,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
        match request {
            ClientToHostCommands::JoinGameType(join_game_request) => {
                Ok(vec![self.on_join(connection_id, join_game_request)?])
            }
            ClientToHostCommands::Steps(add_steps_request) => {
                Ok(vec![self.on_steps(connection_id, add_steps_request)?])
            }
            ClientToHostCommands::DownloadGameState(download_game_state_request) => {
                Ok(self.on_download(connection_id, now, download_game_state_request)?)
            }
            ClientToHostCommands::BlobStreamChannel(blob_stream_command) => {
                Ok(self.on_blob_stream(connection_id, now, blob_stream_command)?)
            }
        }
    }
}
