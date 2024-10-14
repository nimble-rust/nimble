/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
pub mod err;
pub mod prelude;

use crate::err::HostError;
use datagram_chunker::DatagramChunker;
use flood_rs::prelude::OutOctetStream;
use flood_rs::{Deserialize, Serialize};
use hexify::format_hex;
use log::{debug, trace};
use monotonic_time_rs::Millis;
use nimble_host_logic::{connection::Connection, GameSession, GameStateProvider, HostLogic};
use nimble_layer::NimbleLayer;
use nimble_protocol::prelude::ClientToHostCommands;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use tick_id::TickId;

#[derive(Default, Debug)]
pub struct HostConnection {
    layer: NimbleLayer,
}
impl HostConnection {
    pub fn new() -> Self {
        Self {
            layer: Default::default(),
        }
    }
}

pub struct ConnectionId(u8);

impl ConnectionId {
    pub fn inner(&self) -> u8 {
        self.0
    }
}

pub struct Host<StepT: Clone + Debug + Eq + Deserialize + Serialize + Display> {
    logic: HostLogic<StepT>,
    connections: HashMap<u8, HostConnection>,
}

impl<StepT: Clone + Deserialize + Serialize + Eq + Debug + Display> Host<StepT> {
    pub fn new(app_version: app_version::Version, tick_id: TickId) -> Self {
        Self {
            logic: HostLogic::<StepT>::new(tick_id, app_version),
            connections: HashMap::new(),
        }
    }

    pub fn debug_logic(&self) -> &HostLogic<StepT> {
        &self.logic
    }

    pub fn debug_get_logic(
        &self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Option<&Connection<StepT>> {
        self.logic.get(connection_id)
    }

    pub fn session(&self) -> &GameSession<StepT> {
        self.logic.session()
    }

    pub fn update(
        &mut self,
        connection_id: nimble_host_logic::HostConnectionId,
        now: Millis,
        datagram: &[u8],
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<Vec<u8>>, HostError> {
        trace!(
            "time:{now}: host received for connection:{} payload:\n{}",
            connection_id.0,
            format_hex(datagram)
        );

        let found_connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostError::ConnectionNotFound(connection_id.0))?;

        let (datagram_without_layer, client_time) = found_connection.layer.receive(datagram)?;

        let deserialized_commands = datagram_chunker::deserialize_datagram::<
            ClientToHostCommands<StepT>,
        >(datagram_without_layer)?;

        let mut all_commands_to_send = Vec::new();
        for deserialized_command in deserialized_commands {
            let commands_to_send =
                self.logic
                    .update(connection_id, now, &deserialized_command, state_provider)?;

            all_commands_to_send.extend(commands_to_send);
        }

        self.logic.post_update();

        let mut datagram_chunker = DatagramChunker::new(1024);
        for cmd in all_commands_to_send {
            let mut out_stream = OutOctetStream::new();
            cmd.serialize(&mut out_stream)?;
            datagram_chunker.push(out_stream.octets_ref())?;
        }

        let outgoing_datagrams = datagram_chunker.finalize();

        let out_datagrams = found_connection
            .layer
            .send(client_time, outgoing_datagrams)?;

        for (index, datagram) in out_datagrams.iter().enumerate() {
            trace!(
                "host sending index {} payload:\n{}",
                index,
                format_hex(datagram)
            );
        }

        Ok(out_datagrams)
    }

    pub fn get(
        &self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Option<&HostConnection> {
        self.connections.get(&connection_id.0)
    }

    pub fn create_connection(&mut self) -> Option<nimble_host_logic::HostConnectionId> {
        if let Some(connection_id) = self.logic.create_connection() {
            self.connections
                .insert(connection_id.0, HostConnection::new());
            debug!("Created connection {:?}", connection_id);
            Some(connection_id)
        } else {
            None
        }
    }

    pub fn destroy_connection(
        &mut self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Result<(), HostError> {
        debug!("destroying connection {:?}", connection_id);
        self.connections.remove(&connection_id.0);
        self.logic
            .destroy_connection(connection_id)
            .expect("FIX THIS"); // TODO:
        Ok(())
    }
}
