#![allow(non_snake_case)]

use std::convert::Infallible;
use std::io;

use actix::MailboxError;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};
use solana_program::clock::Slot;
use solana_program::message::CompileError;
use solana_program::sanitize::SanitizeError;
use solana_pubkey::Pubkey;
use solana_rpc_client_api::client_error::{Error as ClientError, ErrorKind as ClientErrorKind};
use solana_rpc_client_api::request::{RpcError, RpcRequest, RpcResponseErrorData};
use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_signer::{PresignerError, SignerError};
use solana_transaction_error::TransactionError;

#[derive(Serialize, Deserialize)]
pub enum AsRpcRequestImpl {
    Custom,
    DeregisterNode,
    GetAccountInfo,
    GetBalance,
    GetBlock,
    GetBlockHeight,
    GetBlockProduction,
    GetBlocks,
    GetBlocksWithLimit,
    GetBlockTime,
    GetClusterNodes,
    GetEpochInfo,
    GetEpochSchedule,
    GetFeeForMessage,
    GetFirstAvailableBlock,
    GetGenesisHash,
    GetHealth,
    GetIdentity,
    GetInflationGovernor,
    GetInflationRate,
    GetInflationReward,
    GetLargestAccounts,
    GetLatestBlockhash,
    GetLeaderSchedule,
    GetMaxRetransmitSlot,
    GetMaxShredInsertSlot,
    GetMinimumBalanceForRentExemption,
    GetMultipleAccounts,
    GetProgramAccounts,
    GetRecentPerformanceSamples,
    GetRecentPrioritizationFees,
    GetHighestSnapshotSlot,
    GetSignaturesForAddress,
    GetSignatureStatuses,
    GetSlot,
    GetSlotLeader,
    GetSlotLeaders,
    GetStorageTurn,
    GetStorageTurnRate,
    GetSlotsPerSegment,
    GetStakeMinimumDelegation,
    GetStoragePubkeysForSlot,
    GetSupply,
    GetTokenAccountBalance,
    GetTokenAccountsByDelegate,
    GetTokenAccountsByOwner,
    GetTokenLargestAccounts,
    GetTokenSupply,
    GetTransaction,
    GetTransactionCount,
    GetVersion,
    GetVoteAccounts,
    IsBlockhashValid,
    MinimumLedgerSlot,
    RegisterNode,
    RequestAirdrop,
    SendTransaction,
    SimulateTransaction,
    SignVote,
}

