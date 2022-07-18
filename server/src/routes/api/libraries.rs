use crate::{
	guards::{AdminUser, RequiredAdminUser, RequiredUser, User},
	routes::{library_kind_response_err, sqlx_response_err, SqlxResponseResult},
	Database,
};
use aedron_patchouli_common::{
	libraries::*,
	media::{DbMediaImage, DbMediaMusic, MediaImage, MediaMusic},
	permissions::PermAction,
};
use either::Either;
use rocket::{
	form::Form, http::Status, response::status::Created, serde::msgpack::MsgPack, Route, State,
};
use rocket_db_pools::Connection;
use std::{io, ops::Deref};

#[non_exhaustive]
#[derive(Debug, Clone, Responder)]
enum ResponseLibrary {
	Image(MsgPack<Library<MediaImage>>),
	Music(MsgPack<Library<MediaMusic>>),
}
macro_rules! responder_from_library {
	($($media:ty => $variant:ident),*) => {
		$(
			impl From<Library<$media>> for ResponseLibrary {
				#[inline]
				fn from(lib: Library<$media>) -> Self {
					Self::$variant(MsgPack(lib))
				}
			}
		)*
	};
}
responder_from_library! {
	MediaImage => Image,
	MediaMusic => Music
}

fn spawn_index_library_task(db_pool: &Database, mut db: Connection<Database>, id: i64) {
	let _ = tokio::spawn({
		let db_pool = db_pool.clone();
		async move {
			let config =
				sqlx::query_as!(DbLibraryConfig, "SELECT * FROM libraries WHERE id = ?", id)
					.fetch_one(&mut *db)
					.await
					.unwrap()
					.try_into()
					.unwrap();
			crate::tasks::index_library(&db_pool, &config).await
		}
	});
}

#[get("/")]
async fn read_libraries(
	mut db: Connection<Database>,
	user: RequiredUser<'_>,
) -> SqlxResponseResult<MsgPack<Vec<PartialLibrary>>> {
	let user_id = user.id() as i64;
	let data = sqlx::query_as!(
		DbPartialLibrary,
		"
		SELECT DISTINCT libraries.id, libraries.name, libraries.kind FROM libraries
		JOIN effect_permissions as perms ON perms.library = libraries.id
		WHERE
			? = ? OR
			(perms.user = ? AND perms.action = ?)
		",
		user_id,
		User::ADMIN_ID as i64,
		user_id,
		PermAction::Allow as i8
	)
	.fetch_all(&mut *db)
	.await
	.map_err(sqlx_response_err)?;

	Ok(MsgPack(
		data.into_iter()
			.map(TryFrom::try_from)
			.collect::<Result<_, _>>()
			.map_err(library_kind_response_err)?,
	))
}

#[get("/?<config>")]
async fn read_libraries_config(
	mut db: Connection<Database>,
	admin: AdminUser<'_>,
	config: bool,
) -> SqlxResponseResult<Either<MsgPack<Vec<LibraryConfig>>, MsgPack<Vec<PartialLibrary>>>> {
	if config {
		sqlx::query_as!(DbLibraryConfig, "SELECT * FROM libraries")
			.fetch_all(&mut *db)
			.await
			.map_err(sqlx_response_err)
			.and_then(|data| {
				data.into_iter()
					.map(|db_config| db_config.try_into().map_err(library_kind_response_err))
					.collect()
			})
			.map(|config| Either::Left(MsgPack(config)))
	} else {
		read_libraries(db, admin.deref().into())
			.await
			.map(Either::Right)
	}
}

#[post("/", data = "<raw_config>")]
async fn create_library(
	db_pool: &State<Database>,
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	raw_config: Form<RawLibraryConfig>,
) -> SqlxResponseResult<Created<MsgPack<LibraryConfig>>> {
	let db_config: DbRawLibraryConfig = raw_config.clone().into();
	let config: LibraryConfig = sqlx::query_as!(
		DbLibraryConfig,
		"INSERT INTO libraries (name, kind, paths) VALUES (?, ?, ?) RETURNING *",
		db_config.name,
		db_config.kind,
		db_config.paths
	)
	.fetch_one(&mut *db)
	.await
	.map_err(sqlx_response_err)?
	.try_into()
	.map_err(library_kind_response_err)?;

	let id = config.id as i64;
	sqlx::query!(
		"INSERT INTO permissions (library, action) VALUES (?, ?)",
		id,
		Some(PermAction::Allow as i8)
	)
	.execute(&mut *db)
	.await
	.map_err(sqlx_response_err)?;
	sqlx::query!(
		"
		INSERT INTO permissions
		SELECT ? as library, id as user, NULL as action FROM users
		WHERE id != 1
		",
		id
	)
	.execute(&mut *db)
	.await
	.map_err(sqlx_response_err)?;

	spawn_index_library_task(db_pool, db, id);

	Ok(Created::new(uri!(delete_library(config.id)).to_string()).body(MsgPack(config)))
}

