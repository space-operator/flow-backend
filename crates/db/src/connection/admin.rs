use crate::{Error, FlowRunLogsRow};
use anyhow::anyhow;
use bytes::Bytes;
use chrono::Utc;
use deadpool_postgres::{Object as Connection, Transaction};
use flow_lib::{FlowRunId, UserId};
use futures_util::SinkExt;
use rand::distributions::{Alphanumeric, DistString};
use std::borrow::Borrow;
use tokio_pg_mapper::PostgresMapper;
use tokio_postgres::{
    binary_copy::BinaryCopyInWriter,
    types::{Json, Type},
};
use uuid::Uuid;
use value::Value;

use super::{csv_export, ExportedUserData};

pub struct AdminConn {
    pub conn: Connection,
}

#[derive(Debug)]
pub struct Password {
    pub user_id: Uuid,
    pub email: String,
    pub password: Option<String>,
}

#[derive(Debug)]
pub struct FlowRunInfo {
    pub user_id: UserId,
    pub shared_with: Vec<UserId>,
}

impl AdminConn {
    pub fn new(conn: Connection) -> AdminConn {
        Self { conn }
    }

    pub async fn get_flow_run_info(&self, run_id: FlowRunId) -> crate::Result<FlowRunInfo> {
        let stmt = self
            .conn
            .prepare_cached("SELECT user_id FROM flow_run WHERE id = $1")
            .await
            .map_err(Error::exec("prepare"))?;
        let user_id: UserId = self
            .conn
            .query_one(&stmt, &[&run_id])
            .await
            .map_err(Error::exec("query flow_run table"))?
            .try_get(0)
            .map_err(Error::data("flow_run.user_id"))?;

        let stmt = self
            .conn
            .prepare_cached("SELECT user_id FROM flow_run_shared WHERE flow_run_id = $1")
            .await
            .map_err(Error::exec("prepare"))?;
        let shared_with = self
            .conn
            .query(&stmt, &[&run_id])
            .await
            .map_err(Error::exec("query flow_run_shared"))?
            .into_iter()
            .map(|row| row.try_get(0))
            .collect::<Result<Vec<UserId>, _>>()
            .map_err(Error::data("flow_run_shared.user_id"))?;
        Ok(FlowRunInfo {
            user_id,
            shared_with,
        })
    }

    pub async fn get_flow_run_output(&self, run_id: FlowRunId) -> crate::Result<Value> {
        let stmt = self
            .conn
            .prepare_cached("SELECT output FROM flow_run WHERE id = $1")
            .await
            .map_err(Error::exec("prepare"))?;
        let output = self
            .conn
            .query_one(&stmt, &[&run_id])
            .await
            .map_err(Error::exec("query flow_run"))?
            .try_get::<_, Json<Value>>(0)
            .map_err(Error::data("flow_run.output"))?
            .0;
        Ok(output)
    }

    pub async fn insert_whitelist(&self, pk_bs58: &str) -> crate::Result<()> {
        let stmt = self
            .conn
            .prepare_cached(
                "INSERT INTO pubkey_whitelists (pubkey, info) VALUES ($1, $2)
                ON CONFLICT (pubkey) DO NOTHING",
            )
            .await
            .map_err(Error::exec("prepare insert_whitelist"))?;
        let info = format!("inserted at {}", Utc::now());
        self.conn
            .execute(&stmt, &[&pk_bs58, &info])
            .await
            .map_err(Error::exec("insert_whitelist"))?;
        Ok(())
    }

