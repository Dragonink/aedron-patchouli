use crate::{
	guards::RequiredUser,
	routes::{fetch_library_kind, sqlx_response_err, SqlxResponseResult},
	Database,
};
use aedron_patchouli_common::{libraries::LibraryKind, media::*};
use either::Either;
use rocket::{serde::msgpack::MsgPack, Route};
use rocket_db_pools::Connection;
use std::io;

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
				#[inline]
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
	_user: RequiredUser<'_>,
	library: u64,
	media: u64,
) -> SqlxResponseResult<ResponseMedia> {
	let library = library as i64;
	let media = media as i64;
	let kind = fetch_library_kind(&mut db, library).await?;

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
