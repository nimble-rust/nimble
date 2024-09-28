/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_participant::ParticipantId;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::Index;

// We have a special IndexMap instead of a HashMap, since we want it to be deterministic
// And that the order stored should also be in the same order they were inserted.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IndexMap<K: Eq + Hash, V> {
    key_to_vector_index: HashMap<K, usize>, // HashMap is faster than HashSet for looking up keys
    entries: Vec<(K, V)>,                   // Stores key-value pairs in insertion order
}

#[derive(Debug)]
pub enum IndexMapError {
    IndexAlreadyExists,
}

impl Display for IndexMapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexMapError {self:?}")
    }
}

impl Error for IndexMapError {}

impl<K, V> IndexMap<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            key_to_vector_index: HashMap::new(),
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<(), IndexMapError> {
        #[allow(clippy::map_entry)]
        if self.key_to_vector_index.contains_key(&key) {
            Err(IndexMapError::IndexAlreadyExists)
        } else {
            // Key does not exist, insert at the end
            self.entries.push((key.clone(), value));
            self.key_to_vector_index.insert(key, self.entries.len() - 1);
            Ok(())
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.key_to_vector_index
            .get(key)
            .map(|&index| &mut self.entries[index].1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn slow_get(&self, key: &K) -> Option<&V> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

impl<K, V> Index<K> for IndexMap<K, V>
where
    K: Eq + Hash + Clone,
{
    type Output = V;

    fn index(&self, key: K) -> &Self::Output {
        let index = self
            .key_to_vector_index
            .get(&key)
            .expect("Key not found in IndexMap");

        &self.entries[*index].1
    }
}

// Implementing the Index trait for IndexMap using &K
impl<K, V> Index<&K> for IndexMap<K, V>
where
    K: Eq + Hash + Clone,
{
    type Output = V;

    fn index(&self, key: &K) -> &Self::Output {
        let index = self
            .key_to_vector_index
            .get(key)
            .expect("Key not found in IndexMap");

        &self.entries[*index].1
    }
}

impl<K, V> From<&[(K, V)]> for IndexMap<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone, // Ensure V can be cloned
{
    fn from(slice: &[(K, V)]) -> Self {
        let mut index_map = IndexMap::new();
        for (key, value) in slice {
            // Clone both key and value to insert into IndexMap
            index_map.insert(key.clone(), value.clone()).unwrap();
        }
        index_map
    }
}
impl<K, V> IntoIterator for IndexMap<K, V>
where
    K: Eq + Hash + Clone,
{
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a IndexMap<K, V>
where
    K: Eq + Hash + Clone,
{
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, V)>, fn(&'a (K, V)) -> (&'a K, &'a V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

impl<K, V> Default for IndexMap<K, V>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct AuthoritativeStep<StepT> {
    pub authoritative_participants: IndexMap<ParticipantId, StepT>,
}

pub type LocalIndex = u8;

#[derive(Debug, PartialEq, Clone)]
pub struct PredictedStep<StepT> {
    pub predicted_players: IndexMap<LocalIndex, StepT>,
}

impl<StepT> PredictedStep<StepT> {
    pub fn is_empty(&self) -> bool {
        self.predicted_players.is_empty()
    }
}
