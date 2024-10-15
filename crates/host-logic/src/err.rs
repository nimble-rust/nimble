use err_rs::{ErrorLevel, ErrorLevelProvider};
use freelist_rs::FreeListError;
use nimble_blob_stream::out_stream::OutStreamError;
use nimble_participant::ParticipantId;
use nimble_steps::StepsError;
use crate::combinator::CombinatorError;
use crate::combine::HostCombinatorError;
use crate::HostConnectionId;

#[derive(Debug)]
pub enum HostLogicError {
    UnknownConnectionId(HostConnectionId),
    FreeListError {
        connection_id: HostConnectionId,
        message: FreeListError,
    },
    UnknownPartyMember(ParticipantId),
    NoFreeParticipantIds,
    BlobStreamErr(OutStreamError),
    NoDownloadNow,
    CombinatorError(CombinatorError),
    HostCombinatorError(HostCombinatorError),
    NeedConnectRequestFirst,
    WrongApplicationVersion,
    StepsError(StepsError),
}

impl ErrorLevelProvider for HostLogicError {
    fn error_level(&self) -> ErrorLevel {
        match self {
            Self::UnknownConnectionId(_) => ErrorLevel::Warning,
            Self::FreeListError { .. } => ErrorLevel::Critical,
            Self::UnknownPartyMember(_) => ErrorLevel::Warning,
            Self::NoFreeParticipantIds => ErrorLevel::Warning,
            Self::BlobStreamErr(_) => ErrorLevel::Info,
            Self::NoDownloadNow => ErrorLevel::Info,
            Self::CombinatorError(err) => err.error_level(),
            Self::HostCombinatorError(err) => err.error_level(),
            Self::NeedConnectRequestFirst => ErrorLevel::Info,
            Self::WrongApplicationVersion => ErrorLevel::Critical,
            Self::StepsError(_) => ErrorLevel::Critical,
        }
    }
}

impl From<CombinatorError> for HostLogicError {
    fn from(err: CombinatorError) -> Self {
        Self::CombinatorError(err)
    }
}

impl From<StepsError> for HostLogicError {
    fn from(err: StepsError) -> Self {
        Self::StepsError(err)
    }
}

impl From<HostCombinatorError> for HostLogicError {
    fn from(err: HostCombinatorError) -> Self {
        Self::HostCombinatorError(err)
    }
}

impl From<OutStreamError> for HostLogicError {
    fn from(err: OutStreamError) -> Self {
        Self::BlobStreamErr(err)
    }
}
