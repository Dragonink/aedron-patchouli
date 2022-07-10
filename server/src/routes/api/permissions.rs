use crate::{
	guards::RequiredAdminUser,
	routes::{perm_action_response_err, sqlx_response_err, SqlxResponseResult},
	Database,
};
use aedron_patchouli_common::permissions::*;
use rocket::{form::Form, http::Status, serde::msgpack::MsgPack, Route};
use rocket_db_pools::Connection;
use std::collections::HashMap;

#[get("/<id>")]
async fn read_permissions(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
) -> SqlxResponseResult<MsgPack<Vec<Permission>>> {
	let id = id as i64;
	let data = sqlx::query_as!(
		DbPermission,
		r#"SELECT library, user as "user?", action as "action?" FROM permissions WHERE library = ?"#,
		id
	)
	.fetch_all(&mut *db)
	.await
	.map_err(sqlx_response_err)?;

	Ok(MsgPack(
		data.into_iter()
			.map(TryFrom::try_from)
			.collect::<Result<_, _>>()
			.map_err(perm_action_response_err)?,
	))
}

#[put("/<id>", data = "<perms>")]
async fn write_permissions(
	mut db: Connection<Database>,
	_admin: RequiredAdminUser<'_>,
	id: u64,
	perms: Form<HashMap<&str, PermAction>>,
) -> SqlxResponseResult<Status> {
	let id = id as i64;
	for (&user, &action) in perms.iter() {
		let user = user.parse::<u64>().map(|n| n as i64).ok();
		let action = action.some().map(|action| action as i8);
		sqlx::query!(
			"INSERT OR REPLACE INTO permissions (library, user, action) VALUES (?, ?, ?)",
			id,
			user,
			action
		)
		.execute(&mut *db)
		.await
		.map_err(sqlx_response_err)?;
	}
	Ok(Status::NoContent)
}

#[inline]
pub(super) fn routes() -> Vec<Route> {
	routes![read_permissions, write_permissions]
}
