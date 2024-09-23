/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */
use nimble_ordered_datagram::{DatagramId, OrderedOut};

#[test_log::test]
fn ordered_out() {
    let out = OrderedOut {
        sequence_to_send: DatagramId::new(32),
    };
    assert_eq!(out.sequence_to_send.inner(), 32);
}

#[test_log::test]
fn test_valid() {
    assert!(DatagramId::new(u16::MAX).is_valid_successor(DatagramId::new(0)));
}

#[test_log::test]
fn test_valid_wraparound() {
    assert!(DatagramId::new(u16::MAX).is_valid_successor(DatagramId::new(80)));
}

#[test_log::test]
fn test_wrong_order() {
    assert!(!DatagramId::new(0).is_valid_successor(DatagramId::new(u16::MAX)));
}

#[test_log::test]
fn test_invalid_order() {
    assert!(!DatagramId::new(u16::MAX).is_valid_successor(DatagramId::new(u16::MAX - 31000)));
    assert!(!DatagramId::new(5).is_valid_successor(DatagramId::new(4)));
}
