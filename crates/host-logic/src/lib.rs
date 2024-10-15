/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod combinator;
mod combine;
pub mod connection;
pub mod err;
pub mod session;

use crate::connection::Connection;
use crate::err::HostLogicError;
use app_version::Version;
use flood_rs::{Deserialize, Serialize};
use freelist_rs::FreeList;
use log::trace;
use monotonic_time_rs::Millis;
use nimble_protocol::prelude::{ClientToHostCommands, HostToClientCommands};
use nimble_protocol::NIMBLE_PROTOCOL_VERSION;
use nimble_step::Step;
use std::collections::HashMap;
use std::fmt::Debug;
use tick_id::TickId;
use crate::session::GameSession;

pub trait GameStateProvider {
    fn state(&self, tick_id: TickId) -> (TickId, Vec<u8>);
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
