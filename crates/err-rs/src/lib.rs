/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
#[derive(Eq, Debug, PartialEq)]
pub enum ErrorLevel {
    Info,     // Informative, can be ignored
    Warning,  // Should be logged, but recoverable
    Critical, // Requires immediate attention, unrecoverable
}

pub trait ErrorLevelProvider {
    fn error_level(&self) -> ErrorLevel;
}
