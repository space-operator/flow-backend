use actix::Actor;
use actix_web::{
    middleware::{Compress, Logger},
    web, App, HttpServer,
};
use db::{
    pool::{DbPool, ProxiedDbPool, RealDbPool},
    LocalStorage, WasmStorage,
};
use either::Either;
use flow_server::{
    api::{self, prelude::Success},
    db_worker::{token_worker::token_from_apikeys, DBWorker, SystemShutdown},
    flow_logs,
    middleware::auth_v1,
    user::SupabaseAuth,
    ws, Config,
};
use futures_util::{future::ok, TryFutureExt};
use std::{borrow::Cow, collections::BTreeSet, convert::Infallible, sync::Arc, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use utils::address_book::AddressBook;

// avoid commands being optimized out by the compiler
use cmds_pdg as _;
use cmds_solana as _;
use cmds_std as _;

#[actix::main]
async fn main() {
    let (flow_logs, tracing_data) = flow_logs::FlowLogs::new();
    tracing_subscriber::Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_filter(flow_logs::IgnoreFlowLogs::new(tracing_data.clone())),
        )
        .with(flow_logs)
        .init();

    let config = Config::get_config();

    if let Err(errors) = config.healthcheck().await {
        for error in errors {
            tracing::error!("{}", error);
        }
    }

    let fac = flow::context::CommandFactory::new();
    let natives = fac.natives.keys().collect::<Vec<_>>();
    tracing::info!("native commands: {:?}", natives);

    tracing::info!("allow CORS origins: {:?}", config.cors_origins);

    let wasm_storage = match WasmStorage::new(
        config.supabase.get_endpoint(),
        &config.supabase.anon_key,
        &config.supabase.wasm_bucket,
    ) {
        Ok(db) => db,
        Err(e) => {
            tracing::error!("failed to build storage client: {}", e);
            return;
        }
    };

    let local = match LocalStorage::new(&config.local_storage) {
        Ok(local) => local,
        Err(e) => {
            tracing::error!(
                "failed to open local storage {:?}: {}",
                config.local_storage.display(),
                e
            );
            return;
        }
    };

    let mut actors = AddressBook::new();

    let pool_size = if let Either::Left(cfg) = &config.db {
        cfg.max_pool_size
    } else {
        None
    };

    let db = match match &config.db {
        Either::Left(cfg) => RealDbPool::new(cfg, wasm_storage.clone(), local)
            .await
            .map(DbPool::Real),
        Either::Right(cfg) => {
            let (services, new_actors) = token_from_apikeys(
                cfg.api_keys.clone(),
                local.clone(),
                config.endpoints(),
                cfg.upstream_url.to_string(),
            )
            .await;
            for (id, addr) in new_actors {
                assert!(actors.insert(id, addr.downgrade()));
            }
            ProxiedDbPool::new(cfg.clone(), local, services).map(DbPool::Proxied)
        }
    } {
        Ok(db) => db,
        Err(e) => {
            tracing::error!("failed to start database connection pool: {}", e);
            return;
        }
    };

    if let DbPool::Real(db) = &db {
        let res = db
            .get_admin_conn()
            .and_then(move |conn| async move {
                let names = conn.get_natives_commands().await?;
                let mut missing = BTreeSet::new();
                for name in names {
                    if !natives.contains(&&Cow::Borrowed(name.as_str()))
                        && !rhai_script::is_rhai_script(&name)
                    {
                        missing.insert(name);
                    }
                }
                Ok(missing)
            })
            .await;
        match res {
            Ok(missing) => {
                if !missing.is_empty() {
                    tracing::warn!("missing native commands: {:?}", missing);
                }
            }
            Err(error) => {
                tracing::error!("{}", error);
            }
        }
    }

    let db_worker =
        DBWorker::create(|ctx| DBWorker::new(db.clone(), &config, actors, tracing_data, ctx));

    let sig_auth = config.signature_auth();
    let supabase_auth = match SupabaseAuth::new(&config.supabase, db.clone()) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("missing credentials, some routes are not available: {}", e);
            None
        }
    };

    let host = config.host.clone();
    let port = config.port;

    tracing::info!("listening on {:?} port {:?}", host, port);

    let root = db_worker.clone();

    let shutdown_timeout_secs = config.shutdown_timeout_secs;

    if let Some(key) = &config.helius_api_key {
        std::env::set_var("HELIUS_API_KEY", key);
    }
    if let Some(solana) = &config.solana {
        if let Some(url) = &solana.devnet_url {
            std::env::set_var("SOLANA_DEVNET_URL", url.to_string());
        }
        if let Some(url) = &solana.testnet_url {
            std::env::set_var("SOLANA_TESTNET_URL", url.to_string());
        }
        if let Some(url) = &solana.mainnet_url {
            std::env::set_var("SOLANA_MAINNET_URL", url.to_string());
        }
    }

    let config = Arc::new(config);
    let mut server = HttpServer::new(move || {
        let wallets = supabase_auth.as_ref().map(|supabase_auth| {
            web::scope("/wallets")
                .app_data(web::Data::new(supabase_auth.clone()))
                .service(api::upsert_wallet::service(&config, db.clone()))
        });

        let auth = supabase_auth.as_ref().map(|supabase_auth| {
            web::scope("/auth")
                .app_data(web::Data::new(sig_auth))
                .app_data(web::Data::new(supabase_auth.clone()))
                .service(api::claim_token::service(&config, db.clone()))
                .service(api::init_auth::service(&config))
                .service(api::confirm_auth::service(&config))
        });

        let mut flow = web::scope("/flow")
            .service(api::start_flow::service(&config, db.clone()))
            .service(api::stop_flow::service(&config, db.clone()))
            .service(api::start_flow_shared::service(&config, db.clone()))
            .service(api::clone_flow::service(&config, db.clone()))
            .service(api::get_flow_output::service(&config, db.clone()))
            .service(api::get_signature_request::service(&config, db.clone()))
            .service(api::deploy_flow::service(&config));
        if let Some(supabase_auth) = &supabase_auth {
            flow = flow.service(api::start_flow_unverified::service(
                &config,
                db.clone(),
                web::Data::new(supabase_auth.clone()),
            ))
        }

        let websocket = web::scope("/ws").service(ws::service(&config, db.clone()));
        let signature = web::scope("/signature").service(api::submit_signature::service(&config));

        let healthcheck = web::resource("/healthcheck")
            .route(web::get().to(|()| ok::<_, Infallible>(web::Json(Success))));
        let apikeys = web::scope("/apikey")
            .service(api::create_apikey::service(&config, db.clone()))
            .service(api::delete_apikey::service(&config, db.clone()))
            .service(api::apikey_info::service(&config));
        let kvstore = web::scope("/kv")
            .service(api::kvstore::create_store::service(&config, db.clone()))
            .service(api::kvstore::delete_store::service(&config, db.clone()))
            .service(api::kvstore::write_item::service(&config, db.clone()))
            .service(api::kvstore::delete_item::service(&config, db.clone()))
            .service(api::kvstore::read_item::service(&config, db.clone()));

        let db_proxy = if matches!(db, DbPool::Real(_)) {
            Some(
                web::scope("/proxy")
                    .service(api::db_rpc::service(&config, db.clone()))
                    .service(api::db_push_logs::service(&config, db.clone()))
                    .service(api::auth_proxy::service(&config, db.clone()))
                    .service(api::ws_auth_proxy::service(&config, db.clone())),
            )
        } else {
            None
        };

        let deployment = web::scope("/deployment").service(api::start_deployment::service(&config));

        let mut app = App::new()
            .wrap(Compress::default())
            .wrap(Logger::new(r#""%r" %s %b %{content-encoding}o %Dms"#).exclude("/healthcheck"))
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(db_worker.clone()))
            .configure(|cfg| auth_v1::configure(cfg, &config, &db))
            .app_data(web::Data::new(sig_auth));
        if let Some(auth) = supabase_auth.clone() {
            app = app.app_data(web::Data::new(auth));
        }

        let mut app = match &db {
            DbPool::Real(db) => app.app_data(web::Data::new(db.clone())),
            DbPool::Proxied(db) => app.app_data(web::Data::new(db.clone())),
        };

        if let Some(wallets) = wallets {
            app = app.service(wallets);
        }

        if let Some(auth) = auth {
            app = app.service(auth);
        }

        if let Some(db_proxy) = db_proxy {
            app = app.service(db_proxy);
        }

        let data = {
            let mut svc =
                web::scope("/data").service(api::data_export::service(&config, db.clone()));
            #[cfg(feature = "import")]
            if let Some(import) = api::data_import::service(&config) {
                svc = svc.service(import);
            }
            svc
        };

        app.service(flow)
            .service(data)
            .service(signature)
            .service(apikeys)
            .service(websocket)
            .service(kvstore)
            .service(healthcheck)
            .service(api::get_info::service(&config))
            .service(deployment)
    })
    .shutdown_timeout(shutdown_timeout_secs as u64);
    if let Some(pool_size) = pool_size {
        server = server.workers((pool_size / 2).max(4));
    }
    server.bind((host, port)).unwrap().run().await.unwrap();

    root.send(SystemShutdown {
        timeout: Duration::from_secs(shutdown_timeout_secs as u64),
    })
    .await
    .unwrap();
}
