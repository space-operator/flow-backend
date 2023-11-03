use anchor_lang::AnchorSerialize;
use borsh::{BorshDeserialize, BorshSerialize};
use byteorder::{BigEndian, ReadBytesExt};
use flow_lib::SolanaNet;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_sdk::pubkey;
use std::io::{Cursor, Read};
use wormhole_sdk::{nft::Message as NftMessage, token::Message};

pub mod utils;

pub const WORMHOLE_CORE_MAINNET: Pubkey = pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");
pub const WORMHOLE_CORE_TESTNET: Pubkey = pubkey!("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");
pub const WORMHOLE_CORE_DEVNET: Pubkey = pubkey!("3u8hJUVTA4jH1wYAyUur7FFZVQ8H635K3tSHHF4ssjQ5");

pub const fn wormhole_core_program_id(net: SolanaNet) -> Pubkey {
    match net {
        SolanaNet::Mainnet => crate::wormhole::WORMHOLE_CORE_MAINNET,
        // TODO testnet not deployed yet
        SolanaNet::Testnet => crate::wormhole::WORMHOLE_CORE_TESTNET,
        SolanaNet::Devnet => crate::wormhole::WORMHOLE_CORE_DEVNET,
    }
}

pub const TOKEN_BRIDGE_MAINNET: Pubkey = pubkey!("wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb");
pub const TOKEN_BRIDGE_TESTNET: Pubkey = pubkey!("DZnkkTmCiFWfYTfT41X3Rd1kDgozqzxWaHqsw6W4x2oe");
pub const TOKEN_BRIDGE_DEVNET: Pubkey = pubkey!("DZnkkTmCiFWfYTfT41X3Rd1kDgozqzxWaHqsw6W4x2oe");

pub const fn token_bridge_program_id(net: SolanaNet) -> Pubkey {
    match net {
        SolanaNet::Mainnet => TOKEN_BRIDGE_MAINNET,
        // TODO testnet not deployed yet
        SolanaNet::Testnet => TOKEN_BRIDGE_TESTNET,
        SolanaNet::Devnet => TOKEN_BRIDGE_DEVNET,
    }
}

pub const NFT_BRIDGE_MAINNET: Pubkey = pubkey!("WnFt12ZrnzZrFZkt2xsNsaNWoQribnuQ5B5FrDbwDhD");
pub const NFT_BRIDGE_TESTNET: Pubkey = pubkey!("2rHhojZ7hpu1zA91nvZmT8TqWWvMcKmmNBCr2mKTtMq4");
pub const NFT_BRIDGE_DEVNET: Pubkey = pubkey!("2rHhojZ7hpu1zA91nvZmT8TqWWvMcKmmNBCr2mKTtMq4");

pub const fn nft_bridge_program_id(net: SolanaNet) -> Pubkey {
    match net {
        SolanaNet::Mainnet => NFT_BRIDGE_MAINNET,
        // TODO testnet not deployed yet
        SolanaNet::Testnet => NFT_BRIDGE_TESTNET,
        SolanaNet::Devnet => NFT_BRIDGE_DEVNET,
    }
}

pub mod nft_bridge;
pub mod token_bridge;

pub mod get_vaa;
pub mod parse_vaa;
pub mod post_message;
pub mod post_vaa;
pub mod verify_signatures;

#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize)]
pub enum WormholeInstructions {
    Initialize,
    PostMessage,
    PostVAA,
    SetFees,
    TransferFees,
    UpgradeContract,
    UpgradeGuardianSet,
    VerifySignatures,
    PostMessageUnreliable,
}

#[derive(AnchorSerialize, Deserialize, Serialize)]
pub struct PostMessageData {
    /// Unique nonce for this message
    pub nonce: u32,

    /// Message payload
    pub payload: Vec<u8>,

    /// Commitment Level required for an attestation to be produced
    pub consistency_level: ConsistencyLevel,
}

#[repr(u8)]
#[derive(AnchorSerialize, Clone, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    Confirmed,
    Finalized,
}

#[derive(Clone, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BridgeData {
    /// The current guardian set index, used to decide which signature sets to accept.
    pub guardian_set_index: u32,

    /// Lamports in the collection account
    pub last_lamports: u64,

    /// Bridge configuration, which is set once upon initialization.
    pub config: BridgeConfig,
}

#[derive(Clone, Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Period for how long a guardian set is valid after it has been replaced by a new one.  This
    /// guarantees that VAAs issued by that set can still be submitted for a certain period.  In
    /// this period we still trust the old guardian set.
    pub guardian_set_expiration_time: u32,

    /// Amount of lamports that needs to be paid to the protocol to post a message
    pub fee: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct WormholeResponse {
    data: WormholeData,
    pagination: WormholePagination,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WormholeData {
    sequence: u64,
    id: String,
    version: u64,
    emitter_chain: u64,
    emitter_addr: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::unwrap_or_skip"
    )]
    emitter_native_addr: Option<String>,
    guardian_set_index: u64,
    vaa: String,
    timestamp: String,
    updated_at: String,
    indexed_at: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "::serde_with::rust::unwrap_or_skip"
    )]
    tx_hash: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct WormholePagination {
    next: String,
}

