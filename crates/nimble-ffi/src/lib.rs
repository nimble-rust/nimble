/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

//! # Nimble FFI Library
//!
//! This is sample code that demonstrates the envisioned structure of a future Nimble FFI library.

pub use app_version::{Version, VersionProvider};
use flood_rs::{BufferDeserializer, Deserialize, ReadOctetStream, Serialize, WriteOctetStream};
use monotonic_time_rs::Millis;
use nimble_client::prelude::{AssentCallback, RectifyCallback, SeerCallback};
use nimble_client::Client;
use nimble_step::Step;
use nimble_step_map::StepMap;
use nimble_wrapped_step::{GenericOctetStep, WrappedOctetStep};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::c_int;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub type CallbackFn = extern "C" fn(param: u64);
pub type CallbackHandle = u64;

// --------------------------------------------------------
// Globals
static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1); // Start from 1; 0 is always invalid
static CLIENT_REGISTRY: Lazy<Mutex<HashMap<Handle, ClientInfo>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
// --------------------------------------------------------

type GenericStep = WrappedOctetStep<GenericOctetStep>;
pub struct GenericOctetGameState(pub Vec<u8>);

#[derive(Debug)]
pub struct GenericOctetGame;

type GenericClient = Client<GenericOctetGame, GenericStep>;

/// Holds all information regarding a created client, usually just one
pub struct ClientInfo {
    pub client: GenericClient,
    callback1: Option<CallbackFn>,
    callback2: Option<CallbackFn>,
    callback3: Option<CallbackFn>,
    callback4: Option<CallbackFn>,
}

impl ClientInfo {
    #[must_use]
    pub fn new(now: Millis) -> Self {
        Self {
            client: GenericClient::new(now),
            callback1: None,
            callback2: None,
            callback3: None,
            callback4: None,
        }
    }

    pub fn register_callback1(&mut self, cb: CallbackFn) {
        self.callback1 = Some(cb);
    }

    pub fn register_callback2(&mut self, cb: CallbackFn) {
        self.callback2 = Some(cb);
    }

    pub fn register_callback3(&mut self, cb: CallbackFn) {
        self.callback3 = Some(cb);
    }

    pub fn register_callback4(&mut self, cb: CallbackFn) {
        self.callback4 = Some(cb);
    }

    pub fn unregister_callback1(&mut self) {
        self.callback1 = None;
    }

    pub fn unregister_callback2(&mut self) {
        self.callback2 = None;
    }

    pub fn unregister_callback3(&mut self) {
        self.callback3 = None;
    }

    pub fn unregister_callback4(&mut self) {
        self.callback4 = None;
    }

    /// Methods to invoke callbacks
    #[must_use]
    pub fn invoke_callback1(&self, param: u64) -> Option<c_int> {
        if let Some(cb) = self.callback1 {
            cb(param);
        }
        Some(-1)
    }

    pub fn trigger_all_callbacks(&self) {
        // Example trigger, modify as needed
        let _ = self.invoke_callback1(42);
        if let Some(result) = self.invoke_callback1(10) {
            println!("Callback2 returned: {result}");
        }
        // ... Invoke other callbacks
    }
}

impl BufferDeserializer for GenericOctetGame {
    fn deserialize(_: &[u8]) -> io::Result<(Self, usize)> {
        //        let (sample_state, size) = GenericOctetGameState::from_octets(octets)?;
        Ok((Self {}, 10))
    }
}

impl Deserialize for GenericOctetGame {
    fn deserialize(_: &mut impl ReadOctetStream) -> io::Result<Self> {
        Ok(Self {})
    }
}

impl Serialize for GenericOctetGame {
    fn serialize(&self, _: &mut impl WriteOctetStream) -> io::Result<()> {
        Ok(())
    }
}

impl VersionProvider for GenericOctetGame {
    fn version() -> Version {
        Version::new(0, 0, 5)
    }
}

impl SeerCallback<StepMap<Step<GenericStep>>> for GenericOctetGame {
    fn on_pre_ticks(&mut self) {}
    fn on_tick(&mut self, _: &StepMap<Step<GenericStep>>) {}
    fn on_post_ticks(&mut self) {}
}

impl AssentCallback<StepMap<Step<GenericStep>>> for GenericOctetGame {
    fn on_pre_ticks(&mut self) {}
    fn on_tick(&mut self, _: &StepMap<Step<GenericStep>>) {}
    fn on_post_ticks(&mut self) {}
}

impl RectifyCallback for GenericOctetGame {
    fn on_copy_from_authoritative(&mut self) {}
}

pub type Handle = u64;

// -------------------------------------------------------------------------------------------------
// FFI Interface
// -------------------------------------------------------------------------------------------------

/// Creates a client instance and returns the handle
#[no_mangle]
pub extern "C" fn client_new(now: u64) -> Handle {
    let session = ClientInfo::new(Millis::new(now));

    let handle = HANDLE_COUNTER.fetch_add(1, Ordering::SeqCst);

    {
        let mut registry = CLIENT_REGISTRY.lock().unwrap();
        registry.insert(handle, session);
    }

    handle
}

/// Destroys a Client instance using its handle
#[no_mangle]
pub extern "C" fn client_free(handle: Handle) -> c_int {
    let removed = {
        let mut registry = CLIENT_REGISTRY.lock().unwrap();
        registry.remove(&handle)
    };

    match removed {
        Some(_) => 0, // Success
        None => -1,   // Handle not found
    }
}

/// Updates a Client instance with the specified absolute time
#[no_mangle]
pub extern "C" fn client_update(handle: Handle, now: u64) -> c_int {
    let result = {
        let mut registry = CLIENT_REGISTRY.lock().unwrap();
        if let Some(client) = registry.get_mut(&handle) {
            client.client.update(Millis::new(now))
        } else {
            return -1; // Handle not found
        }
    };

    if result.is_ok() {
        0
    } else {
        -1
    }
}
