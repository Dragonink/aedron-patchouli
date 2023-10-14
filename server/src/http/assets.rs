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
	response::{Css, JavaScript, Wasm},
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
		Some((_, "js")) => JavaScript(body).into_response(),
		Some((_, "wasm")) => Wasm(body).into_response(),
		Some((_, "css")) => Css(body).into_response(),
		_ => body.into_response(),
	})
}

/// Constructs a new configured [`Router`]
#[inline]
pub(super) fn new_router() -> Router<AppState> {
	Router::new().route("/*path", routing::get(get_asset))
}