#[get("/<id>?<full>", rank = 2)]
async fn read_library(
	mut db: Connection<Database>,
	user: RequiredUser<'_>,
	id: u64,
	full: bool,
) -> SqlxResponseResult<Either<MsgPack<PartialLibrary>, ResponseLibrary>> {
	let id = id as i64;
	let user_id = user.id() as i64;
	let library: PartialLibrary = sqlx::query_as!(
		DbPartialLibrary,
		"
		SELECT libraries.id, libraries.name, libraries.kind FROM libraries
		JOIN effect_permissions as perms ON perms.library = libraries.id
		WHERE
			libraries.id = ? AND
			(? = ? OR
			(perms.user = ? AND perms.action = ?))
		",
		id,
		user_id,
		User::ADMIN_ID as i64,
		user_id,
		PermAction::Allow as i8
	)
	.fetch_one(&mut *db)
	.await
	.map_err(sqlx_response_err)?
	.try_into()
	.map_err(library_kind_response_err)?;

	if full {
		macro_rules! return_library {
			($($kind:ident => $query:expr => $media:ty),*) => {
				match library.kind {
					$(
						LibraryKind::$kind => Ok(Either::Right(Library::new(
							library,
							$query
							.fetch_all(&mut *db)
							.await
							.map_err(sqlx_response_err)?
							.into_iter()
							.map(|db_media| db_media.into())
							.collect::<Vec<$media>>(),
						).into())),
					)*
					_ => Err(Either::Left(io::ErrorKind::Unsupported.into())),
				}
			};
		}
		return_library! {
			Image => sqlx::query_as!(
				DbMediaImage,
				"SELECT id, title FROM media_image WHERE library = ?",
				id
			) => MediaImage,
			Music => sqlx::query_as!(
				DbMediaMusic,
				"SELECT id, title, artist, album, track FROM media_music WHERE library = ?",
				id
			) => MediaMusic
		}
	} else {
		Ok(Either::Left(MsgPack(library)))
	}
}

#[get("/<id>?<config>&<full>")]
async fn read_library_config(
	mut db: Connection<Database>,
	admin: AdminUser<'_>,
	id: u64,
	config: bool,
	full: bool,
) -> SqlxResponseResult<
	Either<MsgPack<LibraryConfig>, Either<MsgPack<PartialLibrary>, ResponseLibrary>>,
> {
	if config {
		let id = id as i64;
		sqlx::query_as!(DbLibraryConfig, "SELECT * FROM libraries WHERE id = ?", id)
			.fetch_one(&mut *db)
			.await
			.map_err(sqlx_response_err)
			.and_then(|data| data.try_into().map_err(library_kind_response_err))
			.map(|config| Either::Left(MsgPack(config)))
	} else {
		read_library(db, admin.deref().into(), id, full)
			.await
			.map(Either::Right)
	}
}

#[put("/<id>", data = "<config>")]
async fn update_library(
	db_pool: &State<Database>,
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
	config: Form<RawLibraryConfig>,
) -> SqlxResponseResult<Status> {
	let id = id as i64;
	let db_config: DbRawLibraryConfig = config.into_inner().into();
	let old_paths = sqlx::query_scalar!("SELECT paths FROM libraries WHERE id = ?", id)
		.fetch_one(&mut *db)
		.await
		.map_err(sqlx_response_err)?;
	let affected = sqlx::query!(
		"UPDATE libraries SET name = ?, kind = ?, paths = ? WHERE id = ?",
		db_config.name,
		db_config.kind,
		db_config.paths,
		id
	)
	.execute(&mut *db)
	.await
	.map_err(sqlx_response_err)?
	.rows_affected();

	Ok(if affected > 0 {
		if db_config.paths != old_paths {
			spawn_index_library_task(db_pool, db, id);
		}
		Status::NoContent
	} else {
		Status::NotFound
	})
}

#[delete("/<id>")]
async fn delete_library(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
) -> SqlxResponseResult<Status> {
	let id = id as i64;
	let affected = sqlx::query!("DELETE FROM libraries WHERE id = ?", id)
		.execute(&mut *db)
		.await
		.map_err(sqlx_response_err)?
		.rows_affected();

	Ok(if affected > 0 {
		Status::NoContent
	} else {
		Status::NotFound
	})
}

#[inline]
pub(super) fn routes() -> Vec<Route> {
	routes![
		read_libraries,
		read_libraries_config,
		create_library,
		read_library,
		read_library_config,
		update_library,
		delete_library
	]
}
