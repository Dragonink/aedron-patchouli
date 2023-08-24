//! Provides functions to interact with the server's database

use crate::EXE_NAME;
use axum::{
	extract::{FromRef, FromRequestParts},
	http::{request::Parts, StatusCode},
};
use log::LevelFilter;
use sqlx::{pool::PoolConnection, sqlite::*, ConnectOptions};

/// Initializes the database
pub(super) async fn init_database() -> sqlx::Result<SqlitePool> {
	let options = SqliteConnectOptions::new()
		.filename(
			std::env::var_os("AEPA_DB").unwrap_or_else(|| format!("{}.sqlite", EXE_NAME).into()),
		)
		.create_if_missing(true)
		.journal_mode(SqliteJournalMode::Wal)
		.locking_mode(SqliteLockingMode::Exclusive)
		.synchronous(SqliteSynchronous::Normal)
		.auto_vacuum(SqliteAutoVacuum::Full)
		.optimize_on_close(true, None)
		.thread_name(|id| format!("db-{id}"))
		.log_statements(LevelFilter::Trace)
		.pragma("application_id", u32::from_be_bytes(*b"AEPA").to_string());

	let mut pool_options = SqlitePoolOptions::new();
	if let Ok(count) = std::thread::available_parallelism() {
		pool_options = pool_options.max_connections(count.get() as u32 * 2);
	}

	let db_pool = pool_options.connect_with(options).await?;
	log::info!("Connected to database");

	let mut conn = db_pool.acquire().await?;
	sqlx::query_file!("sql/init.sql")
		.execute(&mut *conn)
		.await?;
	log::debug!("Initialized database");

	Ok(db_pool)
}

/// [Axum extractor](axum::extract) that acquires a database connection from the pool
#[repr(transparent)]
pub(super) struct DbConn(pub(super) PoolConnection<Sqlite>);
#[axum::async_trait]
impl<S> FromRequestParts<S> for DbConn
where
	SqlitePool: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = (StatusCode, &'static str);

	async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		SqlitePool::from_ref(state)
			.acquire()
			.await
			.map_err(|err| {
				log::warn!("Unable to access database: {err}");
				(StatusCode::SERVICE_UNAVAILABLE, "Unable to access database")
			})
			.map(Self)
	}
}
impl AsRef<PoolConnection<Sqlite>> for DbConn {
	#[inline]
	fn as_ref(&self) -> &PoolConnection<Sqlite> {
		&self.0
	}
}
impl AsMut<PoolConnection<Sqlite>> for DbConn {
	#[inline]
	fn as_mut(&mut self) -> &mut PoolConnection<Sqlite> {
		&mut self.0
	}
}
