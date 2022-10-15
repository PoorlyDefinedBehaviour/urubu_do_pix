//! This web server is run so we are able to register a service worker in the html page served by it.

use axum::{
  body::{boxed, Body, BoxBody},
  http::{Request, Response, Uri},
  routing::get,
  Router,
};
use reqwest::StatusCode;

use tower::util::ServiceExt;
use tower_http::services::ServeDir;

pub fn router() -> Router {
  Router::new().nest("/static", get(handler))
}

#[tracing::instrument(name = "GET /static", skip_all, fields(uri = ?uri))]
async fn handler(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
  get_static_file(uri.clone()).await
}

async fn get_static_file(uri: Uri) -> Result<Response<BoxBody>, (StatusCode, String)> {
  let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

  let project_dir = std::env::current_dir()
    .unwrap()
    .to_str()
    .map(String::from)
    .expect("project dir is required");

  // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
  match ServeDir::new(format!("{project_dir}/src/video_stream_api/assets"))
    .oneshot(req)
    .await
  {
    Ok(res) => Ok(res.map(boxed)),
    Err(err) => Err((
      StatusCode::INTERNAL_SERVER_ERROR,
      format!("Something went wrong: {}", err),
    )),
  }
}
