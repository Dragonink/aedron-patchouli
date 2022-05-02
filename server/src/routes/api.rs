use super::{fetch_library_kind, library_kind_response_err, sqlx_response_err, SqlxResponseResult};
use crate::Database;
use either::Either;
use rocket::{
	form::Form,
	http::Status,
	response::{status::Created, Responder},
	serde::msgpack::MsgPack,
	Build, Rocket, Route, State,
};
use rocket_db_pools::Connection;
use std::{fmt::Debug, io};

mod libraries {
	use super::*;
	use aedron_patchouli_common::{
		library::*,
		media::{DbMediaImage, DbMediaMusic, MediaImage, MediaMusic},
	};

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
					#[inline(always)]
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
	) -> SqlxResponseResult<MsgPack<Vec<PartialLibrary>>> {
		let data = sqlx::query_as!(DbPartialLibrary, "SELECT id, name, kind FROM libraries")
			.fetch_all(&mut *db)
			.await
			.map_err(sqlx_response_err)?;

		Ok(MsgPack(
			data.into_iter()
				.map(|db_lib| db_lib.try_into())
				.collect::<Result<_, _>>()
				.map_err(library_kind_response_err)?,
		))
	}

	#[post("/", data = "<raw_config>")]
	async fn create_library(
		db_pool: &State<Database>,
		mut db: Connection<Database>,
		raw_config: Form<RawLibraryConfig>,
	) -> SqlxResponseResult<Created<MsgPack<LibraryConfig>>> {
		let db_config: DbRawLibraryConfig = raw_config.clone().into();
		let config = sqlx::query_as!(
			DbLibraryConfig,
			"INSERT INTO libraries (name, kind, paths) VALUES (?, ?, ?) RETURNING *",
			db_config.name,
			db_config.kind,
			db_config.paths
		)
		.fetch_one(&mut *db)
		.await
		.map_err(sqlx_response_err)?;

		spawn_index_library_task(db_pool, db, config.id);
		let config: LibraryConfig = config.try_into().map_err(library_kind_response_err)?;

		Ok(Created::new(format!("{API_ENDPOINT}/{}", config.id)).body(MsgPack(config)))
	}

	#[get("/<id>?<full>&<config>")]
	async fn read_library(
		mut db: Connection<Database>,
		id: u64,
		full: bool,
		config: bool,
	) -> SqlxResponseResult<
		Either<Either<MsgPack<PartialLibrary>, MsgPack<LibraryConfig>>, ResponseLibrary>,
	> {
		let id = id as i64;
		if config {
			sqlx::query_as!(DbLibraryConfig, "SELECT * FROM libraries WHERE id = ?", id)
				.fetch_one(&mut *db)
				.await
				.map_err(sqlx_response_err)
				.and_then(|data| data.try_into().map_err(library_kind_response_err))
				.map(|config| Either::Left(Either::Right(MsgPack(config))))
		} else {
			let library: PartialLibrary = sqlx::query_as!(
				DbPartialLibrary,
				"SELECT id, name, kind FROM libraries WHERE id = ?",
				id
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
				Ok(Either::Left(Either::Left(MsgPack(library))))
			}
		}
	}

	#[put("/<id>", data = "<config>")]
	async fn update_library(
		db_pool: &State<Database>,
		mut db: Connection<Database>,
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
	async fn delete_library(mut db: Connection<Database>, id: u64) -> SqlxResponseResult<Status> {
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
			create_library,
			read_library,
			update_library,
			delete_library
		]
	}
}

mod media {
	use super::*;
	use aedron_patchouli_common::{library::LibraryKind, media::*};

	#[non_exhaustive]
	#[derive(Debug, Clone, Responder)]
	enum ResponseMedia {
		Image(MsgPack<MediaImage>),
		Music(MsgPack<MediaMusic>),
	}
	macro_rules! responder_from_media {
		($($media:ty => $variant:ident),*) => {
			$(
				impl From<$media> for ResponseMedia {
					#[inline(always)]
					fn from(media: $media) -> Self {
						Self::$variant(MsgPack(media))
					}
				}
			)*
		};
	}
	responder_from_media! {
		MediaImage => Image,
		MediaMusic => Music
	}

	#[get("/<library>/<media>")]
	async fn read_media(
		mut db: Connection<Database>,
		library: u64,
		media: u64,
	) -> SqlxResponseResult<ResponseMedia> {
		let library = library as i64;
		let media = media as i64;
		let kind = super::fetch_library_kind(&mut db, library).await?;

		macro_rules! return_media {
			($($kind:ident => $query:expr => $media:ty),*) => {
				match kind {
					$(
						LibraryKind::$kind => Ok(<$media>::from(
							$query
							.fetch_one(&mut *db)
							.await
							.map_err(sqlx_response_err)?,
						).into()),
					)*
					_ => Err(Either::Left(io::ErrorKind::Unsupported.into())),
				}
			};
		}
		return_media! {
			Image => sqlx::query_as!(
				DbMediaImage,
				"SELECT id, title FROM media_image WHERE library = ? AND id = ?",
				library,
				media
			) => MediaImage,
			Music => sqlx::query_as!(
				DbMediaMusic,
				"SELECT id, title, artist, album, track FROM media_music WHERE library = ? AND id = ?",
				library,
				media
			) => MediaMusic
		}
	}

	#[inline]
	pub(super) fn routes() -> Vec<Route> {
		routes![read_media]
	}
}

#[must_use = "`Rocket<Build>` must be used"]
#[inline]
pub(super) fn mount(rocket: Rocket<Build>) -> Rocket<Build> {
	use aedron_patchouli_common::{library, media};

	rocket
		.mount(library::API_ENDPOINT, libraries::routes())
		.mount(media::API_ENDPOINT, self::media::routes())
}
