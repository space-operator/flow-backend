use actix::{Actor, AsyncContext, SystemRegistry};
use actix_web::{
    App, HttpServer,
    middleware::{Compress, Logger},
    web,
};
use command_rpc::flow_side::address_book::BaseAddressBook;
use db::{LocalStorage, WasmStorage, pool::DbPool};
use flow_lib::{command::CommandFactory, utils::TowerClient};
use flow_server::{
    Config,
    api::{
        self,
        flow_api_input::{NewRequestService, RequestStore},
        prelude::Success,
    },
    cmd_workers::WorkerAuthenticate,
    db_worker::{DBWorker, SystemShutdown},
    middleware::auth_v1,
    user::SupabaseAuth,
    ws,
};
use futures_util::future::ok;
use metrics_rs_dashboard_actix::{
    DashboardInput, create_metrics_actx_scope, metrics_exporter_prometheus::Matcher,
};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};
use utils::address_book::AddressBook;
use x402_actix::middleware::X402Middleware;

// avoid commands being optimized out by the compiler
#[cfg(feature = "commands")]
use cmds_deno as _;
#[cfg(feature = "commands")]
use cmds_pdg as _;
#[cfg(feature = "commands")]
use cmds_solana as _;
#[cfg(feature = "commands")]
use cmds_std as _;

