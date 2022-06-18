//! Structs and server implementations to work with users

#[cfg(feature = "server_impls")]
use rocket::form::FromForm;
use serde::{Deserialize, Serialize};

/// API endpoint for requests about users
pub const API_ENDPOINT: &str = constcat!(super::API_BASE, "/users");

/// User credentials
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "server_impls", derive(FromForm))]
pub struct UserCreds {
	/// Username
	#[cfg_attr(feature = "server_impls", field(validate = len(1..)))]
	pub name: String,
	/// Password
	#[cfg_attr(feature = "server_impls", field(validate = len(1..)))]
	pub passwd: String,
}

/// Configuration of a user profile
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "server_impls", derive(FromForm))]
pub struct UserConfig {
	/// Username
	#[cfg_attr(feature = "server_impls", field(validate = len(1..)))]
	pub name: String,
}

/// Properties of a user
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct User {
	/// ID in database
	pub id: u64,
	/// Username
	pub name: String,
}

#[cfg(feature = "server_impls")]
mod db_version {
	use super::*;

	/// Database version of [`User`]
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct DbUser {
		/// See [`User::id`]
		pub id: i64,
		/// See [`User::name`]
		pub name: String,
	}
	impl From<User> for DbUser {
		#[inline(always)]
		fn from(user: User) -> Self {
			Self {
				id: user.id as i64,
				name: user.name,
			}
		}
	}
	impl From<DbUser> for User {
		#[inline(always)]
		fn from(db_user: DbUser) -> Self {
			Self {
				id: db_user.id as u64,
				name: db_user.name,
			}
		}
	}
}
#[cfg(feature = "server_impls")]
pub use db_version::*;