    pub async fn get_natives_commands(self) -> crate::Result<Vec<String>> {
        self.conn
            .query(
                r#"SELECT data->>'node_id' FROM nodes WHERE type = 'native' AND "isPublic""#,
                &[],
            )
            .await
            .map_err(Error::exec("get_natives_commands"))?
            .into_iter()
            .map(|r| r.try_get::<_, String>(0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::data("nodes.data->>'node_id'"))
    }

    pub async fn copy_in_flow_run_logs<I>(&self, rows: I) -> crate::Result<u64>
    where
        I: IntoIterator,
        I::Item: Borrow<FlowRunLogsRow>,
    {
        let stmt = self
            .conn
            .prepare_cached(
                "COPY flow_run_logs (
                    user_id,
                    flow_run_id,
                    log_index,
                    node_id,
                    times,
                    time,
                    log_level,
                    content,
                    module
                ) FROM STDIN WITH (FORMAT binary)",
            )
            .await
            .map_err(Error::exec("prepare copy_in_flow_run_logs"))?;
        let sink = self
            .conn
            .copy_in::<_, Bytes>(&stmt)
            .await
            .map_err(Error::exec("start copy_in_flow_run_logs"))?;
        let writer = BinaryCopyInWriter::new(
            sink,
            &[
                Type::UUID,      // user_id
                Type::UUID,      // flow_run_id
                Type::INT4,      // log_index
                Type::UUID,      // node_id
                Type::INT4,      // times
                Type::TIMESTAMP, // time
                Type::VARCHAR,   // log_level
                Type::TEXT,      // content
                Type::TEXT,      // module
            ],
        );
        futures_util::pin_mut!(writer);
        let mut size = 0;
        for row in rows {
            let r = row.borrow();
            writer
                .as_mut()
                .write(&[
                    &r.user_id,
                    &r.flow_run_id,
                    &r.log_index,
                    &r.node_id,
                    &r.times,
                    &r.time.naive_utc(),
                    &r.log_level,
                    &r.content,
                    &r.module,
                ])
                .await
                .map_err(Error::exec("write copy_in_flow_run_logs"))?;
            size += 1;
        }
        let inserted = writer
            .as_mut()
            .finish()
            .await
            .map_err(Error::exec("finish copy_in_flow_run_logs"))?;
        if inserted != size {
            return Err(Error::LogicError(anyhow!(
                "size={}, inserted={}",
                size,
                inserted
            )));
        }
        Ok(inserted)
    }