fn ser_rpc_request(r: &RpcRequest) -> AsRpcRequestImpl {
    match r {
        RpcRequest::Custom { .. } => AsRpcRequestImpl::Custom,
        RpcRequest::DeregisterNode => AsRpcRequestImpl::DeregisterNode,
        RpcRequest::GetAccountInfo => AsRpcRequestImpl::GetAccountInfo,
        RpcRequest::GetBalance => AsRpcRequestImpl::GetBalance,
        RpcRequest::GetBlock => AsRpcRequestImpl::GetBlock,
        RpcRequest::GetBlockHeight => AsRpcRequestImpl::GetBlockHeight,
        RpcRequest::GetBlockProduction => AsRpcRequestImpl::GetBlockProduction,
        RpcRequest::GetBlocks => AsRpcRequestImpl::GetBlocks,
        RpcRequest::GetBlocksWithLimit => AsRpcRequestImpl::GetBlocksWithLimit,
        RpcRequest::GetBlockTime => AsRpcRequestImpl::GetBlockTime,
        RpcRequest::GetClusterNodes => AsRpcRequestImpl::GetClusterNodes,
        RpcRequest::GetEpochInfo => AsRpcRequestImpl::GetEpochInfo,
        RpcRequest::GetEpochSchedule => AsRpcRequestImpl::GetEpochSchedule,
        RpcRequest::GetFeeForMessage => AsRpcRequestImpl::GetFeeForMessage,
        RpcRequest::GetFirstAvailableBlock => AsRpcRequestImpl::GetFirstAvailableBlock,
        RpcRequest::GetGenesisHash => AsRpcRequestImpl::GetGenesisHash,
        RpcRequest::GetHealth => AsRpcRequestImpl::GetHealth,
        RpcRequest::GetIdentity => AsRpcRequestImpl::GetIdentity,
        RpcRequest::GetInflationGovernor => AsRpcRequestImpl::GetInflationGovernor,
        RpcRequest::GetInflationRate => AsRpcRequestImpl::GetInflationRate,
        RpcRequest::GetInflationReward => AsRpcRequestImpl::GetInflationReward,
        RpcRequest::GetLargestAccounts => AsRpcRequestImpl::GetLargestAccounts,
        RpcRequest::GetLatestBlockhash => AsRpcRequestImpl::GetLatestBlockhash,
        RpcRequest::GetLeaderSchedule => AsRpcRequestImpl::GetLeaderSchedule,
        RpcRequest::GetMaxRetransmitSlot => AsRpcRequestImpl::GetMaxRetransmitSlot,
        RpcRequest::GetMaxShredInsertSlot => AsRpcRequestImpl::GetMaxShredInsertSlot,
        RpcRequest::GetMinimumBalanceForRentExemption => {
            AsRpcRequestImpl::GetMinimumBalanceForRentExemption
        }
        RpcRequest::GetMultipleAccounts => AsRpcRequestImpl::GetMultipleAccounts,
        RpcRequest::GetProgramAccounts => AsRpcRequestImpl::GetProgramAccounts,
        RpcRequest::GetRecentPerformanceSamples => AsRpcRequestImpl::GetRecentPerformanceSamples,
        RpcRequest::GetRecentPrioritizationFees => AsRpcRequestImpl::GetRecentPrioritizationFees,
        RpcRequest::GetHighestSnapshotSlot => AsRpcRequestImpl::GetHighestSnapshotSlot,
        RpcRequest::GetSignaturesForAddress => AsRpcRequestImpl::GetSignaturesForAddress,
        RpcRequest::GetSignatureStatuses => AsRpcRequestImpl::GetSignatureStatuses,
        RpcRequest::GetSlot => AsRpcRequestImpl::GetSlot,
        RpcRequest::GetSlotLeader => AsRpcRequestImpl::GetSlotLeader,
        RpcRequest::GetSlotLeaders => AsRpcRequestImpl::GetSlotLeaders,
        RpcRequest::GetStorageTurn => AsRpcRequestImpl::GetStorageTurn,
        RpcRequest::GetStorageTurnRate => AsRpcRequestImpl::GetStorageTurnRate,
        RpcRequest::GetSlotsPerSegment => AsRpcRequestImpl::GetSlotsPerSegment,
        RpcRequest::GetStakeMinimumDelegation => AsRpcRequestImpl::GetStakeMinimumDelegation,
        RpcRequest::GetStoragePubkeysForSlot => AsRpcRequestImpl::GetStoragePubkeysForSlot,
        RpcRequest::GetSupply => AsRpcRequestImpl::GetSupply,
        RpcRequest::GetTokenAccountBalance => AsRpcRequestImpl::GetTokenAccountBalance,
        RpcRequest::GetTokenAccountsByDelegate => AsRpcRequestImpl::GetTokenAccountsByDelegate,
        RpcRequest::GetTokenAccountsByOwner => AsRpcRequestImpl::GetTokenAccountsByOwner,
        RpcRequest::GetTokenLargestAccounts => AsRpcRequestImpl::GetTokenLargestAccounts,
        RpcRequest::GetTokenSupply => AsRpcRequestImpl::GetTokenSupply,
        RpcRequest::GetTransaction => AsRpcRequestImpl::GetTransaction,
        RpcRequest::GetTransactionCount => AsRpcRequestImpl::GetTransactionCount,
        RpcRequest::GetVersion => AsRpcRequestImpl::GetVersion,
        RpcRequest::GetVoteAccounts => AsRpcRequestImpl::GetVoteAccounts,
        RpcRequest::IsBlockhashValid => AsRpcRequestImpl::IsBlockhashValid,
        RpcRequest::MinimumLedgerSlot => AsRpcRequestImpl::MinimumLedgerSlot,
        RpcRequest::RegisterNode => AsRpcRequestImpl::RegisterNode,
        RpcRequest::RequestAirdrop => AsRpcRequestImpl::RequestAirdrop,
        RpcRequest::SendTransaction => AsRpcRequestImpl::SendTransaction,
        RpcRequest::SimulateTransaction => AsRpcRequestImpl::SimulateTransaction,
        RpcRequest::SignVote => AsRpcRequestImpl::SignVote,
    }
}

