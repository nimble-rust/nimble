use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use freelist_rs::FreeList;
use tick_id::TickId;
use nimble_participant::ParticipantId;
use crate::combine::HostCombinator;

#[derive(Copy, Clone, Debug)]
pub struct Participant {
    pub id: ParticipantId,
    pub client_local_index: u8,
}

pub struct GameSession<StepT: Clone + std::fmt::Display> {
    pub participants: HashMap<ParticipantId, Rc<RefCell<Participant>>>,
    pub participant_ids: FreeList<u8>,
    pub(crate) combinator: HostCombinator<StepT>,
}

impl<StepT: Clone + std::fmt::Display> Default for GameSession<StepT> {
    fn default() -> Self {
        Self::new(TickId(0))
    }
}


impl<StepT: Clone + std::fmt::Display> GameSession<StepT> {
    pub fn new(tick_id: TickId) -> Self {
        Self {
            participants: HashMap::new(),
            participant_ids: FreeList::new(0xff),
            combinator: HostCombinator::<StepT>::new(tick_id),
        }
    }

    pub fn create_participants(
        &mut self,
        client_local_indices: &[u8],
    ) -> Option<Vec<Rc<RefCell<Participant>>>> {
        let mut participants: Vec<Rc<RefCell<Participant>>> = vec![];

        let ids = self
            .participant_ids
            .allocate_count(client_local_indices.len())?;
        for (index, id_value) in ids.iter().enumerate() {
            let participant_id = ParticipantId(*id_value);
            let participant = Rc::new(RefCell::new(Participant {
                client_local_index: client_local_indices[index],
                id: participant_id,
            }));

            participants.push(participant.clone());

            self.participants
                .insert(participant_id, participant.clone());
        }

        Some(participants)
    }
}
