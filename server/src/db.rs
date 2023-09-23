//! Provides functions to interact with the server's database

use crate::EXE_NAME;
use axum::{
	extract::{FromRef, FromRequestParts},
	http::{request::Parts, StatusCode},
};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{config::DbConfig, OpenFlags};
use scheduled_thread_pool::ScheduledThreadPool;
use std::{error::Error, ffi::c_int, sync::Arc};

/// Initializes the pool of connections to the database
pub(crate) fn init() -> Result<Pool<SqliteConnectionManager>, Box<dyn Error>> {
	/// Callback for [`rusqlite::trace::config_log`]
	fn db_config_log(code: c_int, msg: &str) {
		log::debug!(target: "database", "({code}) {msg}");
	}
	#[allow(unsafe_code)]
	// SAFETY: `db_config_log` does not call any SQLite function and is thread-safe.
	unsafe {
		rusqlite::trace::config_log(Some(db_config_log))?;
	}

	let file = std::env::var_os("AEPA_DB").unwrap_or_else(|| format!("{EXE_NAME}.sqlite").into());
	let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
		| OpenFlags::SQLITE_OPEN_CREATE
		| OpenFlags::SQLITE_OPEN_NO_MUTEX;
	let db_pool = Pool::builder()
		.thread_pool(Arc::new(
			ScheduledThreadPool::builder()
				.num_threads(std::thread::available_parallelism().map_or(3, |num| num.get()))
				.thread_name_pattern("db-{}")
				.build(),
		))
		.build(
			SqliteConnectionManager::file(file)
				.with_flags(flags)
				.with_init(|conn| {
					/// Callback for [`Connection::trace`]
					fn db_trace(msg: &str) {
						log::trace!(target: "sql", "{msg}");
					}
					conn.trace(Some(db_trace));

					conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;
					conn.pragma_update(None, "trusted_schema", false)?;
					conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| {
						row.get::<_, Box<str>>(0)
					})?;
					conn.pragma_update(None, "synchronous", "NORMAL")?;
					conn.pragma_update(None, "auto_vacuum", "FULL")?;
					conn.pragma_update(None, "application_id", i32::from_be_bytes(*b"AEPA"))?;

					log::debug!("Opened a connection to the database");
					Ok(())
				}),
		)?;

	let mut conn = db_pool.get()?;
	let transaction = conn.transaction()?;
	transaction.execute_batch(
		"
			CREATE TABLE IF NOT EXISTS plugins (
				name TEXT NOT NULL,
				kind TEXT NOT NULL,
				version TEXT NOT NULL,

				PRIMARY KEY (name, kind) ON CONFLICT REPLACE
			) STRICT, WITHOUT ROWID;
		"
		.trim(),
	)?;
	transaction.commit()?;

	Ok(db_pool)
}

/// [Axum extractor](axum::extract) for a database connection
#[repr(transparent)]
pub(crate) struct DbConn(pub(crate) PooledConnection<SqliteConnectionManager>);
#[axum::async_trait]
impl<S> FromRequestParts<S> for DbConn
where
	Pool<SqliteConnectionManager>: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = (StatusCode, &'static str);

	#[inline]
	async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		Pool::<SqliteConnectionManager>::from_ref(state)
			.get()
			.map(Self)
			.map_err(|err| {
				log::error!("Database error: {err}");
				(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
			})
	}
}
