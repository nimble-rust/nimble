/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*!
# Nimble Host Crate

The `nimble-host` crate provides the core functionality for managing game sessions and connections in the Nimble multiplayer framework. It handles host logic, connection management, and communication between clients and the host.

## Features

- **Connection Management**: Create, manage, and destroy client connections.
- **Host Logic**: Integrates with `nimble_host_logic` to manage game state and handle client commands.
- **Datagram Handling**: Efficiently processes incoming and outgoing datagrams with chunking support.
- **Serialization**: Supports serialization and deserialization of commands using `flood_rs`.

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
use nimble_host_logic::{
    connection::Connection, session::GameSession, GameStateProvider, HostLogic,
};
use nimble_layer::NimbleLayer;
use nimble_protocol::prelude::ClientToHostCommands;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use tick_id::TickId;

/// Represents a connection managed by the host.
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

/// Unique identifier for a host connection.
pub struct ConnectionId(u8);

impl ConnectionId {
    /// Retrieves the inner u8 value of the ConnectionId.
    pub fn inner(&self) -> u8 {
        self.0
    }
}

/// The main host structure managing game logic and client connections.
///
/// Host handles the game session, processes client commands, and manages the state of each connection.
///
/// # Type Parameters
///
/// - StepT: The type representing a step in the game logic. Must implement Clone, Debug, Eq, Deserialize, Serialize, and Display.
pub struct Host<StepT: Clone + Debug + Eq + Deserialize + Serialize + Display> {
    logic: HostLogic<StepT>,
    connections: HashMap<u8, HostConnection>,
}

impl<StepT: Clone + Deserialize + Serialize + Eq + Debug + Display> Host<StepT> {
    /// Creates a new Host instance with the specified application version and initial tick ID.
    ///
    /// # Arguments
    ///
    /// * app_version - The version of the application.
    /// * tick_id - The initial tick identifier.
    pub fn new(app_version: app_version::Version, tick_id: TickId) -> Self {
        Self {
            logic: HostLogic::<StepT>::new(tick_id, app_version),
            connections: HashMap::new(),
        }
    }

    /// Returns a reference to the internal `HostLogic` for debugging purposes.
    pub fn debug_logic(&self) -> &HostLogic<StepT> {
        &self.logic
    }

    /// Retrieves a specific connection's logic by its connection ID for debugging purposes.
    pub fn debug_get_logic(
        &self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Option<&Connection<StepT>> {
        self.logic.get(connection_id)
    }

    /// Returns a reference to the current game session.
    pub fn session(&self) -> &GameSession<StepT> {
        self.logic.session()
    }

    /// Updates the host state based on incoming datagrams from a client.
    ///
    /// Processes the datagram, updates game logic, and prepares outgoing datagrams to be sent back to the client.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The ID of the connection sending the datagram.
    /// * `now` - The current time in milliseconds.
    /// * `datagram` - The incoming datagram data.
    /// * `state_provider` - A reference to an implementation providing game state if needed.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of outgoing datagrams or a `HostError` on failure.
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

        let datagram_without_layer = found_connection.layer.receive(datagram)?;

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

        let out_datagrams = found_connection.layer.send(outgoing_datagrams)?;

        for (index, datagram) in out_datagrams.iter().enumerate() {
            trace!(
                "host sending index {} payload:\n{}",
                index,
                format_hex(datagram)
            );
        }

        Ok(out_datagrams)
    }

    /// Retrieves a reference to a `HostConnection` by its connection ID.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The ID of the connection to retrieve.
    pub fn get(
        &self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Option<&HostConnection> {
        self.connections.get(&connection_id.0)
    }

    /// Creates a new connection and adds it to the host.
    ///
    /// # Returns
    ///
    /// An `Option` containing the new `HostConnectionId` if successful, or `None` if the connection could not be created.
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

    /// Destroys an existing connection and removes it from the host.
    ///
    /// # Arguments
    ///
    /// * `connection_id` - The ID of the connection to destroy.
    ///
    /// # Errors
    ///
    /// Returns a `HostError` if the connection could not be found or destroyed.
    pub fn destroy_connection(
        &mut self,
        connection_id: nimble_host_logic::HostConnectionId,
    ) -> Result<(), HostError> {
        debug!("destroying connection {:?}", connection_id);
        self.connections.remove(&connection_id.0);
        self.logic.destroy_connection(connection_id)?;
        Ok(())
    }
}
