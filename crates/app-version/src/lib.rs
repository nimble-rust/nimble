/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/nimble-rust/nimble
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

/// A struct representing a semantic version.
///
/// This struct contains three components of a version: major, minor, and patch.
/// It derives common traits for easy comparison and manipulation.
///
/// # Examples
///
/// Creating a new version:
///
/// ```
/// use app_version::Version;
///
/// let version = Version::new(1,0,0);
/// assert_eq!(version.major(), 1);
/// assert_eq!(version.minor(), 0);
/// assert_eq!(version.patch(), 0);
/// ```
///
/// ## Comparison
///
/// Versions can be compared for equality:
///
/// ```
/// use app_version::Version;
///
/// let version1 = Version::new(1,0,0);
/// let version2 = Version::new(1,0,1);
/// assert_ne!(version1, version2);
/// ```
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

impl Version {
    /// Creates a new version.
    ///
    /// # Parameters
    /// - `major`: The major version number.
    /// - `minor`: The minor version number.
    /// - `patch`: The patch version number.
    ///
    /// # Returns
    /// A `Version` instance.
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the major version number.
    pub fn major(&self) -> u16 {
        self.major
    }

    /// Returns the minor version number.
    pub fn minor(&self) -> u16 {
        self.minor
    }

    /// Returns the patch version number.
    pub fn patch(&self) -> u16 {
        self.patch
    }

    /// Increments the patch version.
    pub fn increment_patch(&mut self) {
        self.patch += 1;
    }

    /// Increments the minor version and resets patch to 0.
    pub fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// Increments the major version and resets minor and patch to 0.
    pub fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }
}

// Implement the `fmt::Display` trait for `Version`
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<(u16, u16, u16)> for Version {
    fn from(tuple: (u16, u16, u16)) -> Self {
        Version::new(tuple.0, tuple.1, tuple.2)
    }
}

#[derive(Debug)]
pub enum VersionError {
    ParseIntError(ParseIntError),
    InvalidFormat,
}

impl From<ParseIntError> for VersionError {
    fn from(error: ParseIntError) -> Self {
        VersionError::ParseIntError(error)
    }
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionError::InvalidFormat => write!(f, "Invalid version format"),
            VersionError::ParseIntError(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for VersionError {}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat);
        }

        let major = parts[0].parse::<u16>()?;
        let minor = parts[1].parse::<u16>()?;
        let patch = parts[2].parse::<u16>()?;

        Ok(Version::new(major, minor, patch))
    }
}

/// A trait that provides a version.
///
/// Implementers of this trait must define how to return the version
/// associated with them. This is useful for encapsulating versioning
/// logic within various components or libraries.
///
/// # Examples
///
/// A simple implementation of `VersionProvider`:
///
/// ```
/// use app_version::{Version, VersionProvider};
///
/// struct MySoftware;
///
/// impl VersionProvider for MySoftware {
///     fn version() -> Version {
///         Version::new(1, 0, 0)
///     }
/// }
///
/// let my_version = MySoftware::version();
/// assert_eq!(my_version, Version::new(1, 0, 0 ));
/// ```

pub trait VersionProvider {
    fn version() -> Version;
}
