/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use discoid::discoid::DiscoidBuffer;

use crate::TickId;

#[derive(Clone)]
pub struct PendingStepInfo<StepT: Clone> {
    pub step: StepT,
    pub tick_id: TickId,
}

/// Manages a sequence of pending steps that are queued to be executed at specific ticks in a game loop.
///
/// This struct contains a buffer (`DiscoidBuffer`) of `PendingStepInfo` elements, designed to handle
/// multiple steps that are pending execution across different ticks.
///
/// # Type Parameters
///
/// * `T` - Represents the type of steps stored within the buffer. As a generic parameter, it allows
///   the `PendingSteps` struct to be flexible and applicable to various types of games or simulations
///   where different actions are defined as steps.
///
/// # Fields
///
/// * `steps` - A circular buffer (implemented via `DiscoidBuffer`) optimized for storing and retrieving
///   pending steps efficiently.
///
/// * `front_tick_id` - The tick ID of the first step in the buffer.
///
/// * `capacity` - The maximum number of steps that can be stored in the buffer. This parameter helps
///   control memory usage and maintain performance.
///
/// # Examples
///
/// ```
/// use discoid::discoid::DiscoidBuffer;
/// use tick_id::TickId;
/// use nimble_steps::pending_steps::PendingSteps;
///
/// let pending_steps = PendingSteps::<i32>::new(10, TickId::new(1));
/// ```
pub struct PendingSteps<T: Clone> {
    steps: DiscoidBuffer<PendingStepInfo<T>>,
    front_tick_id: TickId,
    capacity: usize,
}

impl<T: Clone> PendingSteps<T> {
    pub fn new(window_size: usize, tick_id: TickId) -> Self {
        Self {
            steps: DiscoidBuffer::new(window_size),
            front_tick_id: tick_id,
            capacity: window_size,
        }
    }

    pub fn set(&mut self, tick_id: TickId, step: T) -> Result<(), String> {
        let index_in_discoid = tick_id.value() - self.front_tick_id.value();
        if index_in_discoid >= self.capacity as u32 {
            // self.steps.capacity()
            return Err("pending_steps: out of scope".to_string());
        }

        self.steps.set_at_index(
            index_in_discoid as usize,
            PendingStepInfo::<T> { step, tick_id },
        );
        Ok(())
    }

    pub fn discard_up_to(&mut self, tick_id: TickId) {
        let count_in_discoid = tick_id - self.front_tick_id;
        if count_in_discoid < 0 {
            return;
        }
        self.steps.discard_front(count_in_discoid as usize);
    }

    pub fn is_empty(&self) -> bool {
        self.steps.get_ref_at_index(0).is_none()
    }

    pub fn pop(&mut self) -> PendingStepInfo<T> {
        let value = self.steps.get_ref_at_index(0).unwrap().clone();
        self.steps.discard_front(1);
        value
    }

    pub fn front_tick_id(&self) -> Option<TickId> {
        self.steps.get_ref_at_index(0).map(|info| info.tick_id)
    }
}
