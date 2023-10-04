//! Provides routes for the server's assets

use crate::AppState;
use axum::{
	extract::Path,
	http::StatusCode,
	response::{IntoResponse, Response},
	routing, Router,
};
use axum_extra::{
	body::AsyncReadBody,
	response::{Css, Html, JavaScript, Wasm},
};
use tokio::fs::File;

/// `GET /*`
/// [Handler](axum::handler) that returns the requested file from `client/assets/`
#[axum::debug_handler(state = AppState)]
async fn get_asset(Path(path): Path<String>) -> Result<Response, (StatusCode, String)> {
	let assets_dir = std::path::Path::new("client/assets");
	let file = match File::open(assets_dir.join(&path)).await {
		Ok(file) => Ok(file),
		Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => {
			File::open(assets_dir.join("out").join(&path)).await
		}
		Err(err) => Err(err),
	}
	.map_err(|err| {
		(
			match err.kind() {
				std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
				_ => StatusCode::INTERNAL_SERVER_ERROR,
			},
			err.to_string(),
		)
	})?;

	let body = AsyncReadBody::new(file);
	Ok(match path.rsplit_once('.') {
		Some((_, "html")) => Html(body).into_response(),
		Some((_, "css")) => Css(body).into_response(),
		Some((_, "js")) => JavaScript(body).into_response(),
		Some((_, "wasm")) => Wasm(body).into_response(),
		_ => body.into_response(),
	})
}

/// Constructs a new configured [`Router`]
///
/// This router should be [`nest`ed](Router::nest).
#[inline]
pub(super) fn new_nested_router() -> Router<AppState> {
	Router::new().route("/*path", routing::get(get_asset))
}

/// Constructs a new configured [`Router`]
///
/// This router should be [`merge`d](Router::merge).
#[inline]
pub(super) fn new_merged_router() -> Router<AppState> {
	let index_handler = || get_asset(Path("index.html".to_owned()));

	Router::new()
		.route("/", routing::get(index_handler))
		.route("/*any", routing::get(index_handler))
}
