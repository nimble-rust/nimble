/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::combinator::{Combinator, CombinatorError};
use app_version::Version;
use flood_rs::{Deserialize, Serialize};
use freelist_rs::{FreeList, FreeListError};
use log::{debug, info, trace};
use monotonic_time_rs::Millis;
use nimble_blob_stream::prelude::*;
use nimble_participant::ParticipantId;
use nimble_protocol::client_to_host::{
    ClientToHostCommands, ConnectRequest, DownloadGameStateRequest, StepsRequest,
};
use nimble_protocol::host_to_client::{
    AuthoritativeStepRanges, ConnectionAccepted, DownloadGameStateResponse, GameStepResponseHeader,
    HostToClientCommands, JoinGameAccepted, JoinGameParticipant, JoinGameParticipants,
    PartyAndSessionSecret,
};
use nimble_protocol::prelude::{GameStepResponse, JoinGameRequest};
use nimble_protocol::{SessionConnectionSecret, NIMBLE_PROTOCOL_VERSION};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;
use tick_id::TickId;

#[derive(Copy, Clone, Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub client_local_index: u8,
}

pub struct GameSession<StepT: Clone> {
    pub participants: HashMap<ParticipantId, Rc<RefCell<Participant>>>,
    pub participant_ids: FreeList<u8>,
    combinator: Combinator<StepT>,
}

impl<StepT: Clone> Default for GameSession<StepT> {
    fn default() -> Self {
        Self::new(TickId(0))
    }
}

pub trait GameStateProvider {
    fn state(&self, tick_id: TickId) -> (TickId, Vec<u8>);
}

impl<StepT: Clone> GameSession<StepT> {
    pub fn new(tick_id: TickId) -> Self {
        Self {
            participants: HashMap::new(),
            participant_ids: FreeList::new(0xff),
            combinator: Combinator::<StepT>::new(tick_id),
        }
    }

