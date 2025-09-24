use std::convert::Infallible;
use std::io;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};
use solana_rpc_client_api::client_error::{Error as ClientError, ErrorKind as ClientErrorKind};
use solana_rpc_client_api::request::{RpcError, RpcRequest};
use solana_signer::SignerError;
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

#[serde_as]
#[derive(Serialize, Deserialize)]
pub enum AsClientErrorKindImpl {
    Io(#[serde_as(as = "AsIoError")] io::Error),
    // Reqwest(reqwest::Error),
    Middleware(#[serde_as(as = "AsAnyhow")] anyhow::Error),
    RpcError(#[serde_as(as = "AsRpcEeror")] RpcError),
    // SerdeJson(serde_json::error::Error),
    SigningError(#[serde_as(as = "AsSignerError")] SignerError),
    TransactionError(TransactionError),
    Custom(String),
}

serde_conv!(AsClientErrorKind, ClientErrorKind, || {}, || {});

#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct AsClientErrorImpl {
    #[serde_as(as = "Option<AsRpcRequest>")]
    pub request: Option<RpcRequest>,
    #[serde_as(as = "AsClientErrorKind")]
    pub kind: ClientErrorKind,
}

serde_conv!(AsClientError, ClientError, || {}, || {});
