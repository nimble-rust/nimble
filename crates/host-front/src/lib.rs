use flood_rs::in_stream::InOctetStream;
use flood_rs::prelude::OutOctetStream;
use flood_rs::{Deserialize, Serialize, WriteOctetStream};
use monotonic_time_rs::Millis;
use nimble_host_logic::logic::{GameSession, GameStateProvider};
use nimble_host_stream::{HostStream, HostStreamConnection, HostStreamError};
use nimble_ordered_datagram::{DatagramOrderInError, OrderedIn, OrderedOut};
use nimble_protocol_header::ClientTime;
use std::collections::HashMap;
use std::fmt::Debug;
use tick_id::TickId;

pub struct HostFrontConnection {
    ordered_datagram_out: OrderedOut,
    ordered_in: OrderedIn,
}
impl HostFrontConnection {
    pub fn new() -> Self {
        Self {
            ordered_in: OrderedIn::default(),
            ordered_datagram_out: OrderedOut::default(),
        }
    }
}

#[derive(Debug)]
pub enum HostFrontError {
    DatagramOrderInError(DatagramOrderInError),
    HostStreamError(HostStreamError),
    IoError(std::io::Error),
}

impl From<DatagramOrderInError> for HostFrontError {
    fn from(err: DatagramOrderInError) -> Self {
        Self::DatagramOrderInError(err)
    }
}

impl From<HostStreamError> for HostFrontError {
    fn from(err: HostStreamError) -> Self {
        Self::HostStreamError(err)
    }
}

impl From<std::io::Error> for HostFrontError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub struct ConnectionId(u8);

impl ConnectionId {
    pub fn inner(&self) -> u8 {
        self.0
    }
}

pub struct HostFront<
    StepT: Clone + std::fmt::Debug + std::cmp::Eq + flood_rs::Deserialize + flood_rs::Serialize,
> {
    host_stream: HostStream<StepT>,
    connections: HashMap<u8, HostFrontConnection>,
}

impl<StepT: Clone + Deserialize + Serialize + Eq + Debug> HostFront<StepT> {
    pub fn new(app_version: &app_version::Version, tick_id: TickId) -> Self {
        Self {
            host_stream: HostStream::<StepT>::new(app_version, tick_id),
            connections: HashMap::new(),
        }
    }

    pub fn session(&self) -> &GameSession {
        self.host_stream.session()
    }

    pub fn get(
        &self,
        connection_id: nimble_host_logic::logic::HostConnectionId,
    ) -> Option<&HostFrontConnection> {
        self.connections.get(&connection_id.0)
    }

    pub fn get_stream(
        &self,
        connection_id: nimble_host_logic::logic::HostConnectionId,
    ) -> Option<&HostStreamConnection> {
        self.host_stream.get(connection_id)
    }

    pub fn update(
        &mut self,
        connection_id: nimble_host_logic::logic::HostConnectionId,
        now: Millis,
        datagram: &[u8],
        state_provider: &impl GameStateProvider,
    ) -> Result<Vec<Vec<u8>>, HostFrontError> {
        let mut in_stream = InOctetStream::new(datagram);
        let found_connection = self
            .connections
            .get_mut(&connection_id.0)
            .ok_or(HostStreamError::ConnectionNotFound)?;

        found_connection
            .ordered_in
            .read_and_verify(&mut in_stream)?;

        let client_time = ClientTime::deserialize(&mut in_stream)?;

        let datagrams_to_send = self.host_stream.update(
            connection_id,
            now,
            &datagram[in_stream.cursor.position() as usize..],
            state_provider,
        )?;

        let mut out_datagrams: Vec<Vec<u8>> = Vec::new();

        for datagram in datagrams_to_send {
            let mut out_stream = OutOctetStream::new();

            found_connection
                .ordered_datagram_out
                .to_stream(&mut out_stream)?;
            client_time.serialize(&mut out_stream)?;
            out_stream.write(datagram.as_slice())?;

            out_datagrams.push(out_stream.octets_ref().to_vec());
        }

        Ok(out_datagrams)
    }

    pub fn create_connection(&mut self) -> Option<nimble_host_logic::logic::HostConnectionId> {
        if let Some(connection_id) = self.host_stream.create_connection() {
            self.connections
                .insert(connection_id.0, HostFrontConnection::new());
            Some(connection_id)
        } else {
            None
        }
    }
}