#[actix::main]
async fn main() {
    let (flow_logs, ignore) = flow_tracing::new();
    tracing_subscriber::Registry::default()
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_filter(ignore),
        )
        .with(flow_logs.clone())
        .init();

    let config = Config::get_config().unwrap();

    if let Err(errors) = config.healthcheck().await {
        for error in errors {
            tracing::error!("{}", error);
        }
    }

    let natives = CommandFactory::collect().availables().collect::<Vec<_>>();
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

    let actors = AddressBook::new();

    let pool_size = config.db.max_pool_size;

    let db = match DbPool::new(&config.db, wasm_storage.clone(), local).await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!("failed to start database connection pool: {}", e);
            return;
        }
    };

    /*
     * TODO: add this back
    if let DbPool::Real(db) = &db {
        let res = db
            .get_admin_conn()
            .and_then(move |conn| async move {
                let names = conn.get_natives_commands().await?;
                let mut missing = BTreeSet::new();
                for name in names {
                    if !natives.contains(&name.as_str()) && !rhai_script::is_rhai_script(&name) {
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
    */

    let x402 = X402Middleware::new(config.facilitator_client())
        .await
        .inspect_err(|error| {
            tracing::error!("x402 init error: {}", error);
        })
        .ok();

    let store = RequestStore::new_app_data();

    let base_book = {
        let auth = auth_v1::AuthV1::new(&config, &db).unwrap();
        BaseAddressBook::new(
            command_rpc::flow_side::address_book::ServerConfig {
                secret_key: config.iroh.secret_key.clone(),
            },
            TowerClient::new(
                WorkerAuthenticate::builder()
                    .trusted(config.iroh.trusted.clone())
                    .auth(auth)
                    .build(),
            ),
        )
        .await
        .unwrap()
    };

    tracing::info!("iroh node ID: {}", config.iroh.secret_key.public());

    let db_worker = DBWorker::create(|ctx| {
        DBWorker::builder()
            .config(&config)
            .db(db.clone())
            .actors(actors)
            .tracing_data(flow_logs)
            .new_flow_api_request(NewRequestService {
                store: store.clone(),
                db_worker: ctx.address(),
                endpoints: config.endpoints(),
            })
            .remote_command_address_book(base_book)
            .ctx(ctx)
            .build()
    });

    SystemRegistry::set(db_worker.clone());

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
    let server_hostname = config.server_hostname.clone();

    let config = Arc::new(config);
    let mut server = HttpServer::new(move || {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![
                (
                    Matcher::Full("event_lag".to_owned()),
                    &[0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 10.0],
                ),
                (
                    Matcher::Full("batch_nodes_insert_size".to_owned()),
                    &[0.0, 8.0, 16.0, 24.0, 32.0, 40.0, 48.0, 56.0, 64.0],
                ),
                (
                    Matcher::Full("after_insert_size".to_owned()),
                    &[0.0, 8.0, 16.0, 24.0, 32.0, 40.0, 48.0, 56.0, 64.0],
                ),
                (
                    Matcher::Full("new_flow_run".to_owned()),
                    &[0.05, 0.1, 0.2, 0.5, 1.0],
                ),
            ],
        };
        let metrics_scope = create_metrics_actx_scope(&dashboard_input).unwrap();

        let wallets = supabase_auth.as_ref().map(|supabase_auth| {
            web::scope("/wallets")
                .app_data(web::Data::new(supabase_auth.clone()))
                .service(api::upsert_wallet::service(&config))
        });

        let auth = supabase_auth.as_ref().map(|supabase_auth| {
            web::scope("/auth")
                .app_data(web::Data::new(sig_auth))
                .app_data(web::Data::new(supabase_auth.clone()))
                .service(api::claim_token::service(&config))
                .service(api::init_auth::service(&config))
                .service(api::confirm_auth::service(&config))
        });

        let mut flow = web::scope("/flow")
            .service(api::start_flow::service(&config))
            .service(api::stop_flow::service(&config))
            .service(api::start_flow_shared::service(&config))
            .service(api::clone_flow::service(&config))
            .service(api::get_flow_output::service(&config))
            .service(api::get_signature_request::service(&config))
            .service(api::deploy_flow::service(&config))
            .configure(api::flow_api_input::configure(store.clone()));
        if let Some(supabase_auth) = &supabase_auth {
            flow = flow.service(api::start_flow_unverified::service(
                &config,
                web::Data::new(supabase_auth.clone()),
            ))
        }

        let websocket = web::scope("/ws").service(ws::service(&config));
        let signature = web::scope("/signature").service(api::submit_signature::service(&config));

        let healthcheck = web::resource("/healthcheck")
            .route(web::get().to(|()| ok::<_, Infallible>(web::Json(Success))));
        let apikeys = web::scope("/apikey")
            .service(api::create_apikey::service(&config))
            .service(api::delete_apikey::service(&config))
            .service(api::apikey_info::service(&config));
        let kvstore = web::scope("/kv")
            .service(api::kvstore::create_store::service(&config))
            .service(api::kvstore::delete_store::service(&config))
            .service(api::kvstore::write_item::service(&config))
            .service(api::kvstore::delete_item::service(&config))
            .service(api::kvstore::read_item::service(&config));

        let deployment = web::scope("/deployment").service(api::start_deployment::service(&config));

        let logger = Logger::new(r#""%r" %s %b %{content-encoding}o %Dms"#)
            .exclude("/healthcheck")
            .exclude_regex("/metrics/");

        let mut app = App::new()
            .wrap(Compress::default())
            .wrap(logger)
            .app_data(web::Data::new(x402.clone()))
            .app_data(web::Data::new(db.clone()))
            .configure(|cfg| auth_v1::configure(cfg, &config, &db))
            .app_data(web::Data::new(sig_auth))
            .app_data(web::Data::new(db.clone()))
            .service(metrics_scope);
        if let Some(auth) = supabase_auth.clone() {
            app = app.app_data(web::Data::new(auth));
        }

        if let Some(wallets) = wallets {
            app = app.service(wallets);
        }

        if let Some(auth) = auth {
            app = app.service(auth);
        }

        let data = {
            let mut svc = web::scope("/data").service(api::data_export::service(&config));
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
    server = server.server_hostname(server_hostname);
    server.bind((host, port)).unwrap().run().await.unwrap();

    root.send(SystemShutdown {
        timeout: Duration::from_secs(shutdown_timeout_secs as u64),
    })
    .await
    .unwrap();
}
