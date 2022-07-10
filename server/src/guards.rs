use crate::Database;
use rocket::{
	outcome::IntoOutcome,
	request::{FromRequest, Outcome, Request},
};
use rocket_db_pools::Connection;
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct User(pub u64);
impl User {
	pub(crate) const ADMIN_ID: u64 = 1;
	pub const COOKIE_NAME: &'static str = "user_id";

	#[inline(always)]
	pub const fn id(&self) -> u64 {
		self.0
	}

	#[inline(always)]
	pub const fn is_admin(&self) -> bool {
		self.id() == Self::ADMIN_ID
	}
}
#[async_trait]
impl<'r> FromRequest<'r> for &'r User {
	type Error = <Connection<Database> as FromRequest<'r>>::Error;

	async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
		use rocket::outcome::try_outcome;
		use rocket_db_pools::Connection;

		let mut db: Connection<Database> = try_outcome!(req.guard().await);
		let user = req
			.local_cache_async(async {
				let user_id = req
					.cookies()
					.get_private(User::COOKIE_NAME)
					.and_then(|cookie| cookie.value().parse::<u64>().ok())? as i64;
				let id: i64 = sqlx::query_scalar!("SELECT id FROM users WHERE id = ?", user_id)
					.fetch_one(&mut *db)
					.await
					.ok()?;
				Some(User(id as u64))
			})
			.await;
		user.as_ref().or_forward(())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct AdminUser<'r>(&'r User);
impl<'r> Deref for AdminUser<'r> {
	type Target = User;

	#[inline(always)]
	fn deref(&self) -> &Self::Target {
		self.0
	}
}
#[async_trait]
impl<'r> FromRequest<'r> for AdminUser<'r> {
	type Error = <&'r User as FromRequest<'r>>::Error;

	async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
		use rocket::outcome::try_outcome;

		let user: &'r User = try_outcome!(req.guard().await);
		if user.is_admin() {
			Outcome::Success(Self(user))
		} else {
			Outcome::Forward(())
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct RequiredUser<'r>(&'r User);
impl<'r> Deref for RequiredUser<'r> {
	type Target = User;

	#[inline(always)]
	fn deref(&self) -> &Self::Target {
		self.0
	}
}
#[async_trait]
impl<'r> FromRequest<'r> for RequiredUser<'r> {
	type Error = Option<<&'r User as FromRequest<'r>>::Error>;

	#[inline]
	async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
		use rocket::http::Status;

		match req.guard::<&'r User>().await {
			Outcome::Success(user) => Outcome::Success(RequiredUser(user)),
			Outcome::Forward(_) => Outcome::Failure((Status::Unauthorized, None)),
			Outcome::Failure((status, err)) => Outcome::Failure((status, Some(err))),
		}
	}
}
impl<'r> From<&'r User> for RequiredUser<'r> {
	#[inline(always)]
	fn from(user: &'r User) -> Self {
		Self(user)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct RequiredAdminUser<'r>(RequiredUser<'r>);
impl<'r> Deref for RequiredAdminUser<'r> {
	type Target = RequiredUser<'r>;

	#[inline(always)]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
#[async_trait]
impl<'r> FromRequest<'r> for RequiredAdminUser<'r> {
	type Error = <RequiredUser<'r> as FromRequest<'r>>::Error;

	async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
		use rocket::{http::Status, outcome::try_outcome};

		let user: RequiredUser = try_outcome!(req.guard().await);
		if user.is_admin() {
			Outcome::Success(Self(user))
		} else {
			Outcome::Failure((Status::Forbidden, None))
		}
	}
}
impl<'r> From<RequiredAdminUser<'r>> for RequiredUser<'r> {
	#[inline(always)]
	fn from(admin: RequiredAdminUser<'r>) -> Self {
		admin.0
	}
}
