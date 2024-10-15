use datagram_chunker::DatagramChunkerError;
use err_rs::{ErrorLevel, ErrorLevelProvider};
use nimble_host_logic::err::HostLogicError;
use nimble_layer::NimbleLayerError;

#[derive(Debug)]
pub enum HostError {
    ConnectionNotFound(u8),
    IoError(std::io::Error),
    NimbleLayerError(NimbleLayerError),
    HostLogicError(HostLogicError),
    DatagramChunkerError(DatagramChunkerError),
}

impl ErrorLevelProvider for HostError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::IoError(_) => ErrorLevel::Warning,
            Self::ConnectionNotFound(_) => ErrorLevel::Warning,
            Self::NimbleLayerError(_) => ErrorLevel::Warning,
            Self::HostLogicError(err) => err.error_level(),
            Self::DatagramChunkerError(err) => err.error_level(),
        }
    }
}

impl From<DatagramChunkerError> for HostError {
    fn from(err: DatagramChunkerError) -> Self {
        Self::DatagramChunkerError(err)
    }
}

impl From<HostLogicError> for HostError {
    fn from(err: HostLogicError) -> Self {
        Self::HostLogicError(err)
    }
}

impl From<NimbleLayerError> for HostError {
    fn from(e: NimbleLayerError) -> Self {
        Self::NimbleLayerError(e)
    }
}

impl From<std::io::Error> for HostError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}
