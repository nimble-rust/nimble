/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::client_to_host::{
    SerializeAuthoritativeStepRangeForAllParticipants,
    SerializeAuthoritativeStepVectorForOneParticipants,
};
use crate::{ClientRequestId, ServerConnectionId, SessionConnectionSecret};
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use io::ErrorKind;
use log::trace;
use nimble_blob_stream::prelude::SenderToReceiverFrontCommands;
use nimble_participant::ParticipantId;
use nimble_step_types::AuthoritativeStep;
use seq_map::SeqMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::io;
use tick_id::TickId;
// #define NimbleSerializeCmdGameStepResponse (0x08)
// #define NimbleSerializeCmdJoinGameResponse (0x09)
// #define NimbleSerializeCmdGameStatePart (0x0a)
// #define NimbleSerializeCmdGameStateResponse (0x0b)
// #define NimbleSerializeCmdJoinGameOutOfParticipantSlotsResponse (0x0c)
// #define NimbleSerializeCmdConnectResponse (0x0d)

#[repr(u8)]
pub enum HostToClientCommand {
    GameStep = 0x08,
    JoinGame = 0x09,
    DownloadGameState = 0x0B,
    BlobStreamChannel = 0x0C,
}

impl TryFrom<u8> for HostToClientCommand {
    type Error = io::Error;

    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            0x09 => Ok(HostToClientCommand::JoinGame),
            0x08 => Ok(HostToClientCommand::GameStep),
            0x0B => Ok(HostToClientCommand::DownloadGameState),
            0x0C => Ok(HostToClientCommand::BlobStreamChannel),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                format!("Unknown host to client command {:X}", value),
            )),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct TickIdUtil;

impl TickIdUtil {
    pub fn to_stream(tick_id: TickId, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u32(tick_id.0)
    }

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

impl DownloadGameStateResponse {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.client_request)?;
        TickIdUtil::to_stream(self.tick_id, stream)?;
        stream.write_u16(self.blob_stream_channel)
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            client_request: stream.read_u8()?,
            tick_id: TickIdUtil::from_stream(stream)?,
            blob_stream_channel: stream.read_u16()?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct GameStatePart {
    pub blob_stream_command: SenderToReceiverFrontCommands,
}

#[derive(Debug)]
pub struct ConnectResponse {
    pub flags: u8,
    pub client_request_id: ClientRequestId,
    pub server_connection_id: ServerConnectionId,
}

impl ConnectResponse {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.flags)?;
        stream.write_u8(self.client_request_id.0)?;
        stream.write_u8(self.server_connection_id.0)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            flags: stream.read_u8()?,
            client_request_id: ClientRequestId(stream.read_u8()?),
            server_connection_id: ServerConnectionId(stream.read_u8()?),
        })
    }
}

#[derive(Debug)]
pub enum HostToClientCommands<StepT: Deserialize + Serialize + Debug + Clone> {
    JoinGame(JoinGameAccepted),
    GameStep(GameStepResponse<StepT>),
    DownloadGameState(DownloadGameStateResponse),
    BlobStreamChannel(SenderToReceiverFrontCommands),
}

impl<StepT: Deserialize + Serialize + Debug + Clone> HostToClientCommands<StepT> {
    pub fn to_octet(&self) -> u8 {
        match self {
            HostToClientCommands::JoinGame(_) => HostToClientCommand::JoinGame as u8,
            HostToClientCommands::GameStep(_) => HostToClientCommand::GameStep as u8,
            HostToClientCommands::DownloadGameState(_) => {
                HostToClientCommand::DownloadGameState as u8
            }
            HostToClientCommands::BlobStreamChannel(_) => {
                HostToClientCommand::BlobStreamChannel as u8
            }
        }
    }

    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.to_octet())?;
        match self {
            HostToClientCommands::JoinGame(join_game_response) => {
                join_game_response.to_stream(stream)
            }
            HostToClientCommands::GameStep(game_step_response) => {
                game_step_response.to_stream(stream)
            }
            HostToClientCommands::DownloadGameState(download_game_state_response) => {
                download_game_state_response.to_stream(stream)
            }
            HostToClientCommands::BlobStreamChannel(blob_stream_command) => {
                blob_stream_command.to_stream(stream)
            }
        }
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let command_value = stream.read_u8()?;
        let command = HostToClientCommand::try_from(command_value)?;
        let x = match command {
            HostToClientCommand::JoinGame => {
                HostToClientCommands::JoinGame(JoinGameAccepted::from_stream(stream)?)
            }
            HostToClientCommand::GameStep => {
                HostToClientCommands::GameStep(GameStepResponse::from_stream(stream)?)
            } /*
            => {
            return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("unknown command {}", command_value),
            ));
            }

            */
            HostToClientCommand::DownloadGameState => HostToClientCommands::DownloadGameState(
                DownloadGameStateResponse::from_stream(stream)?,
            ),
            HostToClientCommand::BlobStreamChannel => HostToClientCommands::BlobStreamChannel(
                SenderToReceiverFrontCommands::from_stream(stream)?,
            ),
        };
        Ok(x)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PartyAndSessionSecret {
    pub session_secret: SessionConnectionSecret,
    pub party_id: u8,
}

