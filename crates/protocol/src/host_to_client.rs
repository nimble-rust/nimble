/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use crate::serialize::{
    CombinedSteps, InternalAllParticipantVectors, InternalAuthoritativeStepRange,
};
use crate::{ClientRequestId, SessionConnectionSecret};
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use log::trace;
use nimble_blob_stream::prelude::SenderToReceiverFrontCommands;
use nimble_participant::ParticipantId;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use tick_id::TickId;

#[repr(u8)]
#[allow(clippy::module_name_repetitions)] // TODO: Rename module or enum
pub enum HostToClientCommand {
    GameStep = 0x08,
    JoinGame = 0x09,
    DownloadGameState = 0x0B,
    BlobStreamChannel = 0x0C,
    Connect = 0x0D,
    Pong = 0x0E,
}

impl TryFrom<u8> for HostToClientCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        Ok(match value {
            0x09 => Self::JoinGame,
            0x08 => Self::GameStep,
            0x0B => Self::DownloadGameState,
            0x0C => Self::BlobStreamChannel,
            0x0D => Self::Connect,
            0x0E => Self::Pong,
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown host to client command 0x{value:0X}"),
            ))?,
        })
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TickIdUtil;

impl TickIdUtil {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(tick_id: TickId, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u32(tick_id.0)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<TickId> {
        Ok(TickId(stream.read_u32()?))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DownloadGameStateResponse {
    pub client_request: u8,
    pub tick_id: TickId,
    pub blob_stream_channel: u16,
}

impl Display for DownloadGameStateResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "download game state response {} {} {}",
            self.client_request, self.tick_id, self.blob_stream_channel
        )
    }
}

impl DownloadGameStateResponse {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.client_request)?;
        TickIdUtil::to_stream(self.tick_id, stream)?;
        stream.write_u16(self.blob_stream_channel)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            client_request: stream.read_u8()?,
            tick_id: TickIdUtil::from_stream(stream)?,
            blob_stream_channel: stream.read_u16()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct GameStatePart {
    pub blob_stream_command: SenderToReceiverFrontCommands,
}

#[derive(Debug)]
pub struct ConnectResponse {
    pub flags: u8,
    pub client_request_id: ClientRequestId,
}

impl ConnectResponse {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.flags)?;
        stream.write_u8(self.client_request_id.0)?;
        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            flags: stream.read_u8()?,
            client_request_id: ClientRequestId(stream.read_u8()?),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConnectionAccepted {
    pub flags: u8,
    pub response_to_request: ClientRequestId,
}

impl Display for ConnectionAccepted {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "connection accepted {} {}",
            self.flags, self.response_to_request
        )
    }
}

impl ConnectionAccepted {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.flags)?;
        self.response_to_request.serialize(stream)?;
        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            flags: stream.read_u8()?,
            response_to_request: ClientRequestId::deserialize(stream)?,
        })
    }
}

#[derive(Debug)]
pub struct PongInfo {
    pub lower_millis: u16,
}

impl Serialize for PongInfo {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u16(self.lower_millis)
    }
}

impl Deserialize for PongInfo {
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            lower_millis: stream.read_u16()?,
        })
    }
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)] // TODO: Rename module or enum
pub enum HostToClientCommands<StepT: Deserialize + Serialize + Debug + Clone + Display> {
    JoinGame(JoinGameAccepted),
    GameStep(GameStepResponse<StepT>),
    DownloadGameState(DownloadGameStateResponse),
    BlobStreamChannel(SenderToReceiverFrontCommands),
    ConnectType(ConnectionAccepted),
    Pong(PongInfo),
}

impl<StepT: Clone + Debug + Serialize + Deserialize + Display> Serialize
    for HostToClientCommands<StepT>
{
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.into())?;
        match self {
            Self::JoinGame(join_game_response) => join_game_response.to_stream(stream),
            Self::GameStep(game_step_response) => game_step_response.to_stream(stream),
            Self::DownloadGameState(download_game_state_response) => {
                download_game_state_response.to_stream(stream)
            }
            Self::BlobStreamChannel(blob_stream_command) => blob_stream_command.to_stream(stream),
            Self::ConnectType(connect_response) => connect_response.to_stream(stream),
            Self::Pong(pong_info) => pong_info.serialize(stream),
        }
    }
}

impl<StepT: Clone + Debug + Serialize + Deserialize + Display> Display
    for HostToClientCommands<StepT>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JoinGame(join_game_response) => {
                write!(f, "JoinGameResponse({join_game_response})")
            }
            Self::GameStep(game_step_response) => {
                write!(f, "GameStepResponse({game_step_response})")
            }
            Self::DownloadGameState(download_game_state_response) => {
                write!(f, "DownloadGameState({download_game_state_response})")
            }
            Self::BlobStreamChannel(blob_stream_command) => {
                write!(f, "BlobStreamChannel({blob_stream_command})")
            }
            Self::ConnectType(connect_response) => {
                write!(f, "ConnectResponse({connect_response})")
            }
            Self::Pong(pong_info) => write!(f, "Pong({pong_info:?})"),
        }
    }
}

