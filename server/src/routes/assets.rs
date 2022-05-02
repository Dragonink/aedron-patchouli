use super::{sqlx_response_err, SqlxResponseResult};
use crate::Database;
use either::Either;
use rocket::fs::NamedFile;
#[cfg(not(debug_assertions))]
use rocket::{
	http::ContentType,
	response::content::{RawHtml, RawJavaScript},
};
use rocket::{
	http::{Accept, Status},
	Route,
};
use rocket_db_pools::Connection;
#[cfg(debug_assertions)]
use std::io;

#[inline]
fn accept_html(accept: &Accept) -> bool {
	use rocket::http::MediaType;

	accept.media_types().any(|mt| MediaType::HTML.eq(mt))
}

#[cfg(debug_assertions)]
#[get("/<_..>", rank = 69)]
#[inline]
async fn html(accept: &Accept) -> Either<io::Result<NamedFile>, Status> {
	if accept_html(accept) {
		Either::Left(NamedFile::open("client/out/index.html").await)
	} else {
		Either::Right(Status::NotFound)
	}
}
#[cfg(not(debug_assertions))]
#[get("/<_..>", rank = 69)]
#[inline]
fn html(accept: &Accept) -> Either<RawHtml<&'static str>, Status> {
	const CONTENT: &str = include_str!("../../../client/out/index.html");

	if accept_html(accept) {
		Either::Left(RawHtml(CONTENT))
	} else {
		Either::Right(Status::NotFound)
	}
}

#[cfg(debug_assertions)]
#[get("/assets/index.js")]
#[inline]
async fn js() -> io::Result<NamedFile> {
	NamedFile::open("client/out/index.js").await
}
#[cfg(not(debug_assertions))]
#[get("/assets/index.js")]
#[inline(always)]
const fn js() -> RawJavaScript<&'static str> {
	const CONTENT: &str = include_str!("../../../client/out/index.js");

	RawJavaScript(CONTENT)
}

#[cfg(debug_assertions)]
#[get("/assets/index_bg.wasm")]
#[inline]
async fn wasm() -> io::Result<NamedFile> {
	NamedFile::open("client/out/index_bg.wasm").await
}
#[cfg(not(debug_assertions))]
#[get("/assets/index_bg.wasm")]
#[inline(always)]
const fn wasm() -> (ContentType, &'static [u8]) {
	const CONTENT: &[u8] = include_bytes!("../../../client/out/index_bg.wasm");

	(ContentType::WASM, CONTENT)
}

#[get("/media?<library>&<file>")]
async fn media(
	mut db: Connection<Database>,
	library: u64,
	file: u64,
) -> SqlxResponseResult<NamedFile> {
	use std::path::PathBuf;

	let library = library as i64;
	let file = file as i64;
	let kind = super::fetch_library_kind(&mut db, library).await?;
	let path: PathBuf = sqlx::query_scalar::<_, String>(&format!(
		"SELECT path FROM media_{} WHERE id = {file} AND library = {library}",
		format!("{kind:?}").to_ascii_lowercase()
	))
	.fetch_one(&mut *db)
	.await
	.map_err(sqlx_response_err)?
	.into();

	NamedFile::open(path).await.map_err(Either::Left)
}

#[inline]
pub(crate) fn routes() -> Vec<Route> {
	routes![html, js, wasm, media]
}