    pub async fn get_or_reset_password(
        &mut self,
        user_id: &UserId,
    ) -> crate::Result<Option<Password>> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("start"))?;

        let stmt = tx
            .prepare_cached(
                "SELECT t1.email, t2.password
                FROM auth.users AS t1
                LEFT JOIN auth.passwords AS t2 ON t1.id = t2.user_id
                WHERE t1.id = $1",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let password = tx
            .query_opt(&stmt, &[user_id])
            .await
            .map_err(Error::exec("get_password"))?
            .map(|r| {
                Ok::<_, crate::Error>(Password {
                    user_id: *user_id,
                    email: r.try_get(0).map_err(Error::data("email"))?,
                    password: r.try_get(1).map_err(Error::data("password"))?,
                })
            })
            .transpose()?;
        if let Some(mut password) = password {
            if password.password.is_none() {
                let pw = Alphanumeric.sample_string(&mut rand::thread_rng(), 24);
                let hash = bcrypt::hash(&pw, 10).map_err(|_| Error::Bcrypt)?;

                let stmt = tx
                    .prepare_cached("UPDATE auth.users SET encrypted_password = $1 WHERE id = $2")
                    .await
                    .map_err(Error::exec("prepare"))?;
                tx.execute(&stmt, &[&hash, &password.user_id])
                    .await
                    .map_err(Error::exec("reset_pw_hash"))?;

                let stmt = tx
                    .prepare_cached(
                        "INSERT INTO auth.passwords (user_id, password) VALUES ($1, $2)
                    ON CONFLICT (user_id) DO UPDATE SET password = $2",
                    )
                    .await
                    .map_err(Error::exec("prepare"))?;
                tx.execute(&stmt, &[&password.user_id, &pw])
                    .await
                    .map_err(Error::exec("reset_pw_pass"))?;

                tx.commit().await.map_err(Error::exec("reset_pw_commit"))?;
                password.password = Some(pw);
                Ok(Some(password))
            } else {
                tx.commit().await.map_err(Error::exec("reset_pw_commit"))?;
                Ok(Some(password))
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_password(&self, pk_bs58: &str) -> crate::Result<Option<Password>> {
        let stmt = self
            .conn
            .prepare_cached(
                "SELECT t2.id, t2.email, t3.password
                FROM users_public AS t1
                INNER JOIN auth.users AS t2 ON t1.user_id = t2.id
                LEFT JOIN auth.passwords AS t3 ON t1.user_id = t3.user_id
                WHERE t1.pub_key = $1",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        self.conn
            .query_opt(&stmt, &[&pk_bs58])
            .await
            .map_err(Error::exec("get_password"))?
            .map(|r| {
                Ok(Password {
                    user_id: r.try_get(0).map_err(Error::data("user_id"))?,
                    email: r.try_get(1).map_err(Error::data("email"))?,
                    password: r.try_get(2).map_err(Error::data("password"))?,
                })
            })
            .transpose()
    }

    pub async fn reset_password(&mut self, user_id: &UserId, pw: &str) -> crate::Result<()> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("reset_pw_start"))?;

        reset_password(&tx, user_id, pw).await?;

        tx.commit().await.map_err(Error::exec("reset_pw_commit"))?;

        Ok(())
    }

    pub async fn create_store(&mut self, user_id: &UserId, store_name: &str) -> crate::Result<()> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("begin create_store"))?;

        let stmt = tx
            .prepare_cached(
                "INSERT INTO
                kvstore_metadata (user_id, store_name)
                VALUES ($1, $2)",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        tx.execute(&stmt, &[user_id, &store_name])
            .await
            .map_err(Error::exec("insert kvstore_metadata"))?;

        let stmt = tx
            .prepare_cached(
                "INSERT INTO user_quotas
                (user_id, kvstore_count, kvstore_size)
                VALUES ($1, 1, $2)
                ON CONFLICT (user_id) DO UPDATE
                SET kvstore_count = user_quotas.kvstore_count + 1,
                    kvstore_size = user_quotas.kvstore_size + $2
                WHERE user_quotas.kvstore_count + 1 <= user_quotas.kvstore_count_limit
                    AND user_quotas.kvstore_size + $2 <= user_quotas.kvstore_size_limit
                RETURNING 0",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        tx.query_one(&stmt, &[user_id, &(store_name.len() as i64)])
            .await
            .map_err(Error::exec("update user_quotas"))?;

        tx.commit()
            .await
            .map_err(Error::exec("commit create_store"))?;

        Ok(())
    }

    pub async fn delete_store(
        &mut self,
        user_id: &UserId,
        store_name: &str,
    ) -> crate::Result<bool> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("begin delete_store"))?;

        let stmt = tx
            .prepare_cached(
                "DELETE FROM kvstore_metadata
                WHERE user_id = $1 AND store_name = $2
                RETURNING stats_size",
            )
            .await
            .map_err(Error::exec("prepare"))?;
        let res = tx
            .query_opt(&stmt, &[user_id, &store_name])
            .await
            .map_err(Error::exec("delete_store"))?;
        let deleted = res.is_some();
        if let Some(row) = res {
            let size: i64 = row
                .try_get(0)
                .map_err(Error::data("kvstore_metadata.stats_size"))?;
            let size = size + store_name.len() as i64;
            let stmt = tx
                .prepare_cached(
                    "UPDATE user_quotas
                    SET kvstore_size = kvstore_size - $2,
                        kvstore_count = kvstore_count - 1
                    WHERE user_id = $1
                    RETURNING 0",
                )
                .await
                .map_err(Error::exec("prepare"))?;
            tx.query_one(&stmt, &[user_id, &size])
                .await
                .map_err(Error::exec("update user_quotas"))?;
        }

        tx.commit()
            .await
            .map_err(Error::exec("commit delete_store"))?;

        Ok(deleted)
    }

    pub async fn insert_or_replace_item(
        &mut self,
        user_id: &UserId,
        store_name: &str,
        key: &str,
        value: &Value,
    ) -> crate::Result<Option<Value>> {
        let json = serde_json::value::to_raw_value(value).map_err(Error::json("json serialize"))?;
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("insert_item start"))?;

        let stmt = tx
            .prepare_cached(
                "SELECT LENGTH(value::TEXT) + LENGTH(key), value FROM kvstore
                WHERE user_id = $1
                    AND store_name = $2
                    AND key = $3",
            )
            .await
            .map_err(Error::exec("prepare get existing value"))?;
        let (old_size, old_value) = tx
            .query_opt(&stmt, &[user_id, &store_name, &key])
            .await
            .map_err(Error::exec("get existing value"))?
            .map(|row| {
                (
                    row.try_get::<_, i32>(0),
                    row.try_get::<_, Json<Value>>(1).map(|v| Some(v.0)),
                )
            })
            .map(|(r1, r2)| r1.and_then(|r1| Ok((r1, r2?))))
            .transpose()
            .map_err(Error::data("parse value"))?
            .unwrap_or((0, None));

        let stmt = tx
            .prepare_cached(
                "INSERT INTO kvstore (user_id, store_name, key, value)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (user_id, store_name, key)
                DO UPDATE SET value = $4
                RETURNING LENGTH(value::text) + LENGTH(key)",
            )
            .await
            .map_err(Error::exec("prepare update kvstore"))?;
        let new_size: i32 = tx
            .query_one(&stmt, &[user_id, &store_name, &key, &Json(&json)])
            .await
            .map_err(Error::exec("update kvstore"))?
            .try_get::<_, i32>(0)
            .map_err(Error::data("INTEGER"))?;

        let changed = (new_size - old_size) as i64;

        let stmt = tx
            .prepare_cached(
                "UPDATE user_quotas
                SET kvstore_size = kvstore_size + $2
                WHERE
                    user_id = $1
                    AND kvstore_size + $2 < kvstore_size_limit
                RETURNING 0",
            )
            .await
            .map_err(Error::exec("prepare update user_quotas"))?;
        tx.query_one(&stmt, &[user_id, &changed])
            .await
            .map_err(Error::exec("update user_quotas"))?;

        let stmt = tx
            .prepare_cached(
                "UPDATE kvstore_metadata
                SET stats_size = stats_size + $3
                WHERE
                    user_id = $1 AND store_name = $2
                RETURNING 0",
            )
            .await
            .map_err(Error::exec("prepare update kvstore_metadata"))?;
        tx.query_one(&stmt, &[user_id, &store_name, &changed])
            .await
            .map_err(Error::exec("update kvstore_metadata"))?;

        tx.commit()
            .await
            .map_err(Error::exec("insert_item commit"))?;
        Ok(old_value)
    }

    pub async fn remove_item(
        &mut self,
        user_id: &UserId,
        store_name: &str,
        key: &str,
    ) -> crate::Result<Value> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("remove_item start"))?;

        let stmt = tx
            .prepare_cached(
                "DELETE FROM kvstore
                WHERE user_id = $1
                    AND store_name = $2
                    AND key = $3
                RETURNING LENGTH(value::TEXT) + LENGTH(key), value",
            )
            .await
            .map_err(Error::exec("prepare get existing value"))?;
        let (old_size, old_value) = tx
            .query_one(&stmt, &[user_id, &store_name, &key])
            .await
            .and_then(|row| {
                Ok((
                    row.try_get::<_, i32>(0)?,
                    row.try_get::<_, Json<Value>>(1).map(|v| v.0)?,
                ))
            })
            .map_err(Error::exec("remove_item"))?;

        let old_size = old_size as i64;

        let stmt = tx
            .prepare_cached(
                "UPDATE user_quotas
                SET kvstore_size = kvstore_size - $2
                WHERE
                    user_id = $1
                RETURNING 0",
            )
            .await
            .map_err(Error::exec("prepare update user_quotas"))?;
        tx.query_one(&stmt, &[user_id, &old_size])
            .await
            .map_err(Error::exec("update user_quotas"))?;

        let stmt = tx
            .prepare_cached(
                "UPDATE kvstore_metadata
                SET stats_size = stats_size - $3
                WHERE
                    user_id = $1 AND store_name = $2
                RETURNING 0",
            )
            .await
            .map_err(Error::exec("prepare update kvstore_metadata"))?;
        tx.query_one(&stmt, &[user_id, &store_name, &old_size])
            .await
            .map_err(Error::exec("update kvstore_metadata"))?;

        tx.commit()
            .await
            .map_err(Error::exec("remove_item commit"))?;
        Ok(old_value)
    }

    pub async fn import_data(&mut self, data: ExportedUserData) -> crate::Result<()> {
        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("start"))?;

        tx.execute("SELECT auth.disable_users_triggers()", &[])
            .await
            .map_err(Error::exec("disable trigger"))?;

        tx.execute("CREATE TEMP TABLE tmp_table (LIKE pubkey_whitelists INCLUDING DEFAULTS) ON COMMIT DROP", &[]).await.map_err(Error::exec("create temp table"))?;
        copy_in(&tx, "tmp_table", data.pubkey_whitelists).await?;
        tx.execute(
            "INSERT INTO pubkey_whitelists
                SELECT * FROM tmp_table
                ON CONFLICT DO NOTHING;",
            &[],
        )
        .await
        .map_err(Error::exec("bulk insert"))?;

        copy_in(&tx, "auth.users", data.users).await?;
        tx.execute(
            "UPDATE auth.users
            SET
                confirmation_token = '',
                recovery_token = '',
                email_change_token_new = '',
                email_change = '',
                email_change_token_current = '',
                reauthentication_token = '',
                phone_change = '',
                phone_change_token = ''
            WHERE id = $1
            ",
            &[&data.user_id],
        )
        .await
        .map_err(Error::exec("fix users row"))?;

        copy_in(&tx, "auth.identities", data.identities).await?;
        copy_in(&tx, "users_public", data.users_public).await?;

        copy_in(&tx, "wallets", data.wallets).await?;
        update_id_sequence(&tx, "wallets", "id", "wallets_id_seq").await?;

        copy_in(&tx, "apikeys", data.apikeys).await?;
        copy_in(&tx, "user_quotas", data.user_quotas).await?;
        copy_in(&tx, "kvstore_metadata", data.kvstore_metadata).await?;
        copy_in(&tx, "kvstore", data.kvstore).await?;

        copy_in(&tx, "flows", data.flows).await?;
        update_id_sequence(&tx, "flows", "id", "flows_id_seq").await?;

        copy_in(&tx, "nodes", data.nodes).await?;
        update_id_sequence(&tx, "nodes", "id", "nodes_id_seq").await?;

        let user_id = data.user_id;
        let pw = rand_password();
        reset_password(&tx, &user_id, &pw).await?;

        tx.execute("SELECT auth.enable_users_triggers()", &[])
            .await
            .map_err(Error::exec("enable trigger"))?;

        tx.commit().await.map_err(Error::exec("commit"))?;

        Ok(())
    }
}

