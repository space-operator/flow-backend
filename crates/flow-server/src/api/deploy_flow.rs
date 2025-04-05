use crate::middleware::auth_v1::{Auth, AuthenticatedUser};

use super::prelude::*;
use db::pool::DbPool;
use flow::flow_set::{DeploymentId, FlowDeployment};

#[derive(Serialize)]
pub struct Output {
    pub deployment_id: DeploymentId,
}

pub fn service(config: &Config) -> impl HttpServiceFactory + 'static {
    web::resource("/deploy/{id}")
        .wrap(config.cors())
        .route(web::post().to(deploy_flow))
}

async fn deploy_flow(
    flow_id: web::Path<FlowId>,
    user: Auth<AuthenticatedUser>,
    db: web::Data<DbPool>,
) -> Result<web::Json<Output>, Error> {
    let mut conn = db.get_user_conn(*user.user_id()).await?;
    let deployment = FlowDeployment::from_entrypoint(flow_id.into_inner(), &mut conn)
        .await
        .map_err(flow::Error::from)?;
    let deployment_id = conn.insert_deployment(&deployment).await?;
    Ok(web::Json(Output { deployment_id }))
}
