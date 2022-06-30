use crate::Database;
use aedron_patchouli_common::libraries::LibraryKind;
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

#[inline]
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

mod user_session {
	use crate::{guards::User, Database};
	use aedron_patchouli_common::users::{UserCookie, UserCreds};
	use rocket::{
		form::{Form, Strict},
		http::CookieJar,
		response::{Flash, Redirect},
		Route,
	};
	use rocket_db_pools::Connection;

	struct SecUser {
		id: i64,
		name: String,
		passwd: String,
	}
	impl From<SecUser> for UserCookie {
		#[inline(always)]
		fn from(user: SecUser) -> Self {
			Self {
				is_admin: User(user.id as u64).is_admin(),
				name: user.name,
			}
		}
	}

	#[post("/login", data = "<creds>")]
	async fn login(
		mut db: Connection<Database>,
		jar: &CookieJar<'_>,
		creds: Form<Strict<UserCreds>>,
	) -> Result<Redirect, Flash<Redirect>> {
		use aedron_patchouli_common::users::UserCookie;
		use rocket::{http::Cookie, serde::json};
		use time::OffsetDateTime;

		let flash_error = || {
			Flash::error(
				Redirect::to(uri!(super::assets::login_page)),
				"Something went wrong. Please try again.",
			)
		};

		let user = sqlx::query_as!(SecUser, "SELECT * FROM users WHERE name = ?", creds.name)
			.fetch_one(&mut *db)
			.await
			.map_err(|err| {
				console_warn!("Database error", "{err}");
				flash_error()
			})?;
		match argon2::verify_encoded(&user.passwd, creds.passwd.as_bytes()) {
			Ok(true) => {
				let expire = OffsetDateTime::now_utc() + UserCookie::EXPIRE;
				jar.add_private(
					Cookie::build(User::COOKIE_NAME, format!("{}", user.id))
						.secure(true)
						.expires(expire)
						.finish(),
				);
				jar.add(
					Cookie::build(
						UserCookie::COOKIE_NAME,
						json::to_string(&UserCookie::from(user)).map_err(|_err| flash_error())?,
					)
					.expires(expire)
					.finish(),
				);
				Ok(Redirect::to(uri!(super::assets::index_page(""))))
			}
			Ok(false) => Err(Flash::error(
				Redirect::to(uri!(super::assets::login_page)),
				"Invalid password.",
			)),
			Err(err) => {
				console_warn!("Crypto error", "{err}");
				Err(flash_error())
			}
		}
	}

	#[post("/logout")]
	fn logout(jar: &CookieJar<'_>, _user: &User) -> Flash<Redirect> {
		use rocket::http::Cookie;

		jar.remove_private(Cookie::named(User::COOKIE_NAME));
		jar.remove(Cookie::named(UserCookie::COOKIE_NAME));
		Flash::success(
			Redirect::to(uri!(super::assets::login_page)),
			"Successfully logged out.",
		)
	}

	#[inline]
	pub(super) fn routes() -> Vec<Route> {
		routes![login, logout]
	}
}

#[must_use = "`Rocket<Build>` must be used"]
#[inline]
pub(crate) fn mount(rocket: Rocket<Build>) -> Rocket<Build> {
	api::mount(
		rocket
			.mount("/", assets::routes())
			.mount("/", user_session::routes()),
	)
}
