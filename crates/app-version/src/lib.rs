#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

pub trait VersionProvider {
    fn version() -> Version;
}
