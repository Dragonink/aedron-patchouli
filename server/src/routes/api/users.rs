use crate::{
	guards::{RequiredAdminUser, RequiredUser},
	routes::{sqlx_response_err, SqlxResponseResult},
	Database,
};
use aedron_patchouli_common::users::*;
use either::Either;
use rocket::{
	form::{Form, Strict},
	http::Status,
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

#[get("/<id>")]
async fn admin_read_user(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
) -> SqlxResponseResult<MsgPack<User>> {
	let id = id as i64;
	let user = sqlx::query_as!(DbUser, "SELECT id, name FROM users WHERE id = ?", id)
		.fetch_one(&mut *db)
		.await
		.map_err(sqlx_response_err)?;

	Ok(MsgPack(user.into()))
}

#[get("/me")]
async fn read_user(
	mut db: Connection<Database>,
	user: RequiredUser<'_>,
) -> SqlxResponseResult<MsgPack<User>> {
	let data = sqlx::query_as!(DbUser, "SELECT id, name FROM users WHERE id = ?", user.id)
		.fetch_one(&mut *db)
		.await
		.map_err(sqlx_response_err)?;

	Ok(MsgPack(data.into()))
}

#[put("/<id>", data = "<data>")]
async fn update_user(
	mut db: Connection<Database>,
	user: RequiredUser<'_>,
	admin: Option<RequiredAdminUser<'_>>,
	id: u64,
	data: Form<UserConfig>,
) -> SqlxResponseResult<Status> {
	if user.id == id as i64 || admin.is_some() {
		let id = id as i64;
		let affected = sqlx::query!("UPDATE users SET name = ? WHERE id = ?", data.name, id)
			.execute(&mut *db)
			.await
			.map_err(sqlx_response_err)?
			.rows_affected();

		Ok(if affected > 0 {
			Status::NoContent
		} else {
			Status::NotFound
		})
	} else {
		Ok(Status::Forbidden)
	}
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
		admin_read_user,
		read_user,
		update_user,
		delete_user
	]
}