fn de_rpc_request(r: AsRpcRequestImpl) -> Result<RpcRequest, Infallible> {
    Ok(match r {
        AsRpcRequestImpl::Custom => RpcRequest::Custom { method: "unknown" },
        AsRpcRequestImpl::DeregisterNode => RpcRequest::DeregisterNode,
        AsRpcRequestImpl::GetAccountInfo => RpcRequest::GetAccountInfo,
        AsRpcRequestImpl::GetBalance => RpcRequest::GetBalance,
        AsRpcRequestImpl::GetBlock => RpcRequest::GetBlock,
        AsRpcRequestImpl::GetBlockHeight => RpcRequest::GetBlockHeight,
        AsRpcRequestImpl::GetBlockProduction => RpcRequest::GetBlockProduction,
        AsRpcRequestImpl::GetBlocks => RpcRequest::GetBlocks,
        AsRpcRequestImpl::GetBlocksWithLimit => RpcRequest::GetBlocksWithLimit,
        AsRpcRequestImpl::GetBlockTime => RpcRequest::GetBlockTime,
        AsRpcRequestImpl::GetClusterNodes => RpcRequest::GetClusterNodes,
        AsRpcRequestImpl::GetEpochInfo => RpcRequest::GetEpochInfo,
        AsRpcRequestImpl::GetEpochSchedule => RpcRequest::GetEpochSchedule,
        AsRpcRequestImpl::GetFeeForMessage => RpcRequest::GetFeeForMessage,
        AsRpcRequestImpl::GetFirstAvailableBlock => RpcRequest::GetFirstAvailableBlock,
        AsRpcRequestImpl::GetGenesisHash => RpcRequest::GetGenesisHash,
        AsRpcRequestImpl::GetHealth => RpcRequest::GetHealth,
        AsRpcRequestImpl::GetIdentity => RpcRequest::GetIdentity,
        AsRpcRequestImpl::GetInflationGovernor => RpcRequest::GetInflationGovernor,
        AsRpcRequestImpl::GetInflationRate => RpcRequest::GetInflationRate,
        AsRpcRequestImpl::GetInflationReward => RpcRequest::GetInflationReward,
        AsRpcRequestImpl::GetLargestAccounts => RpcRequest::GetLargestAccounts,
        AsRpcRequestImpl::GetLatestBlockhash => RpcRequest::GetLatestBlockhash,
        AsRpcRequestImpl::GetLeaderSchedule => RpcRequest::GetLeaderSchedule,
        AsRpcRequestImpl::GetMaxRetransmitSlot => RpcRequest::GetMaxRetransmitSlot,
        AsRpcRequestImpl::GetMaxShredInsertSlot => RpcRequest::GetMaxShredInsertSlot,
        AsRpcRequestImpl::GetMinimumBalanceForRentExemption => {
            RpcRequest::GetMinimumBalanceForRentExemption
        }
        AsRpcRequestImpl::GetMultipleAccounts => RpcRequest::GetMultipleAccounts,
        AsRpcRequestImpl::GetProgramAccounts => RpcRequest::GetProgramAccounts,
        AsRpcRequestImpl::GetRecentPerformanceSamples => RpcRequest::GetRecentPerformanceSamples,
        AsRpcRequestImpl::GetRecentPrioritizationFees => RpcRequest::GetRecentPrioritizationFees,
        AsRpcRequestImpl::GetHighestSnapshotSlot => RpcRequest::GetHighestSnapshotSlot,
        AsRpcRequestImpl::GetSignaturesForAddress => RpcRequest::GetSignaturesForAddress,
        AsRpcRequestImpl::GetSignatureStatuses => RpcRequest::GetSignatureStatuses,
        AsRpcRequestImpl::GetSlot => RpcRequest::GetSlot,
        AsRpcRequestImpl::GetSlotLeader => RpcRequest::GetSlotLeader,
        AsRpcRequestImpl::GetSlotLeaders => RpcRequest::GetSlotLeaders,
        AsRpcRequestImpl::GetStorageTurn => RpcRequest::GetStorageTurn,
        AsRpcRequestImpl::GetStorageTurnRate => RpcRequest::GetStorageTurnRate,
        AsRpcRequestImpl::GetSlotsPerSegment => RpcRequest::GetSlotsPerSegment,
        AsRpcRequestImpl::GetStakeMinimumDelegation => RpcRequest::GetStakeMinimumDelegation,
        AsRpcRequestImpl::GetStoragePubkeysForSlot => RpcRequest::GetStoragePubkeysForSlot,
        AsRpcRequestImpl::GetSupply => RpcRequest::GetSupply,
        AsRpcRequestImpl::GetTokenAccountBalance => RpcRequest::GetTokenAccountBalance,
        AsRpcRequestImpl::GetTokenAccountsByDelegate => RpcRequest::GetTokenAccountsByDelegate,
        AsRpcRequestImpl::GetTokenAccountsByOwner => RpcRequest::GetTokenAccountsByOwner,
        AsRpcRequestImpl::GetTokenLargestAccounts => RpcRequest::GetTokenLargestAccounts,
        AsRpcRequestImpl::GetTokenSupply => RpcRequest::GetTokenSupply,
        AsRpcRequestImpl::GetTransaction => RpcRequest::GetTransaction,
        AsRpcRequestImpl::GetTransactionCount => RpcRequest::GetTransactionCount,
        AsRpcRequestImpl::GetVersion => RpcRequest::GetVersion,
        AsRpcRequestImpl::GetVoteAccounts => RpcRequest::GetVoteAccounts,
        AsRpcRequestImpl::IsBlockhashValid => RpcRequest::IsBlockhashValid,
        AsRpcRequestImpl::MinimumLedgerSlot => RpcRequest::MinimumLedgerSlot,
        AsRpcRequestImpl::RegisterNode => RpcRequest::RegisterNode,
        AsRpcRequestImpl::RequestAirdrop => RpcRequest::RequestAirdrop,
        AsRpcRequestImpl::SendTransaction => RpcRequest::SendTransaction,
        AsRpcRequestImpl::SimulateTransaction => RpcRequest::SimulateTransaction,
        AsRpcRequestImpl::SignVote => RpcRequest::SignVote,
    })
}

serde_conv!(pub AsRpcRequest, RpcRequest, ser_rpc_request, de_rpc_request);

