/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod combinator;
mod combine;
pub mod connection;

use crate::combinator::CombinatorError;
use crate::combine::{HostCombinator, HostCombinatorError};
use crate::connection::Connection;
use app_version::Version;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use flood_rs::{Deserialize, Serialize};
use freelist_rs::{FreeList, FreeListError};
use log::trace;
use monotonic_time_rs::Millis;
use nimble_blob_stream::prelude::*;
use nimble_participant::ParticipantId;
use nimble_protocol::prelude::{ClientToHostCommands, HostToClientCommands};
use nimble_protocol::NIMBLE_PROTOCOL_VERSION;
use nimble_step::Step;
use nimble_steps::StepsError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use tick_id::TickId;

#[derive(Copy, Clone, Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub client_local_index: u8,
}

pub struct GameSession<StepT: Clone + std::fmt::Display> {
    pub participants: HashMap<ParticipantId, Rc<RefCell<Participant>>>,
    pub participant_ids: FreeList<u8>,
    combinator: HostCombinator<StepT>,
}

impl<StepT: Clone + std::fmt::Display> Default for GameSession<StepT> {
    fn default() -> Self {
        Self::new(TickId(0))
    }
}

pub trait GameStateProvider {
    fn state(&self, tick_id: TickId) -> (TickId, Vec<u8>);
}

impl<StepT: Clone + std::fmt::Display> GameSession<StepT> {
    pub fn new(tick_id: TickId) -> Self {
        Self {
            participants: HashMap::new(),
            participant_ids: FreeList::new(0xff),
            combinator: HostCombinator::<StepT>::new(tick_id),
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
    HostCombinatorError(HostCombinatorError),
    NeedConnectRequestFirst,
    WrongApplicationVersion,
    StepsError(StepsError),
}

impl ErrorLevelProvider for HostLogicError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::UnknownConnectionId(_) => ErrorLevel::Warning,
            Self::FreeListError { .. } => ErrorLevel::Critical,
            Self::UnknownPartyMember(_) => ErrorLevel::Warning,
            Self::NoFreeParticipantIds => ErrorLevel::Warning,
            Self::BlobStreamErr(_) => ErrorLevel::Info,
            Self::NoDownloadNow => ErrorLevel::Info,
            Self::CombinatorError(err) => err.error_level(),
            Self::HostCombinatorError(err) => err.error_level(),
            Self::NeedConnectRequestFirst => ErrorLevel::Info,
            Self::WrongApplicationVersion => ErrorLevel::Critical,
            Self::StepsError(_) => ErrorLevel::Critical,
        }
    }
}

impl From<CombinatorError> for HostLogicError {
    fn from(err: CombinatorError) -> Self {
        Self::CombinatorError(err)
    }
}

impl From<StepsError> for HostLogicError {
    fn from(err: StepsError) -> Self {
        Self::StepsError(err)
    }
}

impl From<HostCombinatorError> for HostLogicError {
    fn from(err: HostCombinatorError) -> Self {
        Self::HostCombinatorError(err)
    }
}

impl From<OutStreamError> for HostLogicError {
    fn from(err: OutStreamError) -> Self {
        Self::BlobStreamErr(err)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct HostConnectionId(pub u8);

pub struct HostLogic<StepT: Clone + Eq + Debug + Deserialize + Serialize + std::fmt::Display> {
    #[allow(unused)]
    connections: HashMap<u8, Connection<StepT>>,
    session: GameSession<StepT>,
    free_list: FreeList<u8>,
    deterministic_simulation_version: Version,
}

impl<StepT: Clone + Eq + Debug + Deserialize + Serialize + std::fmt::Display> HostLogic<StepT> {
    pub fn new(tick_id: TickId, deterministic_simulation_version: Version) -> Self {
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

    pub fn post_update(&mut self) {
        self.session.combinator.produce_authoritative_steps()
    }

    pub fn update(
        &mut self,
        connection_id: HostConnectionId,
        now: Millis,
        request: &ClientToHostCommands<StepT>,
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<HostToClientCommands<Step<StepT>>>, HostLogicError> {
        //trace!("host_logic: receive: \n{request}");
        if let Some(ref mut connection) = self.connections.get_mut(&connection_id.0) {
            match &connection.phase {
                Phase::Connected => {
                    match request {
                        ClientToHostCommands::JoinGameType(join_game_request) => Ok(vec![
                            connection.on_join(&mut self.session, join_game_request)?,
                        ]),
                        ClientToHostCommands::Steps(add_steps_request) => {
                            Ok(vec![connection.on_steps(
                                &mut self.session.combinator,
                                add_steps_request,
                            )?])
                        }
                        ClientToHostCommands::DownloadGameState(download_game_state_request) => {
                            Ok(connection.on_download(
                                self.session.combinator.tick_id_to_produce(),
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
                    }
                }
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
