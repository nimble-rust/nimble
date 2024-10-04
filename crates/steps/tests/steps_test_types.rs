/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
#[derive(Debug, Clone, PartialEq, Eq)] // Debug is needed for asserts in tests
pub enum GameInput {
    #[allow(unused)]
    Jumping(bool),
    #[allow(unused)]
    MoveHorizontal(i32),
}