#[derive(Serialize, Deserialize)]
pub enum AsRpcResponseErrorDataImpl {
    Empty,
    SendTransactionPreflightFailure(RpcSimulateTransactionResult),
    NodeUnhealthy { num_slots_behind: Option<Slot> },
}

fn ser_rpc_response_error_data(error: &RpcResponseErrorData) -> AsRpcResponseErrorDataImpl {
    match error {
        RpcResponseErrorData::Empty => AsRpcResponseErrorDataImpl::Empty,
        RpcResponseErrorData::SendTransactionPreflightFailure(rpc_simulate_transaction_result) => {
            AsRpcResponseErrorDataImpl::SendTransactionPreflightFailure(
                rpc_simulate_transaction_result.clone(),
            )
        }
        RpcResponseErrorData::NodeUnhealthy { num_slots_behind } => {
            AsRpcResponseErrorDataImpl::NodeUnhealthy {
                num_slots_behind: *num_slots_behind,
            }
        }
    }
}

fn de_rpc_response_error_data(
    error: AsRpcResponseErrorDataImpl,
) -> Result<RpcResponseErrorData, Infallible> {
    Ok(match error {
        AsRpcResponseErrorDataImpl::Empty => RpcResponseErrorData::Empty,
        AsRpcResponseErrorDataImpl::SendTransactionPreflightFailure(
            rpc_simulate_transaction_result,
        ) => RpcResponseErrorData::SendTransactionPreflightFailure(rpc_simulate_transaction_result),
        AsRpcResponseErrorDataImpl::NodeUnhealthy { num_slots_behind } => {
            RpcResponseErrorData::NodeUnhealthy { num_slots_behind }
        }
    })
}

serde_conv!(pub AsRpcResponseErrorData, RpcResponseErrorData, ser_rpc_response_error_data, de_rpc_response_error_data);

#[serde_as]
#[derive(Serialize, Deserialize)]
pub enum AsRpcErrorImpl {
    RpcRequestError(String),
    RpcResponseError {
        code: i64,
        message: String,
        #[serde_as(as = "AsRpcResponseErrorData")]
        data: RpcResponseErrorData,
    },
    ParseError(String),
    ForUser(String),
}

fn ser_rpc_error(error: &RpcError) -> AsRpcErrorImpl {
    match error {
        RpcError::RpcRequestError(error) => AsRpcErrorImpl::RpcRequestError(error.clone()),
        RpcError::RpcResponseError {
            code,
            message,
            data,
        } => AsRpcErrorImpl::RpcResponseError {
            code: *code,
            message: message.clone(),
            data: clone_rpc_response_error(data),
        },
        RpcError::ParseError(error) => AsRpcErrorImpl::ParseError(error.clone()),
        RpcError::ForUser(error) => AsRpcErrorImpl::ForUser(error.clone()),
    }
}

fn de_rpc_error(error: AsRpcErrorImpl) -> Result<RpcError, Infallible> {
    Ok(match error {
        AsRpcErrorImpl::RpcRequestError(error) => RpcError::RpcRequestError(error),
        AsRpcErrorImpl::RpcResponseError {
            code,
            message,
            data,
        } => RpcError::RpcResponseError {
            code,
            message,
            data,
        },
        AsRpcErrorImpl::ParseError(error) => RpcError::ParseError(error),
        AsRpcErrorImpl::ForUser(error) => RpcError::ForUser(error),
    })
}

serde_conv!(pub AsRpcEeror, RpcError, ser_rpc_error, de_rpc_error);

