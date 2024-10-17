/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::host_to_client::TickIdUtil;
use crate::serialize::CombinedSteps;
use crate::{ClientRequestId, SessionConnectionSecret, Version};
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use io::ErrorKind;
use nimble_blob_stream::prelude::ReceiverToSenderFrontCommands;
use nimble_participant::ParticipantId;
use std::fmt::{Debug, Display};
use std::{fmt, io};
use tick_id::TickId;

#[repr(u8)]
enum ClientToHostCommand {
    JoinGame = 0x01,
    Steps = 0x02,
    DownloadGameState = 0x03,
    BlobStreamChannel = 0x04,
    Connect = 0x05,
    Ping = 0x06,
}

impl TryFrom<u8> for ClientToHostCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        Ok(match value {
            0x01 => Self::JoinGame,
            0x02 => Self::Steps,
            0x03 => Self::DownloadGameState,
            0x04 => Self::BlobStreamChannel,
            0x05 => Self::Connect,
            0x06 => Self::Ping,
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown ClientToHostCommand {}", value),
            ))?,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ConnectRequest {
    pub nimble_version: Version,
    pub use_debug_stream: bool,
    pub application_version: Version,
    pub client_request_id: ClientRequestId,
}
impl ConnectRequest {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.nimble_version.to_stream(stream)?;
        stream.write_u8(if self.use_debug_stream { 0x01 } else { 0x00 })?;
        self.application_version.to_stream(stream)?;
        self.client_request_id.serialize(stream)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            nimble_version: Version::from_stream(stream)?,
            use_debug_stream: stream.read_u8()? != 0,
            application_version: Version::from_stream(stream)?,
            client_request_id: ClientRequestId::deserialize(stream)?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadGameStateRequest {
    pub request_id: u8,
}

impl DownloadGameStateRequest {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.request_id)
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            request_id: stream.read_u8()?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ClientToHostCommands<StepT: Clone + Debug + Serialize + Deserialize + Display> {
    JoinGameType(JoinGameRequest),
    Steps(StepsRequest<StepT>),
    DownloadGameState(DownloadGameStateRequest),
    BlobStreamChannel(ReceiverToSenderFrontCommands),
    ConnectType(ConnectRequest),
    Ping(u16),
}

impl<StepT: Clone + Debug + Serialize + Deserialize + Display> Serialize
    for ClientToHostCommands<StepT>
{
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.into())?;
        match self {
            Self::Steps(predicted_steps_and_ack) => predicted_steps_and_ack.to_stream(stream),
            Self::JoinGameType(join_game_request) => join_game_request.to_stream(stream),
            Self::DownloadGameState(download_game_state) => download_game_state.to_stream(stream),
            Self::BlobStreamChannel(blob_stream_command) => blob_stream_command.to_stream(stream),
            Self::ConnectType(connect_request) => connect_request.to_stream(stream),
            Self::Ping(ping_time) => stream.write_u16(*ping_time),
        }
    }
}

impl<StepT: Clone + Debug + Serialize + Deserialize + std::fmt::Display> Deserialize
    for ClientToHostCommands<StepT>
{
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let command_value = stream.read_u8()?;
        let command = ClientToHostCommand::try_from(command_value)?;
        let x = match command {
            ClientToHostCommand::JoinGame => {
                Self::JoinGameType(JoinGameRequest::from_stream(stream)?)
            }
            ClientToHostCommand::Steps => Self::Steps(StepsRequest::from_stream(stream)?),
            ClientToHostCommand::DownloadGameState => {
                Self::DownloadGameState(DownloadGameStateRequest::from_stream(stream)?)
            }
            ClientToHostCommand::BlobStreamChannel => {
                Self::BlobStreamChannel(ReceiverToSenderFrontCommands::from_stream(stream)?)
            }
            ClientToHostCommand::Connect => Self::ConnectType(ConnectRequest::from_stream(stream)?),
            ClientToHostCommand::Ping => Self::Ping(stream.read_u16()?),
        };
        Ok(x)
    }
}

impl<StepT: Deserialize + Serialize + Debug + Display + Clone> From<&ClientToHostCommands<StepT>>
    for u8
{
    fn from(command: &ClientToHostCommands<StepT>) -> Self {
        match command {
            ClientToHostCommands::Steps(_) => ClientToHostCommand::Steps as u8,
            ClientToHostCommands::JoinGameType(_) => ClientToHostCommand::JoinGame as u8,
            ClientToHostCommands::DownloadGameState(_) => {
                ClientToHostCommand::DownloadGameState as u8
            }
            ClientToHostCommands::BlobStreamChannel(_) => {
                ClientToHostCommand::BlobStreamChannel as u8
            }
            ClientToHostCommands::ConnectType(_) => ClientToHostCommand::Connect as u8,
            ClientToHostCommands::Ping(_) => ClientToHostCommand::Ping as u8,
        }
    }
}

