use crate::Database;
use aedron_patchouli_common::library::LibraryKind;
use either::Either;
use rocket::{Build, Rocket};
use rocket_db_pools::Connection;
use std::{io, ops::DerefMut};

mod api;
mod assets;

type SqlxResponseError = Either<io::Error, String>;
type SqlxResponseResult<T> = Result<T, SqlxResponseError>;
#[inline]
fn sqlx_response_err(err: sqlx::Error) -> SqlxResponseError {
	match err {
		sqlx::Error::RowNotFound => Either::Left(io::ErrorKind::NotFound.into()),
		sqlx::Error::Io(err) => Either::Left(err),
		err => Either::Right(err.to_string()),
	}
}
#[inline]
fn library_kind_response_err(_: u8) -> SqlxResponseError {
	Either::Right("invalid value for LibraryKind".to_string())
}

async fn fetch_library_kind(
	db: &mut Connection<Database>,
	library: i64,
) -> SqlxResponseResult<LibraryKind> {
	LibraryKind::try_from(
		sqlx::query_scalar!("SELECT kind FROM libraries WHERE id = ?", library)
			.fetch_one(db.deref_mut())
			.await
			.map_err(sqlx_response_err)? as u8,
	)
	.map_err(library_kind_response_err)
}

#[must_use = "`Rocket<Build>` must be used"]
#[inline]
pub(crate) fn mount(rocket: Rocket<Build>) -> Rocket<Build> {
	api::mount(rocket.mount("/", assets::routes()))
}