impl PartyAndSessionSecret {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.session_secret.to_stream(stream)?;
        stream.write_u8(self.party_id)
    }
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            session_secret: SessionConnectionSecret::from_stream(stream)?,
            party_id: stream.read_u8()?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct JoinGameParticipant {
    pub local_index: u8,
    pub participant_id: ParticipantId,
}

impl JoinGameParticipant {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.local_index)?;
        self.participant_id.to_stream(stream)?;
        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            local_index: stream.read_u8()?,
            participant_id: ParticipantId::from_stream(stream)?,
        })
    }
}

#[derive(Debug)]
pub struct JoinGameParticipants(pub Vec<JoinGameParticipant>);

impl JoinGameParticipants {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.0.len() as u8)?;
        for join_game_participant in &self.0 {
            join_game_participant.to_stream(stream)?
        }
        Ok(())
    }

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

impl JoinGameAccepted {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.client_request_id.serialize(stream)?;
        self.party_and_session_secret.to_stream(stream)?;
        self.participants.to_stream(stream)
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            client_request_id: ClientRequestId::deserialize(stream)?,
            party_and_session_secret: PartyAndSessionSecret::from_stream(stream)?,
            participants: JoinGameParticipants::from_stream(stream)?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct GameStepResponseHeader {
    pub connection_buffer_count: u8,
    pub delta_buffer: i8,
    pub last_step_received_from_client: u32,
}

impl GameStepResponseHeader {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.connection_buffer_count)?;
        stream.write_i8(self.delta_buffer)?;
        stream.write_u32(self.last_step_received_from_client)
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            connection_buffer_count: stream.read_u8()?,
            delta_buffer: stream.read_i8()?,
            last_step_received_from_client: stream.read_u32()?,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AuthoritativeStepRange<StepT: Deserialize + Serialize + Debug + Clone> {
    pub tick_id: TickId,
    pub authoritative_steps: Vec<AuthoritativeStep<StepT>>,
}

#[derive(Debug)]
pub struct SerializeAuthoritativeStepRange<StepT: Deserialize + Serialize + Debug + Clone> {
    pub delta_steps_from_previous: u8,
    pub authoritative_steps: SerializeAuthoritativeStepRangeForAllParticipants<StepT>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone> SerializeAuthoritativeStepRange<StepT> {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.delta_steps_from_previous)?;

        self.authoritative_steps.serialize_with_len(stream)?;

        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let delta_steps = stream.read_u8()?;

        let authoritative_combined_step =
            SerializeAuthoritativeStepRangeForAllParticipants::deserialize_with_len(stream)?;

        Ok(Self {
            delta_steps_from_previous: delta_steps,
            authoritative_steps: authoritative_combined_step,
        })
    }
}
#[derive(Debug)]
pub struct AuthoritativeStepRanges<StepT: Deserialize + Serialize + Debug + Clone> {
    pub ranges: Vec<AuthoritativeStepRange<StepT>>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone> Serialize for AuthoritativeStepRanges<StepT> {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        let mut converted_ranges = Vec::new();

        let root_tick_id = self.ranges[0].tick_id;
        let mut tick_id = root_tick_id;
        for auth_range in &self.ranges {
            if auth_range.tick_id < tick_id {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ranges are incorrect",
                ))?;
            }
            let delta_steps_from_previous = (auth_range.tick_id - tick_id) as u8;
            tick_id = auth_range.tick_id + auth_range.authoritative_steps.len() as u32;

            let mut hash_map = SeqMap::<
                ParticipantId,
                SerializeAuthoritativeStepVectorForOneParticipants<StepT>,
            >::new();

            let mut unique_participant_ids: HashSet<ParticipantId> = HashSet::new();

            for auth_step in &auth_range.authoritative_steps {
                for key in auth_step.authoritative_participants.keys() {
                    unique_participant_ids.insert(*key);
                }
            }

            let mut sorted_unique_ids: Vec<ParticipantId> =
                unique_participant_ids.into_iter().collect();
            sorted_unique_ids.sort();

