use super::prelude::*;
use crate::db_worker::{GetUserWorker, user_worker::CloneFlow};
use hashbrown::HashMap;

#[derive(Serialize)]
pub struct Output {
    pub flow_id: FlowId,
    pub id_map: HashMap<FlowId, FlowId>,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/clone/{id}")
        .wrap(config.cors())
        .route(web::post().to(clone_flow))
}

async fn clone_flow(
    flow_id: web::Path<FlowId>,
    user: Auth<auth_v1::AuthenticatedUser>,
) -> Result<web::Json<Output>, Error> {
    let flow_id = flow_id.into_inner();

    let db_worker = DBWorker::from_registry();

    let id_map = db_worker
        .send(GetUserWorker {
            user_id: *user.user_id(),
        })
        .await?
        .send(CloneFlow {
            user_id: *user.user_id(),
            flow_id,
        })
        .await??;

    Ok(web::Json(Output {
        flow_id: *id_map
            .get(&flow_id)
            .ok_or_else(|| Error::custom(StatusCode::INTERNAL_SERVER_ERROR, "bug in clone_flow"))?,
        id_map,
    }))
}