impl<StepT: Clone + Debug + Serialize + Deserialize + Display> Deserialize
    for HostToClientCommands<StepT>
{
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let command_value = stream.read_u8()?;
        let command = HostToClientCommand::try_from(command_value)?;
        Ok(match command {
            HostToClientCommand::JoinGame => Self::JoinGame(JoinGameAccepted::from_stream(stream)?),
            HostToClientCommand::GameStep => Self::GameStep(GameStepResponse::from_stream(stream)?),
            HostToClientCommand::DownloadGameState => {
                Self::DownloadGameState(DownloadGameStateResponse::from_stream(stream)?)
            }
            HostToClientCommand::BlobStreamChannel => {
                Self::BlobStreamChannel(SenderToReceiverFrontCommands::from_stream(stream)?)
            }
            HostToClientCommand::Connect => {
                Self::ConnectType(ConnectionAccepted::from_stream(stream)?)
            }
            HostToClientCommand::Pong => Self::Pong(PongInfo::deserialize(stream)?),
        })
    }
}

impl<StepT: Deserialize + Serialize + Debug + Display + Clone> From<&HostToClientCommands<StepT>>
    for u8
{
    fn from(command: &HostToClientCommands<StepT>) -> Self {
        match command {
            HostToClientCommands::JoinGame(_) => HostToClientCommand::JoinGame as Self,
            HostToClientCommands::GameStep(_) => HostToClientCommand::GameStep as Self,
            HostToClientCommands::DownloadGameState(_) => {
                HostToClientCommand::DownloadGameState as Self
            }
            HostToClientCommands::BlobStreamChannel(_) => {
                HostToClientCommand::BlobStreamChannel as Self
            }
            HostToClientCommands::ConnectType(_) => HostToClientCommand::Connect as Self,
            HostToClientCommands::Pong(_) => HostToClientCommand::Pong as Self,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PartyAndSessionSecret {
    pub session_secret: SessionConnectionSecret,
    pub party_id: u8,
}

impl PartyAndSessionSecret {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.session_secret.to_stream(stream)?;
        stream.write_u8(self.party_id)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            session_secret: SessionConnectionSecret::from_stream(stream)?,
            party_id: stream.read_u8()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct JoinGameParticipant {
    pub local_index: u8,
    pub participant_id: ParticipantId,
}

impl JoinGameParticipant {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.local_index)?;
        self.participant_id.serialize(stream)?;
        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            local_index: stream.read_u8()?,
            participant_id: ParticipantId::deserialize(stream)?,
        })
    }
}

#[derive(Debug)]
pub struct JoinGameParticipants(pub Vec<JoinGameParticipant>);

impl JoinGameParticipants {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.0.len() as u8)?;
        for join_game_participant in &self.0 {
            join_game_participant.to_stream(stream)?;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let count = stream.read_u8()?;
        let mut vec = Vec::<JoinGameParticipant>::with_capacity(count as usize);
        for _ in 0..count {
            vec.push(JoinGameParticipant::from_stream(stream)?);
        }

        Ok(Self(vec))
    }
}

#[derive(Debug)]
pub struct JoinGameAccepted {
    pub client_request_id: ClientRequestId,
    pub party_and_session_secret: PartyAndSessionSecret,
    pub participants: JoinGameParticipants,
}

impl Display for JoinGameAccepted {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JoinGameAccepted {} {:?} {:?}",
            self.client_request_id, self.party_and_session_secret, self.participants
        )
    }
}

impl JoinGameAccepted {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.client_request_id.serialize(stream)?;
        self.party_and_session_secret.to_stream(stream)?;
        self.participants.to_stream(stream)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            client_request_id: ClientRequestId::deserialize(stream)?,
            party_and_session_secret: PartyAndSessionSecret::from_stream(stream)?,
            participants: JoinGameParticipants::from_stream(stream)?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct GameStepResponseHeader {
    pub connection_buffer_count: u8,
    pub delta_buffer: i8,
    pub next_expected_tick_id: TickId,
}

impl Display for GameStepResponseHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "game_step_response: count:{} expected:{} delta-buf:{}",
            self.connection_buffer_count, self.next_expected_tick_id, self.delta_buffer
        )
    }
}