            for participant_id in sorted_unique_ids {
                hash_map
                    .insert(
                        participant_id,
                        SerializeAuthoritativeStepVectorForOneParticipants::<StepT> {
                            delta_tick_id_from_range: 0,
                            steps: vec![],
                        },
                    )
                    .expect("participant ids to be unique");
            }

            for (index_in_range, combined_auth_step) in
                auth_range.authoritative_steps.iter().enumerate()
            {
                for (participant_id, auth_step_for_one_player) in
                    &combined_auth_step.authoritative_participants
                {
                    let vector_for_one_person = hash_map.get_mut(participant_id).unwrap();
                    if vector_for_one_person.steps.is_empty() {
                        vector_for_one_person.delta_tick_id_from_range = index_in_range as u8;
                    }
                    vector_for_one_person
                        .steps
                        .push(auth_step_for_one_player.clone())
                }
            }

            let range = SerializeAuthoritativeStepRange {
                delta_steps_from_previous,
                authoritative_steps: SerializeAuthoritativeStepRangeForAllParticipants::<StepT> {
                    authoritative_participants: hash_map,
                },
            };
            converted_ranges.push(range);
        }

        let all_ranges = SerializeAuthoritativeStepRanges {
            root_tick_id,
            ranges: converted_ranges,
        };

        all_ranges.to_stream(stream)
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone> Deserialize
    for AuthoritativeStepRanges<StepT>
{
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        let source_step_ranges = SerializeAuthoritativeStepRanges::<StepT>::from_stream(stream)?;
        let mut tick_id = source_step_ranges.root_tick_id;

        let mut converted_ranges = Vec::new();
        for serialized_step_range in &source_step_ranges.ranges {
            tick_id += serialized_step_range.delta_steps_from_previous as u32;

            let mut max_vector_length = 0;

            for serialized_step_vector in serialized_step_range
                .authoritative_steps
                .authoritative_participants
                .values()
            {
                if serialized_step_vector.steps.len() > max_vector_length {
                    max_vector_length = serialized_step_vector.steps.len();
                }
            }

            let mut auth_step_range_vec = Vec::<AuthoritativeStep<StepT>>::new();
            for _ in 0..max_vector_length {
                auth_step_range_vec.push(AuthoritativeStep::<StepT> {
                    authoritative_participants: SeqMap::new(),
                })
            }

            for (participant_id, serialized_step_vector) in &serialized_step_range
                .authoritative_steps
                .authoritative_participants
            {
                for (index, serialized_step) in serialized_step_vector.steps.iter().enumerate() {
                    let hash_map_for_auth_step = &mut auth_step_range_vec
                        .get_mut(index)
                        .unwrap()
                        .authoritative_participants;
                    hash_map_for_auth_step
                        .insert(*participant_id, serialized_step.clone())
                        .expect("expect unique participant_id");
                }
            }

            let range = AuthoritativeStepRange::<StepT> {
                tick_id,
                authoritative_steps: auth_step_range_vec,
            };

            converted_ranges.push(range);
        }

        Ok(AuthoritativeStepRanges {
            ranges: converted_ranges,
        })
    }
}

#[derive(Debug)]
pub struct SerializeAuthoritativeStepRanges<StepT: Deserialize + Serialize + Debug + Clone> {
    pub root_tick_id: TickId,
    pub ranges: Vec<SerializeAuthoritativeStepRange<StepT>>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone> SerializeAuthoritativeStepRanges<StepT> {
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

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let root_tick_id = TickIdUtil::from_stream(stream)?;
        let range_count = stream.read_u8()?;

        trace!("root_tick_id {root_tick_id} range_count {range_count}");
        let mut authoritative_step_ranges =
            Vec::<SerializeAuthoritativeStepRange<StepT>>::with_capacity(range_count as usize);

        for _ in 0..range_count {
            authoritative_step_ranges.push(SerializeAuthoritativeStepRange::from_stream(stream)?);
        }

        Ok(Self {
            root_tick_id,
            ranges: authoritative_step_ranges,
        })
    }
}

#[derive(Debug)]
pub struct GameStepResponse<StepT: Serialize + Deserialize + Debug + Clone> {
    pub response_header: GameStepResponseHeader,
    pub authoritative_steps: AuthoritativeStepRanges<StepT>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone> GameStepResponse<StepT> {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        self.response_header.to_stream(stream)?;
        self.authoritative_steps.serialize(stream)
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {
            response_header: GameStepResponseHeader::from_stream(stream)?,
            authoritative_steps: AuthoritativeStepRanges::deserialize(stream)?,
        })
    }
}
