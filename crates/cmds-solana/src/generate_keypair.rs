use crate::WalletOrPubkey;
use crate::prelude::*;
use bip39::{Language, Mnemonic, MnemonicType, Seed};
use solana_commitment_config::CommitmentConfig;
use solana_keypair::{Keypair, keypair_from_seed};

const GENERATE_KEYPAIR: &str = "generate_keypair";

const DEFINITION: &str = flow_lib::node_definition!("generate_keypair.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(GENERATE_KEYPAIR));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(GENERATE_KEYPAIR, |_| build()));

fn random_seed() -> String {
    Mnemonic::new(MnemonicType::Words12, Language::English).into_phrase()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    #[serde(default)]
    private_key: Option<WalletOrPubkey>,
    #[serde(default = "random_seed")]
    seed: String,
    #[serde(default)]
    passphrase: String,
    #[serde(default = "value::default::bool_false")]
    check_new_account: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    #[serde(with = "value::pubkey")]
    pub pubkey: Pubkey,
    pub keypair: Wallet,
}

async fn generate_keypair(
    passphrase: &str,
    seed: &str,
    check: Option<Arc<RpcClient>>,
) -> crate::Result<Keypair> {
    loop {
        let sanitized = seed.split_whitespace().collect::<Vec<&str>>().join(" ");
        let parse_language_fn = || {
            for language in &[
                Language::English,
                Language::ChineseSimplified,
                Language::ChineseTraditional,
                Language::Japanese,
                Language::Spanish,
                Language::Korean,
                Language::French,
                Language::Italian,
            ] {
                if let Ok(mnemonic) = Mnemonic::from_phrase(&sanitized, *language) {
                    return Ok(mnemonic);
                }
            }
            Err(crate::Error::CantGetMnemonicFromPhrase)
        };
        let mnemonic = parse_language_fn()?;
        let seed = Seed::new(&mnemonic, passphrase);
        let keypair = keypair_from_seed(seed.as_bytes())
            .map_err(|e| crate::Error::KeypairFromSeed(e.to_string()))?;

        match check.as_ref() {
            Some(rpc) => {
                let exists = account_exists(rpc, &keypair.pubkey()).await?;
                if exists {
                    continue;
                } else {
                    break Ok(keypair);
                }
            }
            _ => {
                break Ok(keypair);
            }
        }
    }
}

async fn account_exists(rpc: &RpcClient, pk: &Pubkey) -> Result<bool, CommandError> {
    Ok(rpc
        .get_account_with_commitment(pk, CommitmentConfig::processed())
        .await?
        .value
        .is_some())
}

async fn run(ctx: CommandContextX, input: Input) -> Result<Output, CommandError> {
    let keypair = match input.private_key {
        Some(either) => {
            let keypair = match either {
                WalletOrPubkey::Wallet(keypair) => keypair,
                WalletOrPubkey::Pubkey(public_key) => Wallet::Adapter { public_key },
            };
            if input.check_new_account
                && account_exists(ctx.solana_client(), &keypair.pubkey()).await?
            {
                return Err(CommandError::msg("account already exists"));
            }
            keypair
        }
        None => generate_keypair(
            &input.passphrase,
            &input.seed,
            input
                .check_new_account
                .then_some(ctx.solana_client().clone()),
        )
        .await?
        .into(),
    };

    Ok(Output {
        pubkey: keypair.pubkey(),
        keypair,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        build().unwrap();
    }

    #[tokio::test]
    async fn test_no_input() {
        let ctx = CommandContextX::default();
        build().unwrap().run(ctx, ValueSet::new()).await.unwrap();
    }

    #[tokio::test]
    async fn test_no_password() {
        let seed_phrase =
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above";
        let ctx = CommandContextX::default();
        build()
            .unwrap()
            .run(
                ctx,
                value::map! {
                    "seed" => Value::String(seed_phrase.to_owned()),
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_private_key_keypair() {
        let private_key = "56Ngo8EY5ZWmYKDZAmKYcUf2y2LZVRSMMnptGp9JtQuSZHyU3Pwhhkmj5YVf89VTQZqrzkabhybWdWwJWCa74aYu";
        let input = value::map! {
            "private_key" => private_key,
        };
        let output = build()
            .unwrap()
            .run(CommandContextX::default(), input)
            .await
            .unwrap();
        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(
            output.keypair.keypair().unwrap().to_base58_string(),
            private_key
        );
        assert_eq!(
            output.pubkey.to_string(),
            "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9"
        );
    }

    #[tokio::test]
    async fn test_private_key_pubkey() {
        let input = value::map! {
            "private_key" => "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9",
        };
        let output = build()
            .unwrap()
            .run(CommandContextX::default(), input)
            .await
            .unwrap();
        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(
            output.pubkey.to_string(),
            "GQZRKDqVzM4DXGGMEUNdnBD3CC4TTywh3PwgjYPBm8W9"
        );
        assert!(output.keypair.is_adapter_wallet());
        assert_eq!(output.keypair.pubkey(), output.pubkey);
    }

    #[tokio::test]
    async fn test_seed_and_pass() {
        let seed_phrase =
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above";
        let passphrase = "Hunter1!";

        let keypair = generate_keypair(passphrase, seed_phrase, None)
            .await
            .unwrap();

        let input = value::map! {
            "seed" => Value::String(seed_phrase.to_owned()),
            "passphrase" => Value::String(passphrase.to_owned()),
        };
        let output = build()
            .unwrap()
            .run(CommandContextX::default(), input)
            .await
            .unwrap();
        let output = value::from_map::<Output>(output).unwrap();
        assert_eq!(
            output.pubkey.to_string(),
            "ESxeViFP4r7THzVx9hJDkhj4HrNGSjJSFRPbGaAb97hN"
        );
        assert_eq!(
            output.keypair.keypair().unwrap().to_base58_string(),
            "3LUpzbebV5SCftt8CPmicbKxNtQhtJegEz4n8s6LBf3b1s4yfjLapgJhbMERhP73xLmWEP2XJ2Rz7Y3TFiYgTpXv"
        );
        assert_eq!(output.pubkey, keypair.pubkey());
        assert_eq!(output.keypair, Wallet::Keypair(keypair));
    }

    #[tokio::test]
    async fn test_invalid() {
        let seed_phrase =
            "letter advice cage absurd amount doctor acoustic avoid letter advice cage above";
        let passphrase = "Hunter1!";
        let private_key =
            "4rQanLxTFvdgtLsGirizXejgY5ShoZgvz4wwXi4jnii7XHSyUFJbvAk4ojRiEAHvzK6Qnjq7UyJFNbydeQ";
        let input = value::map! {
            "seed" => Value::String(seed_phrase.to_owned()),
            "passphrase" => Value::String(passphrase.to_owned()),
            "private_key" => Value::String(private_key.to_string()),
        };
        let result = build()
            .unwrap()
            .run(CommandContextX::default(), input)
            .await;
        assert!(result.is_err());
    }
}
