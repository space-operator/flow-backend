use super::prelude::*;
use crate::db_worker::{DBWorker, FlowRunEvent, SubscribeEvents};
use actix::AsyncContext;
use actix_web_actors::ws;
use flow::event;
use value::json_repr::JsonRepr;

pub fn service(config: &Config, _db: DbPool) -> impl HttpServiceFactory + 'static {
    web::scope("/run/{flow_run_id}")
        .wrap(config.cors())
        .service(web::resource("/events").route(web::get().to(events_stream)))
}

async fn events_stream(
    flow_run_id: web::Path<FlowRunId>,
    ctx: web::Data<Context>,
    req: actix_web::HttpRequest,
    stream: web::Payload,
) -> Result<actix_web::HttpResponse, Error> {
    tracing::info!("{}", req.path());
    let resp = ws::start(
        EventsWs {
            flow_run_id: flow_run_id.into_inner(),
            db_worker: ctx.db_worker.clone(),
        },
        &req,
        stream,
    )?;
    Ok(resp)
}

pub struct EventsWs {
    flow_run_id: FlowRunId,
    db_worker: actix::Addr<DBWorker>,
}

impl actix::Actor for EventsWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.db_worker.do_send(SubscribeEvents {
            flow_run_id: self.flow_run_id,
            ws: ctx.address(),
        });
    }
}

/// Handler for ws::Message message
impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for EventsWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

impl actix::Handler<FlowRunEvent> for EventsWs {
    type Result = ();
    fn handle(&mut self, msg: FlowRunEvent, ctx: &mut Self::Context) -> Self::Result {
        let event = msg.0;
        match event.event {
            event::EventType::FlowStart => {}
            event::EventType::FlowFinish { output, .. } => {
                let json =
                    serde_json::to_string(&JsonRepr::new(&value::Value::Map(output))).unwrap();
                ctx.text(json);
            }
            event::EventType::FlowError { .. } => {}
            event::EventType::NodeStart { .. } => {}
            event::EventType::NodeOutput { .. } => {}
            event::EventType::NodeFinish { .. } => {}
            event::EventType::NodeError { .. } => {}
            event::EventType::NodeLog { .. } => {}
            event::EventType::FlowLog { .. } => {}
        }
    }
}
