use std::error::Error as StdError;
use std::result::Result as StdResult;
use thiserror::Error as ThisError;

pub type BoxedError = Box<dyn StdError + Send + Sync>;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Any(#[from] anyhow::Error),
    #[error("{}", flow_lib::solana::verbose_solana_error(.0))]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error(transparent)]
    SolanaProgram(#[from] solana_sdk::program_error::ProgramError),
    #[error(transparent)]
    Signer(#[from] solana_sdk::signer::SignerError),
    #[error(transparent)]
    Value(#[from] value::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Bundlr(#[from] bundlr_sdk::error::BundlrError),
    #[error("solana error: associated token account doesn't exist")]
    AssociatedTokenAccountDoesntExist,
    #[error("bundlr isn't available on solana testnet")]
    BundlrNotAvailableOnTestnet,
    #[error("bundlr api returned an invalid response: {0}")]
    BundlrApiInvalidResponse(String),
    #[error("failed to register funding tx to bundlr. tx_id={0};")]
    BundlrTxRegisterFailed(String),
    #[error("can't get mnemonic from phrase")]
    CantGetMnemonicFromPhrase,
    #[error("mime type not found")]
    MimeTypeNotFound,
    #[error("failed to get keypair from seed: {0}")]
    KeypairFromSeed(String),
    #[error("solana error: unsupported recipient address: {0}")]
    UnsupportedRecipientAddress(String),
    #[error("solana error: recipient address not funded")]
    RecipientAddressNotFunded,
    #[error("specified account: {0} isn't a token account")]
    NotTokenAccount(solana_sdk::pubkey::Pubkey),
    #[error("insufficient solana balance, needed={needed}; have={balance};")]
    InsufficientSolanaBalance { needed: u64, balance: u64 },
    #[error("failed to snapshot mints: {0}")]
    ErrorSnapshottingMints(String),
    #[error("failed to fetch mint snapshot")]
    FailedToFetchMintSnapshot,
    #[error("worker stopped")]
    WorkerStopped,
    #[error("time-out waiting for signature")]
    SignatureTimeout,
    #[error("an error occured while running rhai expression: {0}")]
    RhaiExecutionError(String),
    #[error("value not found in field \"{0}\"")]
    ValueNotFound(String),
}

impl Error {
    pub fn custom<E: Into<anyhow::Error>>(e: E) -> Self {
        Error::Any(e.into())
    }
}
