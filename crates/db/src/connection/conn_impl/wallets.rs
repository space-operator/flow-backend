use crate::{EncryptedWallet, config::Encrypted, local_storage::CacheBucket};
use utils::bs58_decode;

use super::super::*;

struct EncryptedWalletCache;

impl CacheBucket for EncryptedWalletCache {
    type Key = UserId;
    type EncodedKey = kv::Raw;
    type Object = Vec<EncryptedWallet>;

    fn name() -> &'static str {
        "EncryptedWalletCache"
    }

    fn can_read(_: &Self::Object, _: &UserId) -> bool {
        true
    }

    fn encode_key(key: &Self::Key) -> Self::EncodedKey {
        key.as_bytes().into()
    }

    fn cache_time() -> Duration {
        Duration::from_secs(10)
    }
}

fn parse_encrypted_wallet(r: Row) -> Result<EncryptedWallet, Error> {
    let pubkey_str = r
        .try_get::<_, String>(0)
        .map_err(Error::data("wallets.public_key"))?;
    let pubkey = bs58_decode(&pubkey_str).map_err(Error::parsing("wallets.public_key"))?;

    let encrypted_keypair = r
        .try_get::<_, Option<Json<Encrypted>>>(1)
        .map_err(Error::data("wallets.encrypted_keypair"))?
        .map(|json| json.0);

    let id = r.try_get(2).map_err(Error::data("wallets.id"))?;

    Ok(EncryptedWallet {
        id,
        pubkey,
        encrypted_keypair,
    })
}

impl UserConnection {
    pub(crate) async fn get_encrypted_wallet_by_pubkey(
        &self,
        pubkey: &[u8; 32],
    ) -> crate::Result<EncryptedWallet> {
        let pubkey_str = bs58::encode(pubkey).into_string();
        let conn = self.pool.get_conn().await?;
        parse_encrypted_wallet(
            conn.do_query_one(
                "select public_key, encrypted_keypair, id
                from wallets where user_id = $1 and public_key = $2",
                &[&self.user_id, &pubkey_str],
            )
            .await
            .map_err(Error::exec("select wallet"))?,
        )
    }

    pub(crate) async fn get_encrypted_wallets_impl(&self) -> crate::Result<Vec<EncryptedWallet>> {
        if let Some(cached) = self
            .local
            .get_cache::<EncryptedWalletCache>(&self.user_id, &self.user_id)
        {
            return Ok(cached);
        }
        let result = self.get_encrypted_wallets_query_impl().await;
        if let Ok(result) = &result
            && let Err(error) = self
                .local
                .set_cache::<EncryptedWalletCache>(&self.user_id, result.clone())
        {
            tracing::error!("set_cache error: {}", error);
        }
        result
    }

    pub(crate) async fn get_some_wallets_impl(
        &self,
        ids: &[i64],
    ) -> crate::Result<Vec<EncryptedWallet>> {
        let conn = self.pool.get_conn().await?;
        let result = conn
            .do_query(
                "select public_key, encrypted_keypair, id from wallets
                where id = any($1::bigint[]) and user_id = $2",
                &[&ids, &self.user_id],
            )
            .await
            .map_err(Error::exec("select wallets"))?
            .into_iter()
            .map(parse_encrypted_wallet)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }

    pub(crate) async fn get_encrypted_wallets_query_impl(
        &self,
    ) -> crate::Result<Vec<EncryptedWallet>> {
        let conn = self.pool.get_conn().await?;
        let result = conn
            .do_query(
                "SELECT public_key, encrypted_keypair, id FROM wallets WHERE user_id = $1",
                &[&self.user_id],
            )
            .await
            .map_err(Error::exec("get wallets"))?
            .into_iter()
            .map(parse_encrypted_wallet)
            .collect::<crate::Result<Vec<EncryptedWallet>>>()?;

        Ok(result)
    }
}
