use crate::host_to_client::TickIdUtil;
use flood_rs::{Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use nimble_participant::ParticipantId;
use nimble_step_types::StepForParticipants;
use seq_map::SeqMap;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use tick_id::TickId;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CombinedSteps<StepT: Deserialize + Serialize + Debug + Clone + std::fmt::Display> {
    pub tick_id: TickId,
    pub steps: Vec<StepForParticipants<StepT>>,
}

impl<StepT: Deserialize + Serialize + Debug + Clone + std::fmt::Display> Display
    for CombinedSteps<StepT>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: step_count:{}", self.tick_id, self.steps.len())
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + std::fmt::Display> Serialize
    for CombinedSteps<StepT>
{
    fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()> {
        TickIdUtil::to_stream(self.tick_id, stream)?;
        self.to_internal().serialize(stream)
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + std::fmt::Display> Deserialize
    for CombinedSteps<StepT>
{
    fn deserialize(stream: &mut impl ReadOctetStream) -> io::Result<Self> {
        let start_tick_id = TickIdUtil::from_stream(stream)?;
        let internal = InternalAllParticipantVectors::deserialize(stream)?;
        Ok(Self::from_internal(&internal, start_tick_id))
    }
}

impl<StepT: Deserialize + Serialize + Debug + Clone + std::fmt::Display> CombinedSteps<StepT> {
    pub fn to_internal(&self) -> InternalAllParticipantVectors<StepT> {
        let mut hash_map =
            SeqMap::<ParticipantId, InternalStepVectorForOneParticipant<StepT>>::new();

        let mut unique_participant_ids: HashSet<ParticipantId> = HashSet::new();

        for auth_step in &self.steps {
            for key in auth_step.combined_step.keys() {
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
                    InternalStepVectorForOneParticipant::<StepT> {
                        delta_tick_id: 0,
                        steps: vec![],
                    },
                )
                .expect("participant ids to be unique");
        }

        for (index_in_range, combined_auth_step) in self.steps.iter().enumerate() {
            for (participant_id, auth_step_for_one_player) in &combined_auth_step.combined_step {
                let vector_for_one_person = hash_map.get_mut(participant_id).unwrap();
                if vector_for_one_person.steps.is_empty() {
                    vector_for_one_person.delta_tick_id = index_in_range as u8;
                }
                vector_for_one_person
                    .steps
                    .push(auth_step_for_one_player.clone())
            }
        }

        InternalAllParticipantVectors::<StepT> {
            participant_step_vectors: hash_map,
        }
    }

    pub fn from_internal(
        separate_vectors: &InternalAllParticipantVectors<StepT>,
        start_tick_id: TickId,
    ) -> Self {
        let mut max_vector_length = 0;

        for serialized_step_vector in separate_vectors.participant_step_vectors.values() {
            if serialized_step_vector.steps.len() > max_vector_length {
                max_vector_length = serialized_step_vector.steps.len();
            }
        }

        let mut auth_step_range_vec = Vec::<StepForParticipants<StepT>>::new();
        for _ in 0..max_vector_length {
            auth_step_range_vec.push(StepForParticipants::<StepT> {
                combined_step: SeqMap::new(),
            })
        }

        for (participant_id, serialized_step_vector) in &separate_vectors.participant_step_vectors {
            for (index, serialized_step) in serialized_step_vector.steps.iter().enumerate() {
                let hash_map_for_auth_step =
                    &mut auth_step_range_vec.get_mut(index).unwrap().combined_step;
                hash_map_for_auth_step
                    .insert(*participant_id, serialized_step.clone())
                    .expect("expect unique participant_id");
            }
        }

        CombinedSteps::<StepT> {
            tick_id: start_tick_id,
            steps: auth_step_range_vec,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct InternalStepVectorForOneParticipant<StepT: Serialize + Deserialize>
where
    StepT: std::fmt::Display,
{
    pub delta_tick_id: u8, // enables one vector to start at a later tick_id than the others
    pub steps: Vec<StepT>,
}

impl<StepT: Serialize + Deserialize + Display> Display
    for InternalStepVectorForOneParticipant<StepT>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "delta_tick {} step_count:{}",
            self.delta_tick_id,
            self.steps.len()
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct InternalAllParticipantVectors<StepT: Serialize + Deserialize + std::fmt::Display> {
    pub participant_step_vectors: SeqMap<ParticipantId, InternalStepVectorForOneParticipant<StepT>>,
}

impl<StepT: Serialize + Deserialize + Debug + std::fmt::Display>
    InternalAllParticipantVectors<StepT>
{
    pub fn serialize(&self, stream: &mut impl WriteOctetStream) -> io::Result<()>
    where
        Self: Sized,
    {
        // How many participants streams follows
        stream.write_u8(self.participant_step_vectors.len() as u8)?;

        for (participant_id, authoritative_steps_for_one_player_vector) in
            &self.participant_step_vectors
        {
            participant_id.to_stream(stream)?;
            stream.write_u8(authoritative_steps_for_one_player_vector.delta_tick_id)?;
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
                    InternalStepVectorForOneParticipant {
                        delta_tick_id: delta_tick_id_from_range,
                        steps: authoritative_steps_for_one_participant,
                    },
                )
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        }

        Ok(Self {
            participant_step_vectors: authoritative_participants,
        })
    }
}

// ----

#[derive(Debug)]
pub struct InternalAuthoritativeStepRange<
    StepT: Deserialize + Serialize + Debug + Clone + std::fmt::Display,
> {
    pub delta_tick_id_from_previous: u8,
    pub authoritative_steps: InternalAllParticipantVectors<StepT>,
}