impl GameStepResponseHeader {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.connection_buffer_count)?;
        stream.write_i8(self.delta_buffer)?;
        TickIdUtil::to_stream(self.next_expected_tick_id, stream)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            connection_buffer_count: stream.read_u8()?,
            delta_buffer: stream.read_i8()?,
            next_expected_tick_id: TickIdUtil::from_stream(stream)?,
        })
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + Display>
    InternalAuthoritativeStepRange<StepT>
{
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.delta_tick_id_from_previous)?;

        self.authoritative_steps.serialize(stream)?;

        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let delta_steps = stream.read_u8()?;

        let authoritative_combined_step = InternalAllParticipantVectors::deserialize(stream)?;

        Ok(Self {
            delta_tick_id_from_previous: delta_steps,
            authoritative_steps: authoritative_combined_step,
        })
    }
}

#[derive(Debug)]
pub struct AuthoritativeStepRanges<StepT: Deserialize + Serialize + Debug + Clone + Display> {
    pub ranges: Vec<CombinedSteps<StepT>>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone + Display> Display
    for AuthoritativeStepRanges<StepT>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "auth_steps range-count:{} ranges:", self.ranges.len())?;

        for range in &self.ranges {
            write!(f, "\n{range}")?;
        }

        Ok(())
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + Display> Serialize
    for AuthoritativeStepRanges<StepT>
{
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        let mut converted_ranges = Vec::new();

        let root_tick_id = if self.ranges.is_empty() {
            TickId(0)
        } else {
            self.ranges[0].tick_id
        };
        let mut tick_id = root_tick_id;
        for auth_range in &self.ranges {
            let delta_ticks_from_previous = u8::try_from(auth_range.tick_id - tick_id)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ranges are incorrect"))?;
            tick_id = auth_range.tick_id + auth_range.steps.len() as u32;

            let internal = auth_range.to_internal();

            let range = InternalAuthoritativeStepRange {
                delta_tick_id_from_previous: delta_ticks_from_previous,
                authoritative_steps: internal,
            };
            converted_ranges.push(range);
        }

        let all_ranges = InternalAuthoritativeStepRanges {
            root_tick_id,
            ranges: converted_ranges,
        };

        all_ranges.to_stream(stream)
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + Display> Deserialize
    for AuthoritativeStepRanges<StepT>
{
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        let internal_auth_step_ranges =
            InternalAuthoritativeStepRanges::<StepT>::from_stream(stream)?;
        let mut tick_id = internal_auth_step_ranges.root_tick_id;

        let mut converted_ranges = Vec::new();
        for internal_step_range in &internal_auth_step_ranges.ranges {
            tick_id += internal_step_range.delta_tick_id_from_previous as u32;

            let combined_steps =
                CombinedSteps::from_internal(&internal_step_range.authoritative_steps, tick_id);

            converted_ranges.push(combined_steps);
        }

        Ok(Self {
            ranges: converted_ranges,
        })
    }
}

#[derive(Debug)]
pub struct InternalAuthoritativeStepRanges<StepT: Deserialize + Serialize + Debug + Clone + Display>
{
    pub root_tick_id: TickId,
    pub ranges: Vec<InternalAuthoritativeStepRange<StepT>>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone + Display>
    InternalAuthoritativeStepRanges<StepT>
{
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        TickIdUtil::to_stream(self.root_tick_id, stream)?;
        stream.write_u8(self.ranges.len() as u8)?;
        trace!(
            "tick_id: {} range_count: {}",
            self.root_tick_id,
            self.ranges.len()
        );
        for range in &self.ranges {
            range.to_stream(stream)?;
        }
        Ok(())
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let root_tick_id = TickIdUtil::from_stream(stream)?;
        let range_count = stream.read_u8()?;

        let mut authoritative_step_ranges =
            Vec::<InternalAuthoritativeStepRange<StepT>>::with_capacity(range_count as usize);

        for _ in 0..range_count {
            authoritative_step_ranges.push(InternalAuthoritativeStepRange::from_stream(stream)?);
        }

        Ok(Self {
            root_tick_id,
            ranges: authoritative_step_ranges,
        })
    }
}

#[derive(Debug)]
pub struct GameStepResponse<StepT: Serialize + Deserialize + Debug + Clone + Display> {
    pub response_header: GameStepResponseHeader,
    pub authoritative_steps: AuthoritativeStepRanges<StepT>,
}

impl<StepT: Serialize + Deserialize + Debug + Clone + Display> Display for GameStepResponse<StepT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "response: {} auth-steps: {}",
            self.response_header, self.authoritative_steps
        )
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + Display> GameStepResponse<StepT> {
    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.response_header.to_stream(stream)?;
        self.authoritative_steps.serialize(stream)
    }

    /// # Errors
    ///
    /// `io::Error` // TODO:
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            response_header: GameStepResponseHeader::from_stream(stream)?,
            authoritative_steps: AuthoritativeStepRanges::deserialize(stream)?,
        })
    }
}