impl<StepT: Clone + Debug + Eq + PartialEq + Serialize + Deserialize + Display> Display
    for ClientToHostCommands<StepT>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JoinGameType(join) => write!(f, "join {:?}", join),
            Self::Steps(predicted_steps_and_ack) => {
                write!(f, "steps {predicted_steps_and_ack}")
            }
            Self::DownloadGameState(download_game_state) => {
                write!(f, "download game state {:?}", download_game_state)
            }
            Self::BlobStreamChannel(blob_command) => {
                write!(f, "blob stream channel {:?}", blob_command)
            }
            &Self::ConnectType(connect_request) => write!(f, "connect {:?}", connect_request),
            Self::Ping(_) => write!(f, "ping"),
        }
    }
}

// --- Individual commands ---

#[repr(u8)]
pub enum JoinGameTypeValue {
    NoSecret,
    SessionSecret,
    HostMigrationParticipantId,
}

impl JoinGameTypeValue {
    pub fn to_octet(&self) -> u8 {
        match self {
            Self::NoSecret => Self::NoSecret as u8,
            Self::SessionSecret => Self::SessionSecret as u8,
            Self::HostMigrationParticipantId => Self::HostMigrationParticipantId as u8,
        }
    }
    pub fn to_stream(self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let join_game_type_value_raw = stream.read_u8()?;
        JoinGameTypeValue::try_from(join_game_type_value_raw)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum JoinGameType {
    NoSecret,
    UseSessionSecret(SessionConnectionSecret),
    HostMigrationParticipantId(ParticipantId),
}

impl TryFrom<u8> for JoinGameTypeValue {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        Ok(match value {
            0x00 => Self::NoSecret,
            0x01 => Self::SessionSecret,
            0x02 => Self::HostMigrationParticipantId,
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown join game type {}", value),
            ))?,
        })
    }
}

impl JoinGameType {
    pub fn to_octet(&self) -> u8 {
        match self {
            Self::NoSecret => JoinGameTypeValue::NoSecret as u8,
            Self::UseSessionSecret(_) => JoinGameTypeValue::SessionSecret as u8,
            Self::HostMigrationParticipantId(_) => {
                JoinGameTypeValue::HostMigrationParticipantId as u8
            }
        }
    }

    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            Self::NoSecret => {}
            Self::UseSessionSecret(session_secret) => session_secret.to_stream(stream)?,
            Self::HostMigrationParticipantId(participant_id) => participant_id.serialize(stream)?,
        }
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let join_game_type_value_raw = stream.read_u8()?;
        let value = JoinGameTypeValue::try_from(join_game_type_value_raw)?;
        let join_game_type = match value {
            JoinGameTypeValue::NoSecret => Self::NoSecret,
            JoinGameTypeValue::SessionSecret => {
                Self::UseSessionSecret(SessionConnectionSecret::from_stream(stream)?)
            }
            JoinGameTypeValue::HostMigrationParticipantId => {
                Self::HostMigrationParticipantId(ParticipantId::deserialize(stream)?)
            }
        };
        Ok(join_game_type)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct JoinPlayerRequest {
    pub local_index: u8,
}

impl JoinPlayerRequest {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.local_index)
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            local_index: stream.read_u8()?,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct JoinPlayerRequests {
    pub players: Vec<JoinPlayerRequest>,
}

impl JoinPlayerRequests {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.players.len() as u8)?;
        for player in self.players.iter() {
            player.to_stream(stream)?;
        }
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let count = stream.read_u8()?;
        let mut vec = Vec::<JoinPlayerRequest>::with_capacity(count as usize);
        for _ in 0..count {
            vec.push(JoinPlayerRequest::from_stream(stream)?);
        }

        Ok(Self { players: vec })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct JoinGameRequest {
    pub client_request_id: ClientRequestId,
    pub join_game_type: JoinGameType,
    pub player_requests: JoinPlayerRequests,
}

impl JoinGameRequest {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.client_request_id.serialize(stream)?;
        self.join_game_type.to_stream(stream)?;
        // TODO: Add more for other join game types.
        self.player_requests.to_stream(stream)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            client_request_id: ClientRequestId::deserialize(stream)?,
            join_game_type: JoinGameType::from_stream(stream)?,
            player_requests: JoinPlayerRequests::from_stream(stream)?,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct StepsAck {
    pub waiting_for_tick_id: TickId,
}

impl Display for StepsAck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "waiting:{}", self.waiting_for_tick_id)
    }
}

impl StepsAck {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        TickIdUtil::to_stream(self.waiting_for_tick_id, stream)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            waiting_for_tick_id: TickIdUtil::from_stream(stream)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StepsRequest<StepT: Clone + Serialize + Deserialize + Debug + std::fmt::Display> {
    pub ack: StepsAck,
    pub combined_predicted_steps: CombinedSteps<StepT>,
}

impl<StepT: Clone + Serialize + Deserialize + Debug + std::fmt::Display> Display
    for StepsRequest<StepT>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "steps-request ack:{}, steps:{}",
            self.ack, self.combined_predicted_steps
        )
    }
}

impl<StepT: Clone + Serialize + Deserialize + Debug + std::fmt::Display> StepsRequest<StepT> {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.ack.to_stream(stream)?;

        self.combined_predicted_steps.serialize(stream)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            ack: StepsAck::from_stream(stream)?,
            combined_predicted_steps: CombinedSteps::deserialize(stream)?,
        })
    }
}
