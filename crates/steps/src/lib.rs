/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

/*!
# Nimble Steps Crate

The `nimble-steps` crate provides utilities for managing a sequence of deterministic simulation steps.
Each step is associated with a unique tick identifier ([`TickId`]), ensuring that steps are processed in the correct order.

The crate offers functionality for pushing steps, iterating over them, and managing the internal state of the step queue.
It supports both direct manipulation of the step queue and indexed iteration, useful for simulation rollback or similar operations.

## Example

```rust
use nimble_steps::{Steps, StepInfo};
use tick_id::TickId;

// Create a new Steps instance with an initial tick
let mut steps = Steps::new(TickId::new(0));

// Push steps into the queue
steps.push_with_check(TickId::new(0), "Step 1").unwrap();
steps.push_with_check(TickId::new(1), "Step 2").unwrap();

// Pop the first step
let step = steps.pop();
assert_eq!(step.unwrap().step, "Step 1");

// Iterate over remaining steps
for step in steps.iter() {
 println!("Tick {}: {}", step.tick_id, step.step);
}
```

*/

use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter};
use tick_id::TickId;

#[derive(Debug, PartialEq, Clone)]
pub struct StepInfo<T> {
    pub step: T,
    pub tick_id: TickId,
}

impl<T: Display> Display for StepInfo<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.tick_id, self.step)
    }
}

#[derive(Default, Debug)]
pub struct Steps<T> {
    steps: VecDeque<StepInfo<T>>,
    expected_read_id: TickId,
    expected_write_id: TickId,
}

impl<T> Steps<T> {
    pub fn iter(&self) -> impl Iterator<Item = &StepInfo<T>> {
        self.steps.iter()
    }
}

pub struct FromIndexIterator<'a, T> {
    deque: &'a VecDeque<StepInfo<T>>,
    #[allow(unused)]
    start_index: usize,
    current_index: usize,
}

impl<'a, T> FromIndexIterator<'a, T> {
    pub fn new(deque: &'a VecDeque<StepInfo<T>>, start_index: usize) -> Self {
        Self {
            deque,
            start_index,
            current_index: start_index,
        }
    }
}

impl<StepType: Clone> Iterator for FromIndexIterator<'_, StepType> {
    type Item = StepInfo<StepType>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.deque.get(self.current_index)?;
        self.current_index += 1;
        Some(item.clone())
    }
}

pub const TICK_ID_MAX: u32 = u32::MAX;

#[derive(Debug)]
pub enum StepsError {
    WrongTickId {
        expected: TickId,
        encountered: TickId,
    },
    CanNotPushEmptyPredictedSteps,
}

impl<StepType: Clone> Steps<StepType> {
    pub fn new(tick_id: TickId) -> Self {
        Self {
            steps: VecDeque::new(),
            expected_read_id: tick_id,
            expected_write_id: tick_id,
        }
    }

    /// Clears the queue and resets the expected read and write tick IDs.
    pub fn clear(&mut self, initial_tick_id: TickId) {
        self.steps.clear();
        self.expected_read_id = initial_tick_id;
        self.expected_write_id = initial_tick_id;
    }

    pub fn push_with_check(&mut self, tick_id: TickId, step: StepType) -> Result<(), StepsError> {
        if self.expected_write_id != tick_id {
            Err(StepsError::WrongTickId {
                expected: self.expected_write_id,
                encountered: tick_id,
            })?;
        }

        self.push(step);

        Ok(())
    }

    fn push(&mut self, step: StepType) {
        let info = StepInfo {
            step,
            tick_id: self.expected_write_id,
        };
        self.steps.push_back(info);
        self.expected_write_id += 1;
    }

    pub fn debug_get(&self, index: usize) -> Option<&StepInfo<StepType>> {
        self.steps.get(index)
    }

    pub fn pop(&mut self) -> Option<StepInfo<StepType>> {
        let info = self.steps.pop_front();
        if let Some(ref step_info) = info {
            assert_eq!(step_info.tick_id, self.expected_read_id);
            self.expected_read_id += 1;
        }
        info
    }

    pub fn discard_up_to(&mut self, tick_id: TickId) {
        while let Some(info) = self.steps.front() {
            if info.tick_id >= tick_id {
                break;
            }

            self.steps.pop_front();
        }
    }

    pub fn discard_count(&mut self, count: usize) {
        if count >= self.steps.len() {
            self.steps.clear();
        } else {
            self.steps.drain(..count);
        }
    }

    /// Pops up to a certain amount of steps from the front of the queue and returns
    /// the first `TickId` and a vector of `StepType`. Returns `None` if the queue
    /// is empty.
    ///
    /// # Parameters
    /// - `count`: The number of steps to pop (or fewer if not enough steps are available).
    ///
    /// # Returns
    /// - `Some((TickId, Vec<StepType>))` if there are steps available.
    /// - `None` if the queue is empty.
    ///
    /// # Example
    /// ```rust
    /// use tick_id::TickId;
    /// use nimble_steps::Steps;
    /// let mut steps = Steps::new(TickId::new(0));
    /// steps.push_with_check(TickId::new(0), "Step 1").unwrap();
    /// steps.push_with_check(TickId::new(1), "Step 2").unwrap();
    ///
    /// let result = steps.take(5);  // Will return up to 5 steps (in this case 2)
    /// if let Some((tick_id, popped_steps)) = result {
    ///     assert_eq!(tick_id, TickId::new(0));
    ///     assert_eq!(popped_steps, vec!["Step 1", "Step 2"]);
    /// }
    /// ```
    pub fn take(&mut self, count: usize) -> Option<(TickId, Vec<StepType>)> {
        let first_tick_id = self.front_tick_id()?;

        let steps_to_take: Vec<StepType> = self
            .steps
            .drain(..count.min(self.steps.len()))
            .map(|step_info| step_info.step)
            .collect();

        // Advance the expected read ID
        self.expected_read_id += steps_to_take.len() as u32;

        Some((first_tick_id, steps_to_take))
    }

    pub fn front_tick_id(&self) -> Option<TickId> {
        self.steps.front().map(|step_info| step_info.tick_id)
    }

    pub fn expected_write_tick_id(&self) -> TickId {
        self.expected_write_id
    }

    pub fn back_tick_id(&self) -> Option<TickId> {
        self.steps.back().map(|step_info| step_info.tick_id)
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn to_vec(&self) -> Vec<StepType> {
        let (front_slice, back_slice) = self.steps.as_slices();
        front_slice
            .iter()
            .chain(back_slice.iter())
            .map(|step_info| step_info.step.clone())
            .collect()
    }

    pub fn iter_index(&self, start_index: usize) -> FromIndexIterator<StepType> {
        FromIndexIterator::new(&self.steps, start_index)
    }
}
