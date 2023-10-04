//! Provides routes for the API

use crate::{db::DbConn, plugins::PluginStore, AppState};
use axum::{
	extract::{Path, State},
	Json, Router,
};
use axum_extra::routing::Resource;
use hyper::StatusCode;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

/// `GET /api/libraries`
#[inline]
#[axum::debug_handler(state = AppState)]
async fn libraries_index(State(plugins): State<Arc<PluginStore>>) -> Json<HashMap<String, String>> {
	Json(
		plugins
			.media
			.iter()
			.map(|(name, plugin)| (name.clone(), plugin.media.name.to_str().to_owned()))
			.collect(),
	)
}

/// `GET /api/libraries/:name`
#[axum::debug_handler(state = AppState)]
async fn libraries_show(
	State(plugins): State<Arc<PluginStore>>,
	DbConn(conn): DbConn,
	Path(name): Path<String>,
) -> Result<Json<Vec<HashMap<String, Value>>>, (StatusCode, String)> {
	let plugin = plugins.media.get(&name).ok_or_else(|| {
		(
			StatusCode::NOT_FOUND,
			"The requested library does not exist".to_owned(),
		)
	})?;
	let map_err = |err: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string());

	let mut stmt = conn
		.prepare_cached(&format!(
			"SELECT * FROM {table}",
			table = plugin.table_ident()
		))
		.map_err(map_err)?;
	let cols = stmt
		.column_names()
		.into_iter()
		.map(|s| s.to_owned())
		.collect::<Vec<_>>();
	let rows = stmt
		.query_map((), |row| {
			cols.iter()
				.map(|col| {
					row.get::<_, Value>(col.as_str())
						.or_else(|err| match err {
							rusqlite::Error::FromSqlConversionFailure(..) => {
								row.get::<_, String>(col.as_str()).map(Value::from)
							}
							_ => Err(err),
						})
						.map(|val| (col.to_owned(), val))
				})
				.collect::<Result<HashMap<String, Value>, _>>()
		})
		.map_err(map_err)?;
	Ok(Json(rows.collect::<Result<_, _>>().map_err(map_err)?))
}

/// Constructs a new configured [`Router`]
///
/// This router should be [`nest`ed](Router::nest).
pub(super) fn new_router() -> Router<AppState> {
	let libraries = Resource::named("libraries")
		.index(libraries_index)
		.show(libraries_show);

	Router::new().merge(libraries)
}