#[serde_as]
#[derive(Serialize, Deserialize)]
pub enum AsClientErrorKindImpl {
    Io(#[serde_as(as = "AsIoError")] io::Error),
    Middleware(#[serde_as(as = "AsAnyhow")] anyhow::Error),
    RpcError(#[serde_as(as = "AsRpcEeror")] RpcError),
    SigningError(#[serde_as(as = "AsSignerError")] SignerError),
    TransactionError(TransactionError),
    Custom(String),
}

#[derive(Serialize, Deserialize)]
pub enum AsErrorKindImpl {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

fn ser_error_kind(kind: &io::ErrorKind) -> AsErrorKindImpl {
    match kind {
        io::ErrorKind::NotFound => AsErrorKindImpl::NotFound,
        io::ErrorKind::PermissionDenied => AsErrorKindImpl::PermissionDenied,
        io::ErrorKind::ConnectionRefused => AsErrorKindImpl::ConnectionRefused,
        io::ErrorKind::ConnectionReset => AsErrorKindImpl::ConnectionReset,
        io::ErrorKind::HostUnreachable => AsErrorKindImpl::HostUnreachable,
        io::ErrorKind::NetworkUnreachable => AsErrorKindImpl::NetworkUnreachable,
        io::ErrorKind::ConnectionAborted => AsErrorKindImpl::ConnectionAborted,
        io::ErrorKind::NotConnected => AsErrorKindImpl::NotConnected,
        io::ErrorKind::AddrInUse => AsErrorKindImpl::AddrInUse,
        io::ErrorKind::AddrNotAvailable => AsErrorKindImpl::AddrNotAvailable,
        io::ErrorKind::NetworkDown => AsErrorKindImpl::NetworkDown,
        io::ErrorKind::BrokenPipe => AsErrorKindImpl::BrokenPipe,
        io::ErrorKind::AlreadyExists => AsErrorKindImpl::AlreadyExists,
        io::ErrorKind::WouldBlock => AsErrorKindImpl::WouldBlock,
        io::ErrorKind::NotADirectory => AsErrorKindImpl::NotADirectory,
        io::ErrorKind::IsADirectory => AsErrorKindImpl::IsADirectory,
        io::ErrorKind::DirectoryNotEmpty => AsErrorKindImpl::DirectoryNotEmpty,
        io::ErrorKind::ReadOnlyFilesystem => AsErrorKindImpl::ReadOnlyFilesystem,
        io::ErrorKind::StaleNetworkFileHandle => AsErrorKindImpl::StaleNetworkFileHandle,
        io::ErrorKind::InvalidInput => AsErrorKindImpl::InvalidInput,
        io::ErrorKind::InvalidData => AsErrorKindImpl::InvalidData,
        io::ErrorKind::TimedOut => AsErrorKindImpl::TimedOut,
        io::ErrorKind::WriteZero => AsErrorKindImpl::WriteZero,
        io::ErrorKind::StorageFull => AsErrorKindImpl::StorageFull,
        io::ErrorKind::NotSeekable => AsErrorKindImpl::NotSeekable,
        io::ErrorKind::QuotaExceeded => AsErrorKindImpl::QuotaExceeded,
        io::ErrorKind::FileTooLarge => AsErrorKindImpl::FileTooLarge,
        io::ErrorKind::ResourceBusy => AsErrorKindImpl::ResourceBusy,
        io::ErrorKind::ExecutableFileBusy => AsErrorKindImpl::ExecutableFileBusy,
        io::ErrorKind::Deadlock => AsErrorKindImpl::Deadlock,
        io::ErrorKind::CrossesDevices => AsErrorKindImpl::CrossesDevices,
        io::ErrorKind::TooManyLinks => AsErrorKindImpl::TooManyLinks,
        io::ErrorKind::InvalidFilename => AsErrorKindImpl::InvalidFilename,
        io::ErrorKind::ArgumentListTooLong => AsErrorKindImpl::ArgumentListTooLong,
        io::ErrorKind::Interrupted => AsErrorKindImpl::Interrupted,
        io::ErrorKind::Unsupported => AsErrorKindImpl::Unsupported,
        io::ErrorKind::UnexpectedEof => AsErrorKindImpl::UnexpectedEof,
        io::ErrorKind::OutOfMemory => AsErrorKindImpl::OutOfMemory,
        io::ErrorKind::Other => AsErrorKindImpl::Other,
        _ => AsErrorKindImpl::Other,
    }
}

fn de_error_kind(kind: AsErrorKindImpl) -> Result<io::ErrorKind, Infallible> {
    Ok(match kind {
        AsErrorKindImpl::NotFound => io::ErrorKind::NotFound,
        AsErrorKindImpl::PermissionDenied => io::ErrorKind::PermissionDenied,
        AsErrorKindImpl::ConnectionRefused => io::ErrorKind::ConnectionRefused,
        AsErrorKindImpl::ConnectionReset => io::ErrorKind::ConnectionReset,
        AsErrorKindImpl::HostUnreachable => io::ErrorKind::HostUnreachable,
        AsErrorKindImpl::NetworkUnreachable => io::ErrorKind::NetworkUnreachable,
        AsErrorKindImpl::ConnectionAborted => io::ErrorKind::ConnectionAborted,
        AsErrorKindImpl::NotConnected => io::ErrorKind::NotConnected,
        AsErrorKindImpl::AddrInUse => io::ErrorKind::AddrInUse,
        AsErrorKindImpl::AddrNotAvailable => io::ErrorKind::AddrNotAvailable,
        AsErrorKindImpl::NetworkDown => io::ErrorKind::NetworkDown,
        AsErrorKindImpl::BrokenPipe => io::ErrorKind::BrokenPipe,
        AsErrorKindImpl::AlreadyExists => io::ErrorKind::AlreadyExists,
        AsErrorKindImpl::WouldBlock => io::ErrorKind::WouldBlock,
        AsErrorKindImpl::NotADirectory => io::ErrorKind::NotADirectory,
        AsErrorKindImpl::IsADirectory => io::ErrorKind::IsADirectory,
        AsErrorKindImpl::DirectoryNotEmpty => io::ErrorKind::DirectoryNotEmpty,
        AsErrorKindImpl::ReadOnlyFilesystem => io::ErrorKind::ReadOnlyFilesystem,
        AsErrorKindImpl::StaleNetworkFileHandle => io::ErrorKind::StaleNetworkFileHandle,
        AsErrorKindImpl::InvalidInput => io::ErrorKind::InvalidInput,
        AsErrorKindImpl::InvalidData => io::ErrorKind::InvalidData,
        AsErrorKindImpl::TimedOut => io::ErrorKind::TimedOut,
        AsErrorKindImpl::WriteZero => io::ErrorKind::WriteZero,
        AsErrorKindImpl::StorageFull => io::ErrorKind::StorageFull,
        AsErrorKindImpl::NotSeekable => io::ErrorKind::NotSeekable,
        AsErrorKindImpl::QuotaExceeded => io::ErrorKind::QuotaExceeded,
        AsErrorKindImpl::FileTooLarge => io::ErrorKind::FileTooLarge,
        AsErrorKindImpl::ResourceBusy => io::ErrorKind::ResourceBusy,
        AsErrorKindImpl::ExecutableFileBusy => io::ErrorKind::ExecutableFileBusy,
        AsErrorKindImpl::Deadlock => io::ErrorKind::Deadlock,
        AsErrorKindImpl::CrossesDevices => io::ErrorKind::CrossesDevices,
        AsErrorKindImpl::TooManyLinks => io::ErrorKind::TooManyLinks,
        AsErrorKindImpl::InvalidFilename => io::ErrorKind::InvalidFilename,
        AsErrorKindImpl::ArgumentListTooLong => io::ErrorKind::ArgumentListTooLong,
        AsErrorKindImpl::Interrupted => io::ErrorKind::Interrupted,
        AsErrorKindImpl::Unsupported => io::ErrorKind::Unsupported,
        AsErrorKindImpl::UnexpectedEof => io::ErrorKind::UnexpectedEof,
        AsErrorKindImpl::OutOfMemory => io::ErrorKind::OutOfMemory,
        AsErrorKindImpl::Other => io::ErrorKind::Other,
    })
}

serde_conv!(pub AsErrorKind, io::ErrorKind, ser_error_kind, de_error_kind);

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct AsIoErrorImpl {
    #[serde_as(as = "AsErrorKind")]
    pub kind: io::ErrorKind,
    pub msg: String,
}

fn ser_io_error(error: &io::Error) -> AsIoErrorImpl {
    AsIoErrorImpl {
        kind: error.kind(),
        msg: error.to_string(),
    }
}

fn de_io_error(error: AsIoErrorImpl) -> Result<io::Error, Infallible> {
    Ok(io::Error::new(error.kind, error.msg))
}

serde_conv!(pub AsIoError, io::Error, ser_io_error, de_io_error);

serde_conv!(
    pub AsAnyhow,
    anyhow::Error,
    |error: &anyhow::Error| error.to_string(),
    |error: String| Ok::<_, Infallible>(anyhow::Error::msg(error))
);

#[derive(Serialize, Deserialize)]
pub enum AsPresignerErrorImpl {
    VerificationFailure,
}

fn ser_presigner_error(error: &PresignerError) -> AsPresignerErrorImpl {
    match error {
        PresignerError::VerificationFailure => AsPresignerErrorImpl::VerificationFailure,
    }
}

fn de_presigner_error(error: AsPresignerErrorImpl) -> Result<PresignerError, Infallible> {
    Ok(match error {
        AsPresignerErrorImpl::VerificationFailure => PresignerError::VerificationFailure,
    })
}

serde_conv!(
    AsPresignerError,
    PresignerError,
    ser_presigner_error,
    de_presigner_error
);

#[serde_as]
#[derive(Serialize, Deserialize)]
pub enum AsSignerErrorImpl {
    KeypairPubkeyMismatch,
    NotEnoughSigners,
    TransactionError(TransactionError),
    Custom(String),
    // Presigner-specific Errors
    PresignerError(#[serde_as(as = "AsPresignerError")] PresignerError),
    // Remote Keypair-specific Errors
    Connection(String),
    InvalidInput(String),
    NoDeviceFound,
    Protocol(String),
    UserCancel(String),
    TooManySigners,
}

fn ser_signer_error(error: &SignerError) -> AsSignerErrorImpl {
    match clone_signer_error(error) {
        SignerError::KeypairPubkeyMismatch => AsSignerErrorImpl::KeypairPubkeyMismatch,
        SignerError::NotEnoughSigners => AsSignerErrorImpl::NotEnoughSigners,
        SignerError::TransactionError(transaction_error) => {
            AsSignerErrorImpl::TransactionError(transaction_error)
        }
        SignerError::Custom(error) => AsSignerErrorImpl::Custom(error),
        SignerError::PresignerError(presigner_error) => {
            AsSignerErrorImpl::PresignerError(presigner_error)
        }
        SignerError::Connection(error) => AsSignerErrorImpl::Connection(error),
        SignerError::InvalidInput(error) => AsSignerErrorImpl::InvalidInput(error),
        SignerError::NoDeviceFound => AsSignerErrorImpl::NoDeviceFound,
        SignerError::Protocol(error) => AsSignerErrorImpl::Protocol(error),
        SignerError::UserCancel(error) => AsSignerErrorImpl::UserCancel(error),
        SignerError::TooManySigners => AsSignerErrorImpl::TooManySigners,
    }
}

fn de_signer_error(error: AsSignerErrorImpl) -> Result<SignerError, Infallible> {
    Ok(match error {
        AsSignerErrorImpl::KeypairPubkeyMismatch => SignerError::KeypairPubkeyMismatch,
        AsSignerErrorImpl::NotEnoughSigners => SignerError::NotEnoughSigners,
        AsSignerErrorImpl::TransactionError(transaction_error) => {
            SignerError::TransactionError(transaction_error)
        }
        AsSignerErrorImpl::Custom(error) => SignerError::Custom(error),
        AsSignerErrorImpl::PresignerError(presigner_error) => {
            SignerError::PresignerError(presigner_error)
        }
        AsSignerErrorImpl::Connection(error) => SignerError::Connection(error),
        AsSignerErrorImpl::InvalidInput(error) => SignerError::InvalidInput(error),
        AsSignerErrorImpl::NoDeviceFound => SignerError::NoDeviceFound,
        AsSignerErrorImpl::Protocol(error) => SignerError::Protocol(error),
        AsSignerErrorImpl::UserCancel(error) => SignerError::UserCancel(error),
        AsSignerErrorImpl::TooManySigners => SignerError::TooManySigners,
    })
}

serde_conv!(
    AsSignerError,
    SignerError,
    ser_signer_error,
    de_signer_error
);

fn clone_rpc_response_error(data: &RpcResponseErrorData) -> RpcResponseErrorData {
    match data {
        RpcResponseErrorData::Empty => RpcResponseErrorData::Empty,
        RpcResponseErrorData::SendTransactionPreflightFailure(result) => {
            RpcResponseErrorData::SendTransactionPreflightFailure(result.clone())
        }
        RpcResponseErrorData::NodeUnhealthy { num_slots_behind } => {
            RpcResponseErrorData::NodeUnhealthy {
                num_slots_behind: *num_slots_behind,
            }
        }
    }
}

fn clone_rpc_error(error: &RpcError) -> RpcError {
    match error {
        RpcError::RpcRequestError(error) => RpcError::RpcRequestError(error.clone()),
        RpcError::RpcResponseError {
            code,
            message,
            data,
        } => RpcError::RpcResponseError {
            code: *code,
            message: message.clone(),
            data: clone_rpc_response_error(data),
        },
        RpcError::ParseError(error) => RpcError::ParseError(error.clone()),
        RpcError::ForUser(error) => RpcError::ForUser(error.clone()),
    }
}

fn clone_presigner_error(error: &PresignerError) -> PresignerError {
    match error {
        PresignerError::VerificationFailure => PresignerError::VerificationFailure,
    }
}

fn clone_signer_error(error: &SignerError) -> SignerError {
    match error {
        SignerError::KeypairPubkeyMismatch => SignerError::KeypairPubkeyMismatch,
        SignerError::NotEnoughSigners => SignerError::NotEnoughSigners,
        SignerError::TransactionError(error) => SignerError::TransactionError(error.clone()),
        SignerError::Custom(error) => SignerError::Custom(error.clone()),
        SignerError::PresignerError(error) => {
            SignerError::PresignerError(clone_presigner_error(error))
        }
        SignerError::Connection(error) => SignerError::Connection(error.clone()),
        SignerError::InvalidInput(error) => SignerError::InvalidInput(error.clone()),
        SignerError::NoDeviceFound => SignerError::NoDeviceFound,
        SignerError::Protocol(error) => SignerError::Protocol(error.clone()),
        SignerError::UserCancel(error) => SignerError::UserCancel(error.clone()),
        SignerError::TooManySigners => SignerError::TooManySigners,
    }
}

fn ser_kind(kind: &ClientErrorKind) -> AsClientErrorKindImpl {
    match kind {
        ClientErrorKind::Io(error) => {
            AsClientErrorKindImpl::Io(io::Error::new(error.kind(), error.to_string()))
        }
        ClientErrorKind::Reqwest(error) => AsClientErrorKindImpl::Custom(error.to_string()),
        ClientErrorKind::Middleware(error) => {
            AsClientErrorKindImpl::Middleware(anyhow!(format!("{:#}", error)))
        }
        ClientErrorKind::RpcError(error) => AsClientErrorKindImpl::RpcError(clone_rpc_error(error)),
        ClientErrorKind::SerdeJson(error) => AsClientErrorKindImpl::Custom(error.to_string()),
        ClientErrorKind::SigningError(error) => {
            AsClientErrorKindImpl::SigningError(clone_signer_error(error))
        }
        ClientErrorKind::TransactionError(error) => {
            AsClientErrorKindImpl::TransactionError(error.clone())
        }
        ClientErrorKind::Custom(error) => AsClientErrorKindImpl::Custom(error.clone()),
    }
}

fn de_kind(kind: AsClientErrorKindImpl) -> Result<ClientErrorKind, Infallible> {
    Ok(match kind {
        AsClientErrorKindImpl::Io(error) => ClientErrorKind::Io(error),
        AsClientErrorKindImpl::Middleware(error) => ClientErrorKind::Middleware(error),
        AsClientErrorKindImpl::RpcError(rpc_error) => ClientErrorKind::RpcError(rpc_error),
        AsClientErrorKindImpl::SigningError(signer_error) => {
            ClientErrorKind::SigningError(signer_error)
        }
        AsClientErrorKindImpl::TransactionError(transaction_error) => {
            ClientErrorKind::TransactionError(transaction_error)
        }
        AsClientErrorKindImpl::Custom(error) => ClientErrorKind::Custom(error),
    })
}

serde_conv!(AsClientErrorKind, ClientErrorKind, ser_kind, de_kind);

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct AsClientErrorImpl {
    #[serde_as(as = "Option<AsRpcRequest>")]
    pub request: Option<RpcRequest>,
    #[serde_as(as = "AsClientErrorKind")]
    pub kind: ClientErrorKind,
}

fn clone_client_error_kind(kind: &ClientErrorKind) -> ClientErrorKind {
    de_kind(ser_kind(kind)).unwrap()
}

fn ser_client_error(error: &ClientError) -> AsClientErrorImpl {
    AsClientErrorImpl {
        request: error.request.clone(),
        kind: clone_client_error_kind(&error.kind),
    }
}

fn de_client_error(error: AsClientErrorImpl) -> Result<ClientError, Infallible> {
    Ok(ClientError {
        request: error.request,
        kind: error.kind,
    })
}

serde_conv!(
    AsClientError,
    ClientError,
    ser_client_error,
    de_client_error
);

#[derive(Serialize, Deserialize)]
pub enum AsCompileErrorImpl {
    AccountIndexOverflow,
    AddressTableLookupIndexOverflow,
    UnknownInstructionKey(Pubkey),
}

fn ser_CompileError(error: &CompileError) -> AsCompileErrorImpl {
    match error {
        CompileError::AccountIndexOverflow => AsCompileErrorImpl::AccountIndexOverflow,
        CompileError::AddressTableLookupIndexOverflow => {
            AsCompileErrorImpl::AddressTableLookupIndexOverflow
        }
        CompileError::UnknownInstructionKey(pubkey) => {
            AsCompileErrorImpl::UnknownInstructionKey(*pubkey)
        }
    }
}

fn de_CompileError(error: AsCompileErrorImpl) -> Result<CompileError, Infallible> {
    Ok(match error {
        AsCompileErrorImpl::AccountIndexOverflow => CompileError::AccountIndexOverflow,
        AsCompileErrorImpl::AddressTableLookupIndexOverflow => {
            CompileError::AddressTableLookupIndexOverflow
        }
        AsCompileErrorImpl::UnknownInstructionKey(pubkey) => {
            CompileError::UnknownInstructionKey(pubkey)
        }
    })
}

serde_conv!(pub AsCompileError, CompileError, ser_CompileError, de_CompileError);

#[derive(Serialize, Deserialize)]
pub enum AsSanitizeErrorImpl {
    IndexOutOfBounds,
    ValueOutOfBounds,
    InvalidValue,
}

fn ser_SanitizeError(error: &SanitizeError) -> AsSanitizeErrorImpl {
    match error {
        SanitizeError::IndexOutOfBounds => AsSanitizeErrorImpl::IndexOutOfBounds,
        SanitizeError::ValueOutOfBounds => AsSanitizeErrorImpl::ValueOutOfBounds,
        SanitizeError::InvalidValue => AsSanitizeErrorImpl::InvalidValue,
    }
}

fn de_SanitizeError(error: AsSanitizeErrorImpl) -> Result<SanitizeError, Infallible> {
    Ok(match error {
        AsSanitizeErrorImpl::IndexOutOfBounds => SanitizeError::IndexOutOfBounds,
        AsSanitizeErrorImpl::ValueOutOfBounds => SanitizeError::ValueOutOfBounds,
        AsSanitizeErrorImpl::InvalidValue => SanitizeError::InvalidValue,
    })
}

serde_conv!(pub AsSanitizeError, SanitizeError, ser_SanitizeError, de_SanitizeError);

#[derive(Serialize, Deserialize)]
pub enum AsMailboxErrorImpl {
    Closed,
    Timeout,
}

fn ser_MailboxError(error: &MailboxError) -> AsMailboxErrorImpl {
    match error {
        MailboxError::Closed => AsMailboxErrorImpl::Closed,
        MailboxError::Timeout => AsMailboxErrorImpl::Timeout,
    }
}

fn de_MailboxError(error: AsMailboxErrorImpl) -> Result<MailboxError, Infallible> {
    Ok(match error {
        AsMailboxErrorImpl::Closed => MailboxError::Closed,
        AsMailboxErrorImpl::Timeout => MailboxError::Timeout,
    })
}

serde_conv!(pub AsMailboxError, MailboxError, ser_MailboxError, de_MailboxError);