    pub fn create_participants(
        &mut self,
        client_local_indices: &[u8],
    ) -> Option<Vec<Rc<RefCell<Participant>>>> {
        let mut participants: Vec<Rc<RefCell<Participant>>> = vec![];

        let ids = self
            .participant_ids
            .allocate_count(client_local_indices.len())?;
        for (index, id_value) in ids.iter().enumerate() {
            let participant_id = ParticipantId(*id_value);
            let participant = Rc::new(RefCell::new(Participant {
                client_local_index: client_local_indices[index],
                id: participant_id,
            }));

            participants.push(participant.clone());

            self.participants
                .insert(participant_id, participant.clone());
        }

        Some(participants)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Phase {
    WaitingForValidConnectRequest,
    Connected,
}

pub const NIMBLE_VERSION: Version = Version::new(
    NIMBLE_PROTOCOL_VERSION.major,
    NIMBLE_PROTOCOL_VERSION.minor,
    NIMBLE_PROTOCOL_VERSION.patch,
);

#[derive(Debug)]
#[allow(clippy::new_without_default)]
pub struct Connection<StepT: Clone + Eq + Debug + Deserialize + Serialize> {
    pub participant_lookup: HashMap<ParticipantId, Rc<RefCell<Participant>>>,
    pub out_blob_stream: Option<OutLogicFront>,
    pub blob_stream_for_client_request: Option<u8>,
    last_transfer_id: u16,
    phase: Phase,
    #[allow(unused)]
    debug_counter: u16,
    phantom_data: PhantomData<StepT>,
}

#[allow(clippy::new_without_default)]
impl<StepT: Clone + Eq + Debug + Deserialize + Serialize> Connection<StepT> {
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
        required_deterministic_simulation_version: &app_version::Version,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
        self.phase = Phase::Connected;

        let connect_version = app_version::Version::new(
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
        debug!("host-stream received connect request {:?}", connect_request);
        Ok([HostToClientCommands::ConnectType(response)].into())
    }

    pub fn is_state_received_by_remote(&self) -> bool {
        self.out_blob_stream
            .as_ref()
            .map_or(false, |stream| stream.is_received_by_remote())
    }

    fn on_blob_stream(
        &mut self,
        now: Millis,
        blob_stream_command: &ReceiverToSenderFrontCommands,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
        let blob_stream = self
            .out_blob_stream
            .as_mut()
            .ok_or(HostLogicError::NoDownloadNow)?;
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

    fn on_join(
        &mut self,
        session: &mut GameSession<StepT>,
        request: &JoinGameRequest,
    ) -> Result<HostToClientCommands<StepT>, HostLogicError> {
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

    fn on_download(
        &mut self,
        tick_id_to_be_produced: TickId,
        now: Millis,
        request: &DownloadGameStateRequest,
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
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
        let blob_commands = self
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

    fn on_steps(
        &mut self,
        request: &StepsRequest<StepT>,
    ) -> Result<HostToClientCommands<StepT>, HostLogicError> {
        trace!("on_step {:?}", request);

        for combined_predicted_step in &request.combined_predicted_steps.steps {
            for participant_id in combined_predicted_step.combined_step.keys() {
                // TODO:
                if let Some(participant) = self.participant_lookup.get(participant_id) {
                    // TODO: ADD to participant queue
                    info!("participant {participant:?}");
                } else {
                    return Err(HostLogicError::UnknownPartyMember(*participant_id));
                }
            }
        }

        let game_step_response = GameStepResponse {
            response_header: GameStepResponseHeader {
                connection_buffer_count: 0,
                delta_buffer: 0,
                next_expected_tick_id: TickId(0),
            },
            authoritative_steps: AuthoritativeStepRanges { ranges: vec![] },
        };
        Ok(HostToClientCommands::GameStep(game_step_response))
    }
}

#[derive(Debug)]
pub enum HostLogicError {
    UnknownConnectionId(HostConnectionId),
    FreeListError {
        connection_id: HostConnectionId,
        message: FreeListError,
    },
    UnknownPartyMember(ParticipantId),
    NoFreeParticipantIds,
    BlobStreamErr(OutStreamError),
    NoDownloadNow,
    CombinatorError(CombinatorError),
    NeedConnectRequestFirst,
    WrongApplicationVersion,
}

impl From<CombinatorError> for HostLogicError {
    fn from(err: CombinatorError) -> Self {
        Self::CombinatorError(err)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct HostConnectionId(pub u8);

pub struct HostLogic<
    StepT: Clone + std::cmp::Eq + std::fmt::Debug + flood_rs::Deserialize + flood_rs::Serialize,
> {
    #[allow(unused)]
    connections: HashMap<u8, Connection<StepT>>,
    session: GameSession<StepT>,
    free_list: FreeList<u8>,
    deterministic_simulation_version: app_version::Version,
}

impl<StepT: Clone + Eq + Debug + Deserialize + Serialize> HostLogic<StepT> {
    pub fn new(tick_id: TickId, deterministic_simulation_version: app_version::Version) -> Self {
        Self {
            connections: HashMap::new(),
            session: GameSession::new(tick_id),
            free_list: FreeList::<u8>::new(0xff),
            deterministic_simulation_version,
        }
    }

    pub fn create_connection(&mut self) -> Option<HostConnectionId> {
        let new_connection_id = self.free_list.allocate();
        if let Some(id) = new_connection_id {
            self.connections.insert(id, Connection::new());
            Some(HostConnectionId(id))
        } else {
            None
        }
    }

    pub fn get(&self, connection_id: HostConnectionId) -> Option<&Connection<StepT>> {
        self.connections.get(&connection_id.0)
    }

    pub fn destroy_connection(
        &mut self,
        connection_id: HostConnectionId,
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

    pub fn session(&self) -> &GameSession<StepT> {
        &self.session
    }

    pub fn update(
        &mut self,
        connection_id: HostConnectionId,
        now: Millis,
        request: &ClientToHostCommands<StepT>,
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<HostToClientCommands<StepT>>, HostLogicError> {
        if let Some(ref mut connection) = self.connections.get_mut(&connection_id.0) {
            match &connection.phase {
                Phase::Connected => match request {
                    ClientToHostCommands::JoinGameType(join_game_request) => {
                        Ok(vec![
                            connection.on_join(&mut self.session, join_game_request)?
                        ])
                    }
                    ClientToHostCommands::Steps(add_steps_request) => {
                        for combined_step in &add_steps_request.combined_predicted_steps.steps {
                            for (participant_id, step) in combined_step.combined_step.into_iter() {
                                if !connection.participant_lookup.contains_key(participant_id) {
                                    Err(HostLogicError::UnknownPartyMember(*participant_id))?;
                                }
                                self.session.combinator.add(*participant_id, step.clone())?;
                            }
                        }
                        Ok(vec![connection.on_steps(add_steps_request)?])
                    }
                    ClientToHostCommands::DownloadGameState(download_game_state_request) => {
                        Ok(connection.on_download(
                            self.session.combinator.tick_id_to_produce,
                            now,
                            download_game_state_request,
                            state_provider,
                        )?)
                    }
                    ClientToHostCommands::BlobStreamChannel(blob_stream_command) => {
                        connection.on_blob_stream(now, blob_stream_command)
                    }
                    ClientToHostCommands::ConnectType(connect_request) => {
                        trace!("notice: got connection request, even though we are connected, but will send response anyway");
                        connection
                            .on_connect(connect_request, &self.deterministic_simulation_version)
                    }
                },
                Phase::WaitingForValidConnectRequest => match request {
                    ClientToHostCommands::ConnectType(connect_request) => connection
                        .on_connect(connect_request, &self.deterministic_simulation_version),
                    _ => Err(HostLogicError::NeedConnectRequestFirst),
                },
            }
        } else {
            Err(HostLogicError::UnknownConnectionId(connection_id))
        }
    }
}
