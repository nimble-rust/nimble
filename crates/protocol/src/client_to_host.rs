/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::host_to_client::TickIdUtil;
use crate::serialize::CombinedSteps;
use crate::{ClientRequestId, SessionConnectionSecret};
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use io::ErrorKind;
use nimble_blob_stream::prelude::ReceiverToSenderFrontCommands;
use nimble_participant::ParticipantId;
use std::fmt::Debug;
use std::{fmt, io};
use tick_id::TickId;

#[repr(u8)]
enum ClientToHostCommand {
    JoinGame = 0x01,
    Steps = 0x02,
    DownloadGameState = 0x03,
    BlobStreamChannel = 0x04,
}

impl TryFrom<u8> for ClientToHostCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            0x01 => Ok(Self::JoinGame),
            0x02 => Ok(Self::Steps),
            0x03 => Ok(Self::DownloadGameState),
            0x04 => Ok(Self::BlobStreamChannel),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown ClientToHostCommand {}", value),
            )),
        }
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
pub enum ClientToHostCommands<StepT: Clone + Debug + Serialize + Deserialize> {
    JoinGameType(JoinGameRequest),
    Steps(StepsRequest<StepT>),
    DownloadGameState(DownloadGameStateRequest),
    BlobStreamChannel(ReceiverToSenderFrontCommands),
}

impl<StepT: Clone + Debug + Serialize + Deserialize> Serialize for ClientToHostCommands<StepT> {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            Self::Steps(predicted_steps_and_ack) => predicted_steps_and_ack.to_stream(stream),
            Self::JoinGameType(join_game_request) => join_game_request.to_stream(stream),
            Self::DownloadGameState(download_game_state) => download_game_state.to_stream(stream),
            Self::BlobStreamChannel(blob_stream_command) => blob_stream_command.to_stream(stream),
        }
    }
}

impl<StepT: Clone + Debug + Serialize + Deserialize> Deserialize for ClientToHostCommands<StepT> {
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
        };
        Ok(x)
    }
}

impl<StepT: Clone + Debug + Serialize + Deserialize> ClientToHostCommands<StepT> {
    pub fn to_octet(&self) -> u8 {
        match self {
            Self::Steps(_) => ClientToHostCommand::Steps as u8,
            Self::JoinGameType(_) => ClientToHostCommand::JoinGame as u8,
            Self::DownloadGameState(_) => ClientToHostCommand::DownloadGameState as u8,
            Self::BlobStreamChannel(_) => ClientToHostCommand::BlobStreamChannel as u8,
        }
    }
}

impl<StepT: Clone + Debug + Eq + PartialEq + Serialize + Deserialize> fmt::Display
    for ClientToHostCommands<StepT>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JoinGameType(join) => write!(f, "join {:?}", join),
            Self::Steps(predicted_steps_and_ack) => {
                write!(f, "steps {:?}", predicted_steps_and_ack)
            }
            Self::DownloadGameState(download_game_state) => {
                write!(f, "download game state {:?}", download_game_state)
            }
            Self::BlobStreamChannel(blob_command) => {
                write!(f, "blob stream channel {:?}", blob_command)
            }
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
        match value {
            0x00 => Ok(Self::NoSecret),
            0x01 => Ok(Self::SessionSecret),
            0x02 => Ok(Self::HostMigrationParticipantId),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown join game type {}", value),
            )),
        }
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
            Self::HostMigrationParticipantId(participant_id) => participant_id.to_stream(stream)?,
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
                Self::HostMigrationParticipantId(ParticipantId::from_stream(stream)?)
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
    pub lost_steps_mask_after_last_received: u64,
}

impl StepsAck {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        TickIdUtil::to_stream(self.waiting_for_tick_id, stream)?;
        stream.write_u64(self.lost_steps_mask_after_last_received)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            waiting_for_tick_id: TickIdUtil::from_stream(stream)?,
            lost_steps_mask_after_last_received: stream.read_u64()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StepsRequest<StepT: Clone + Serialize + Deserialize + Debug> {
    pub ack: StepsAck,
    pub combined_predicted_steps: CombinedSteps<StepT>,
}

impl<StepT: Clone + Serialize + Deserialize + Debug> StepsRequest<StepT> {
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
