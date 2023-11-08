use crate::{Error, FlowRunLogsRow};
use anyhow::anyhow;
use bytes::Bytes;
use deadpool_postgres::Object as Connection;
use flow_lib::UserId;
use rand::distributions::{Alphanumeric, DistString};
use std::borrow::Borrow;
use tokio_postgres::{
    binary_copy::BinaryCopyInWriter,
    types::{Json, Type},
};
use uuid::Uuid;
use value::Value;

pub struct AdminConn {
    pub conn: Connection,
}

#[derive(Debug)]
pub struct Password {
    pub user_id: Uuid,
    pub email: String,
    pub password: Option<String>,
}

impl AdminConn {
    pub fn new(conn: Connection) -> AdminConn {
        Self { conn }
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
        let hash = bcrypt::hash(pw, 10).map_err(|_| Error::Bcrypt)?;

        let tx = self
            .conn
            .transaction()
            .await
            .map_err(Error::exec("reset_pw_start"))?;

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
}
