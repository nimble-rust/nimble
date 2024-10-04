/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use crate::host_to_client::TickIdUtil;
use crate::{ClientRequestId, SessionConnectionSecret};
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use io::ErrorKind;
use nimble_blob_stream::prelude::ReceiverToSenderFrontCommands;
use nimble_participant::ParticipantId;
use nimble_step_types::{LocalIndex, PredictedStep};
use seq_map::SeqMap;
use std::collections::HashSet;
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

#[derive(Debug, PartialEq, Clone)]
pub struct SerializeAuthoritativeStepVectorForOneParticipants<StepT: Serialize + Deserialize> {
    pub delta_tick_id_from_range: u8,
    pub steps: Vec<StepT>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct SerializeAuthoritativeStepRangeForAllParticipants<StepT: Serialize + Deserialize> {
    pub authoritative_participants:
        SeqMap<ParticipantId, SerializeAuthoritativeStepVectorForOneParticipants<StepT>>,
}

impl<StepT: Serialize + Deserialize + std::fmt::Debug>
    SerializeAuthoritativeStepRangeForAllParticipants<StepT>
{
    pub fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        // How many participants streams follows
        stream.write_u8(self.authoritative_participants.len() as u8)?;

        for (participant_id, authoritative_steps_for_one_player_vector) in
            &self.authoritative_participants
        {
            participant_id.to_stream(stream)?;
            stream.write_u8(authoritative_steps_for_one_player_vector.delta_tick_id_from_range)?;
            stream.write_u8(authoritative_steps_for_one_player_vector.steps.len() as u8)?;

            for authoritative_step_for_one_player in
                &authoritative_steps_for_one_player_vector.steps
            {
                authoritative_step_for_one_player.serialize(stream)?;
            }
        }
        Ok(())
    }

    pub fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let required_participant_count_in_range = stream.read_u8()?;
        let mut authoritative_participants = SeqMap::new();
        for _ in 0..required_participant_count_in_range {
            let participant_id = ParticipantId::from_stream(stream)?;
            let delta_tick_id_from_range = stream.read_u8()?;
            let number_of_steps_that_follows = stream.read_u8()? as usize;

            let mut authoritative_steps_for_one_participant =
                Vec::with_capacity(number_of_steps_that_follows);

            for _ in 0..number_of_steps_that_follows {
                let authoritative_step = StepT::deserialize(stream)?;
                authoritative_steps_for_one_participant.push(authoritative_step);
            }

            authoritative_participants
                .insert(
                    participant_id,
                    SerializeAuthoritativeStepVectorForOneParticipants {
                        delta_tick_id_from_range,
                        steps: authoritative_steps_for_one_participant,
                    },
                )
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        }

        Ok(Self {
            authoritative_participants,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CombinedPredictedSteps<StepT> {
    pub first_tick: TickId,
    pub steps: Vec<PredictedStep<StepT>>,
}

impl<StepT: Serialize + Deserialize + Clone> Deserialize for CombinedPredictedSteps<StepT> {
    fn deserialize(_: &mut impl ReadOctetStream) -> io::Result<Self>
    where
        Self: Sized,
    {
        // TODO: Deserialize
        Ok(Self {
            first_tick: Default::default(),
            steps: vec![],
        })
    }
}

impl<StepT: Serialize + Deserialize + Clone> Serialize for CombinedPredictedSteps<StepT> {
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        let mut unique_keys: HashSet<u8> = HashSet::new();
        for map in &self.steps {
            for key in map.predicted_players.keys() {
                unique_keys.insert(*key);
            }
        }

        let mut sorted_unique_ids: Vec<u8> = unique_keys.into_iter().collect();
        sorted_unique_ids.sort();

        let mut root_hash_map =
            SeqMap::<LocalIndex, SerializePredictedStepsVectorForOnePlayer<StepT>>::new();

        for local_index in sorted_unique_ids {
            let vector_for_one_player = SerializePredictedStepsVectorForOnePlayer::<StepT> {
                first_tick_id: Default::default(),
                predicted_steps: vec![],
            };
            root_hash_map
                .insert(local_index, vector_for_one_player)
                .expect("local_index should be unique");
        }

        let mut current_tick_id = self.first_tick;

        for combined_step in &self.steps {
            for (local_index, predicted_step) in &combined_step.predicted_players {
                let vector_for_one_player = root_hash_map.get_mut(local_index).unwrap();
                if vector_for_one_player.predicted_steps.is_empty() {
                    vector_for_one_player.first_tick_id = current_tick_id;
                }
                vector_for_one_player
                    .predicted_steps
                    .push(predicted_step.clone());
            }
            current_tick_id += 1;
        }

        let all = SerializePredictedStepsForAllPlayers {
            predicted_players: root_hash_map,
        };

        all.to_stream(stream)
    }
}

#[derive(Debug, Clone)]
pub struct SerializePredictedStepsForAllPlayers<StepT: Serialize + Deserialize> {
    pub predicted_players: SeqMap<LocalIndex, SerializePredictedStepsVectorForOnePlayer<StepT>>,
}

impl<StepT: Serialize + Deserialize> SerializePredictedStepsForAllPlayers<StepT> {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        stream.write_u8(self.predicted_players.len() as u8)?;

        for (local_player_id, predicted_steps_for_one_player) in &self.predicted_players {
            stream.write_u8(*local_player_id)?;
            predicted_steps_for_one_player.to_stream(stream)?;
        }

        Ok(())
    }

    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let player_count = stream.read_u8()?;

        let mut players_vector = SeqMap::new();

        for _ in 0..player_count {
            let predicted_steps_for_one_player =
                SerializePredictedStepsVectorForOnePlayer::from_stream(stream)?;
            let index = stream.read_u8()?;
            players_vector
                .insert(index, predicted_steps_for_one_player)
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        }

        Ok(Self {
            predicted_players: players_vector,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SerializePredictedStepsVectorForOnePlayer<StepT: Serialize + Deserialize> {
    pub first_tick_id: TickId,
    pub predicted_steps: Vec<StepT>,
}

impl<StepT: Serialize + Deserialize> SerializePredictedStepsVectorForOnePlayer<StepT> {
    pub fn to_stream(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        TickIdUtil::to_stream(self.first_tick_id, stream)?;
        stream.write_u8(self.predicted_steps.len() as u8)?;

        for predicted_step_for_player in self.predicted_steps.iter() {
            predicted_step_for_player.serialize(stream)?;
        }

        Ok(())
    }
    pub fn from_stream(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let first_tick_id = TickIdUtil::from_stream(stream)?;
        let step_count = stream.read_u8()?;

        let mut predicted_steps_for_players = Vec::<StepT>::with_capacity(step_count as usize);

        for _ in 0..step_count {
            let predicted_steps_for_player = StepT::deserialize(stream)?;
            predicted_steps_for_players.push(predicted_steps_for_player);
        }

        Ok(Self {
            first_tick_id,
            predicted_steps: predicted_steps_for_players,
        })
    }
}

#[derive(Debug, Clone)]
pub struct StepsRequest<StepT: Clone + Serialize + Deserialize + Debug> {
    pub ack: StepsAck,
    pub combined_predicted_steps: CombinedPredictedSteps<StepT>,
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
            combined_predicted_steps: CombinedPredictedSteps::deserialize(stream)?,
        })
    }
}