async fn update_id_sequence(
    tx: &Transaction<'_>,
    table: &str,
    column: &str,
    sequence_name: &str,
) -> crate::Result<()> {
    let query = format!(
        "SELECT setval('{}', (SELECT max({}) FROM {}), TRUE)",
        sequence_name, column, table
    );
    let stmt = tx
        .prepare_cached(&query)
        .await
        .map_err(Error::exec("prepare"))?;
    tx.execute(&stmt, &[])
        .await
        .map_err(Error::exec("update sequence"))?;
    Ok(())
}

async fn copy_in(tx: &Transaction<'_>, table: &str, data: String) -> crate::Result<()> {
    let header = csv_export::reader()
        .from_reader(data.as_bytes())
        .headers()
        .map_err(Error::parsing("csv"))?
        .into_iter()
        .fold(String::new(), |mut r, header| {
            if !r.is_empty() {
                r.push(',');
            }
            std::fmt::write(&mut r, format_args!("{:?}", header)).unwrap();
            r
        });
    let query = format!(
        "COPY {} ({}) FROM stdin WITH (FORMAT csv, DELIMITER ';', QUOTE '''', HEADER MATCH)",
        table, header
    );
    let sink = tx
        .copy_in::<_, Bytes>(&query)
        .await
        .map_err(Error::exec("copy-in users"))?;
    futures_util::pin_mut!(sink);
    sink.send(data.into())
        .await
        .map_err(Error::data("write copy-in"))?;
    sink.finish().await.map_err(Error::data("finish copy_in"))?;

    Ok(())
}

