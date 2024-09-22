/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

//! A module that re-exports commonly used items for convenience.
//!
//! The `prelude` module provides easy access to commonly used types, structs, and enums across
//! various parts of the library. By including this prelude, you can reduce the number of individual
//! imports needed in your code.
pub use {
    crate::err::BlobError,
    crate::in_logic_front::{FrontLogic, Info},
    crate::out_logic_front::OutLogicFront,
    crate::out_stream::OutStreamError,
    crate::protocol::{SetChunkData, StartTransferData, TransferId},
    crate::protocol_front::{
        ReceiverToSenderFrontCommands, SenderToReceiverFrontCommands, SetChunkFrontData,
    },
};
