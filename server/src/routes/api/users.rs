use crate::{
	guards::{RequiredAdminUser, RequiredUser, User as UserGuard},
	routes::{sqlx_response_err, SqlxResponseResult},
	Database,
};
use aedron_patchouli_common::users::*;
use either::Either;
use rocket::{
	form::{Form, Strict},
	http::{CookieJar, Status},
	response::status::Created,
	serde::msgpack::MsgPack,
	Route,
};
use rocket_db_pools::Connection;

#[get("/")]
async fn read_users(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
) -> SqlxResponseResult<MsgPack<Vec<User>>> {
	let data = sqlx::query_as!(DbUser, "SELECT id, name FROM users")
		.fetch_all(&mut *db)
		.await
		.map_err(sqlx_response_err)?;

	Ok(MsgPack(data.into_iter().map(From::from).collect()))
}

#[post("/", data = "<creds>")]
async fn create_user(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	creds: Form<Strict<UserCreds>>,
) -> SqlxResponseResult<Created<MsgPack<User>>> {
	let enc = crate::tasks::hash_passwd(&creds.passwd).map_err(|err| {
		Either::Right(match err {
			Either::Left(err) => err.to_string(),
			Either::Right(err) => err.to_string(),
		})
	})?;

	let user = sqlx::query_as!(
		DbUser,
		"INSERT INTO users (name, passwd) VALUES (?, ?) RETURNING id, name",
		creds.name,
		enc
	)
	.fetch_one(&mut *db)
	.await
	.map_err(sqlx_response_err)?;

	let user = User::from(user);
	Ok(Created::new(uri!(delete_user(user.id)).to_string()).body(MsgPack(user)))
}

#[get("/me")]
async fn read_user(
	mut db: Connection<Database>,
	user: RequiredUser<'_>,
) -> SqlxResponseResult<MsgPack<User>> {
	let id = user.id() as i64;
	let data = sqlx::query_as!(DbUser, "SELECT id, name FROM users WHERE id = ?", id)
		.fetch_one(&mut *db)
		.await
		.map_err(sqlx_response_err)?;

	Ok(MsgPack(data.into()))
}

#[get("/<id>")]
async fn admin_read_user(
	db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
) -> SqlxResponseResult<MsgPack<User>> {
	let user = UserGuard(id);
	read_user(db, (&user).into()).await
}

#[put("/me", data = "<data>")]
async fn update_user(
	mut db: Connection<Database>,
	jar: Option<&CookieJar<'_>>,
	user: RequiredUser<'_>,
	data: Form<UserConfig>,
) -> SqlxResponseResult<Status> {
	use rocket::{http::Cookie, serde::json};

	let id = user.id() as i64;
	if let Some(db_user) = sqlx::query_as!(
		DbUser,
		"UPDATE users SET name = ? WHERE id = ? RETURNING id, name",
		data.name,
		id
	)
	.fetch_optional(&mut *db)
	.await
	.map_err(sqlx_response_err)?
	{
		if let Some(jar) = jar {
			let mut cookie = Cookie::new(
				UserCookie::COOKIE_NAME,
				json::to_string(&UserCookie {
					is_admin: UserGuard(user.id()).is_admin(),
					name: db_user.name,
				})
				.map_err(|err| Either::Right(err.to_string()))?,
			);
			if let Some(expiry) = jar.get_private(UserGuard::COOKIE_NAME).unwrap().expires() {
				cookie.set_expires(expiry);
			}
			jar.add(cookie);
		}
		Ok(Status::NoContent)
	} else {
		Ok(Status::NotFound)
	}
}

#[put("/<id>", data = "<data>")]
async fn admin_update_user(
	db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
	data: Form<UserConfig>,
) -> SqlxResponseResult<Status> {
	let user = UserGuard(id);
	update_user(db, None, (&user).into(), data).await
}

#[delete("/<id>")]
async fn delete_user(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
) -> SqlxResponseResult<Status> {
	let id = id as i64;
	let affected = sqlx::query!("DELETE FROM users WHERE id = ?", id)
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
		read_users,
		create_user,
		read_user,
		admin_read_user,
		update_user,
		admin_update_user,
		delete_user
	]
}