async fn reset_password(tx: &Transaction<'_>, user_id: &UserId, pw: &str) -> crate::Result<()> {
    let hash = bcrypt::hash(pw, 10).map_err(|_| Error::Bcrypt)?;

    let stmt = tx
        .prepare_cached("UPDATE auth.users SET encrypted_password = $1 WHERE id = $2")
        .await
        .map_err(Error::exec("prepare"))?;
    tx.execute(&stmt, &[&hash, user_id])
        .await
        .map_err(Error::exec("reset_pw_hash"))?;

    let stmt = tx
        .prepare_cached(
            "INSERT INTO auth.passwords (user_id, password) VALUES ($1, $2)
                ON CONFLICT (user_id) DO UPDATE SET password = $2",
        )
        .await
        .map_err(Error::exec("prepare"))?;
    tx.execute(&stmt, &[user_id, &pw])
        .await
        .map_err(Error::exec("reset_pw_pass"))?;

    Ok(())
}

fn rand_password() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 24)
}

#[cfg(test)]
mod tests {
    use crate::{
        config::DbConfig, connection::ExportedUserData, pool::RealDbPool, LocalStorage, WasmStorage,
    };
    use serde::Deserialize;
    use toml::value::Table;

    #[tokio::test]
    #[ignore]
    async fn test_import() {
        let full_config: Table =
            toml::from_str(&std::fs::read_to_string("/tmp/local.toml").unwrap()).unwrap();
        let db_config = DbConfig::deserialize(full_config["db"].clone()).unwrap();
        let wasm = WasmStorage::new("http://localhost".parse().unwrap(), "", "").unwrap();
        let temp = tempfile::tempdir().unwrap();
        let local = LocalStorage::new(temp.path()).unwrap();
        let pool = RealDbPool::new(&db_config, wasm, local).await.unwrap();
        let mut conn = pool.get_admin_conn().await.unwrap();
        let data: ExportedUserData =
            serde_json::from_str(&std::fs::read_to_string("/tmp/data.json").unwrap()).unwrap();
        conn.import_data(data).await.unwrap();
    }
}
