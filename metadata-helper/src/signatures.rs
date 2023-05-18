use std::fmt::format;
use std::io::Read;
use crate::AppState;

use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse};
use actix_web::http::header;
use commons::{self, Fallible, GraphError};
use commons::tracing::get_tracer;
use opentelemetry::{
    trace::{mark_span_as_active, FutureExt, Tracer},
    Context as ot_context,
};
use prometheus::{histogram_opts, Histogram, IntCounterVec, Opts, Registry};
use std::path::PathBuf;

lazy_static! {
    static ref SIGNATURES_INCOMING_REQS: IntCounterVec = IntCounterVec::new(
        Opts::new("signatures_incoming_requests_total",
        "Total number of incoming HTTP client request"),
        &["uri_path"]
    )
    .unwrap();
    // Histogram with custom bucket values for serving latency metric (in seconds), values are picked based on monthly data
    static ref SIGNATURES_SERVE_HIST: Histogram = Histogram::with_opts(histogram_opts!(
        "signatures_serve_duration_seconds",
        "HTTP graph serving latency in seconds",
        vec![0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 5.0]
    ))
    .unwrap();
}

/// Register relevant metrics to a prometheus registry.
pub(crate) fn register_metrics(registry: &Registry) -> Fallible<()> {
    commons::register_metrics(registry)?;
    registry.register(Box::new(SIGNATURES_INCOMING_REQS.clone()))?;
    registry.register(Box::new(SIGNATURES_SERVE_HIST.clone()))?;
    Ok(())
}

/// Serve Cincinnati graph requests.
pub(crate) async fn index(
    req: HttpRequest,
    app_data: actix_web::web::Data<AppState>,
    digest : String,
    signature: String,
) -> Result<HttpResponse, GraphError> {
    debug!("{}, {}", digest, signature);
    _index(&req, app_data)
        .await
        .map_err(|e| api_response_error(&req, e))

}

async fn _index(
    req: &HttpRequest,
    app_data: actix_web::web::Data<AppState>,
) -> Result<HttpResponse, GraphError> {
    let span = get_tracer().start("index");
    let _active_span = mark_span_as_active(span);

    let path = req.uri().path();
    SIGNATURES_INCOMING_REQS.with_label_values(&[path]).inc();

    let timer = SIGNATURES_SERVE_HIST.start_timer();

    let params = req.match_info();
    let digest = params.get("digest").unwrap();
    let signature = params.get("signature").unwrap();

    let signatures_data_path = app_data.signatures_dir.clone();
    let mut signature_path = PathBuf::from(signatures_data_path);
    signature_path.push(&format!("{}/{}", digest, signature));

    let f = NamedFile::open(signature_path);
    if f.is_err() {
        return Err(GraphError::DoesNotExist(format!(
            "signature does not exist {}",
            f.unwrap_err()
        )));
    }

    let mut signature = String::new();
    let sig_open_res = f.unwrap().read_to_string(&mut signature);
    if sig_open_res.is_err(){
        return Err(GraphError::FileOpenError(format!(
            "unable to read {}",
            sig_open_res.unwrap_err()
        )));
    };
    timer.observe_duration();
    Ok(HttpResponse::Ok().body(signature))
}

// logs api request error
fn api_response_error(req: &HttpRequest, e: GraphError) -> GraphError {
    error!(
        "Error serving request \"{}\" from '{}': {:?}",
        format_request(req),
        req.peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "<not available>".into()),
        e
    );
    e
}

// format the request before logging. Include only details that we need.
pub fn format_request(req: &HttpRequest) -> String {
    let no_user_agent = header::HeaderValue::from_str("user-agent not available").unwrap();
    let no_accept_type = header::HeaderValue::from_str("Accept value unavailable").unwrap();
    let req_type = req.method().as_str();
    let request = req.path();
    let query = req.query_string();
    let user_agent = req
        .headers()
        .get("user-agent")
        .unwrap_or(&no_user_agent)
        .to_str()
        .unwrap();
    let accept_type = req
        .headers()
        .get("accept")
        .unwrap_or(&no_accept_type)
        .to_str()
        .unwrap();
    format!(
        "Method: '{}', Request: '{}', Query: '{}', User-Agent: '{}', Accept: '{}'",
        req_type, request, query, user_agent, accept_type
    )
}