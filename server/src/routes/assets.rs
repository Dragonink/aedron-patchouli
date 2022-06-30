use super::{sqlx_response_err, SqlxResponseResult};
use crate::{guards::User, Database};
use either::Either;
use rocket::{fs::NamedFile, response::Redirect};
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
fn redirect_to_login_page() -> Redirect {
	Redirect::to(uri!(login_page))
}

#[inline]
fn accept_html(accept: &Accept) -> bool {
	use rocket::http::MediaType;

	accept.media_types().any(|mt| MediaType::HTML.eq(mt))
}

#[get("/<_..>", rank = 69)]
#[inline]
pub(super) fn index() -> Redirect {
	redirect_to_login_page()
}

#[cfg(debug_assertions)]
#[get("/<_..>", rank = 42)]
#[inline]
pub(super) async fn index_page(
	accept: &Accept,
	_user: &User,
) -> Either<io::Result<NamedFile>, Status> {
	if accept_html(accept) {
		Either::Left(NamedFile::open("client/out/index.html").await)
	} else {
		Either::Right(Status::NotFound)
	}
}
#[cfg(not(debug_assertions))]
#[get("/<_..>", rank = 42)]
#[inline]
pub(super) fn index_page(accept: &Accept, _user: &User) -> Either<RawHtml<&'static str>, Status> {
	const CONTENT: &str = include_str!("../../../client/out/index.html");

	if accept_html(accept) {
		Either::Left(RawHtml(CONTENT))
	} else {
		Either::Right(Status::NotFound)
	}
}

#[get("/login")]
#[inline]
pub(super) fn login(_user: &User) -> Redirect {
	Redirect::to(uri!(index_page("")))
}

#[cfg(debug_assertions)]
#[get("/login", rank = 2)]
pub(super) async fn login_page(accept: &Accept) -> Either<io::Result<NamedFile>, Status> {
	if accept_html(accept) {
		Either::Left(NamedFile::open("client/out/login.html").await)
	} else {
		Either::Right(Status::NotFound)
	}
}
#[cfg(not(debug_assertions))]
#[get("/login", rank = 2)]
#[inline]
pub(super) fn login_page(accept: &Accept) -> Either<RawHtml<&'static str>, Status> {
	const CONTENT: &str = include_str!("../../../client/out/login.html");

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

#[get("/media", rank = 2)]
#[inline]
fn media() -> Redirect {
	redirect_to_login_page()
}

#[get("/media?<library>&<file>")]
async fn media_resource(
	mut db: Connection<Database>,
	_user: &User,
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
	routes![
		index,
		index_page,
		login,
		login_page,
		js,
		wasm,
		media,
		media_resource
	]
}