// // Structs for API VAA parsing
// #[derive(Serialize, Deserialize, Debug)]
// struct GuardianSignature {
//     index: u8,
//     signature: Vec<u8>,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct ParsedVaa {
//     version: u8,
//     guardian_set_index: u32,
//     guardian_signatures: Vec<GuardianSignature>,
//     timestamp: u32,
//     nonce: u32,
//     emitter_chain: u16,
//     emitter_address: [u8; 32],
//     sequence: u64,
//     consistency_level: u8,
//     payload: Vec<u8>,
// }

/// Type representing an Ethereum style public key for Guardians.
pub type GuardianPublicKey = [u8; 20];

#[derive(Default, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct GuardianSetData {
    /// Index representing an incrementing version number for this guardian set.
    pub index: u32,

    /// ETH style public keys
    pub keys: Vec<GuardianPublicKey>,

    /// Timestamp representing the time this guardian became active.
    pub creation_time: u32,

    /// Expiration time when VAAs issued by this set are no longer valid.
    pub expiration_time: u32,
}

pub struct SignatureItem {
    pub signature: Vec<u8>,
    pub key: [u8; 20],
    pub index: u8,
}

const MAX_LEN_GUARDIAN_KEYS: usize = 19;

#[derive(Default, BorshSerialize, BorshDeserialize)]
pub struct VerifySignaturesData {
    /// instruction indices of signers (-1 for missing)
    pub signers: [i8; MAX_LEN_GUARDIAN_KEYS],
}

pub type ForeignAddress = [u8; 32];

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct VAASignature {
    pub signature: Vec<u8>,
    pub guardian_index: u8,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct VAA {
    // Header part
    pub version: u8,
    pub guardian_set_index: u32,
    pub signatures: Vec<VAASignature>,
    // Body part
    pub timestamp: u32,
    pub nonce: u32,
    pub emitter_chain: u16,
    pub emitter_address: ForeignAddress,
    pub sequence: u64,
    pub consistency_level: u8,
    pub payload: Vec<u8>,
}

impl VAA {
    pub const HEADER_LEN: usize = 6;
    pub const SIGNATURE_LEN: usize = 66;

    pub fn deserialize(data: &[u8]) -> std::result::Result<VAA, std::io::Error> {
        let mut rdr = Cursor::new(data);

        let version = rdr.read_u8()?;
        let guardian_set_index = rdr.read_u32::<BigEndian>()?;

        let len_sig = rdr.read_u8()?;
        let mut signatures: Vec<VAASignature> = Vec::with_capacity(len_sig as usize);
        for _i in 0..len_sig {
            let guardian_index = rdr.read_u8()?;
            let mut signature_data = [0u8; 65];
            rdr.read_exact(&mut signature_data)?;
            let signature = signature_data.to_vec();

            signatures.push(VAASignature {
                guardian_index,
                signature,
            });
        }

        let timestamp = rdr.read_u32::<BigEndian>()?;
        let nonce = rdr.read_u32::<BigEndian>()?;
        let emitter_chain = rdr.read_u16::<BigEndian>()?;

        let mut emitter_address = [0u8; 32];
        rdr.read_exact(&mut emitter_address)?;

        let sequence = rdr.read_u64::<BigEndian>()?;
        let consistency_level = rdr.read_u8()?;

        let mut payload = Vec::new();
        rdr.read_to_end(&mut payload)?;

        Ok(VAA {
            version,
            guardian_set_index,
            signatures,
            timestamp,
            nonce,
            emitter_chain,
            emitter_address,
            sequence,
            consistency_level,
            payload,
        })
    }
}

#[derive(Default, BorshSerialize, BorshDeserialize, Clone, Serialize, Deserialize, Debug)]
pub struct PostVAAData {
    // Header part
    pub version: u8,
    pub guardian_set_index: u32,

    // Body part
    pub timestamp: u32,
    pub nonce: u32,
    pub emitter_chain: u16,
    pub emitter_address: ForeignAddress,
    pub sequence: u64,
    pub consistency_level: u8,
    pub payload: Vec<u8>,
}
impl From<VAA> for PostVAAData {
    fn from(vaa: VAA) -> Self {
        PostVAAData {
            version: vaa.version,
            guardian_set_index: vaa.guardian_set_index,
            timestamp: vaa.timestamp,
            nonce: vaa.nonce,
            emitter_chain: vaa.emitter_chain,
            emitter_address: vaa.emitter_address,
            sequence: vaa.sequence,
            consistency_level: vaa.consistency_level,
            payload: vaa.payload,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageAlias {
    Transfer(Message),
    NftTransfer(NftMessage),
}
