//! Structs and server implementations to work with permissions

use const_format::concatcp;
use derive_try_from_primitive::TryFromPrimitive;
#[cfg(feature = "server_impls")]
use rocket::form::FromFormField;
use serde::{Deserialize, Serialize};
use std::ops::Not;

/// API endpoint for requests about permissions
pub const API_ENDPOINT: &str = concatcp!(super::API_BASE, "/permissions");

/// Action of a permission setting
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromPrimitive, Deserialize, Serialize)]
#[cfg_attr(feature = "server_impls", derive(FromFormField))]
#[serde(rename_all = "lowercase")]
#[repr(i8)]
pub enum PermAction {
	/// Inherit the access to the item
	#[default]
	Inherit = 0,
	/// Deny the access to the item
	Deny = -1,
	/// Allow the access to the item
	Allow = 1,
}
impl PermAction {
	/// Convert to an `Option<Self>`
	///
	/// If `self` is [`Self::Inherit`], this function returns [`None`].
	/// Otherwise, it returns `Some(self)`.
	#[inline]
	pub const fn some(self) -> Option<Self> {
		match self {
			Self::Inherit => None,
			x => Some(x),
		}
	}

	/// Return the action; or inherit from another one
	///
	/// If `self` is [`Self::Inherit`], this function returns `from`.
	/// Otherwise, it returns `self`.
	#[inline]
	pub fn or_inherit(self, from: Self) -> Self {
		self.some().unwrap_or(from)
	}

	/// Check if the action is [`Self::Allow`]
	///
	/// If `self` is [`Self::Inherit`], this function returns [`None`].
	#[inline]
	pub fn is_allow(self) -> Option<bool> {
		self.some().map(|action| action == Self::Allow)
	}
}
impl From<Option<PermAction>> for PermAction {
	#[inline]
	fn from(opt: Option<PermAction>) -> Self {
		opt.unwrap_or_default()
	}
}
impl Not for PermAction {
	type Output = Self;

	#[inline]
	fn not(self) -> Self::Output {
		(-(self as i8)).try_into().unwrap()
	}
}

/// Permission setting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Permission {
	/// ID in database of the library protected by this permission
	pub library: u64,
	/// ID in database of the user restricted by this permission
	pub user: Option<u64>,
	/// Action of this permission
	pub action: PermAction,
}

#[cfg(feature = "server_impls")]
mod db_version {
	use super::*;

	/// Database version of [`Permission`]
	pub struct DbPermission {
		/// See [`Permission::library`]
		pub library: i64,
		/// See [`Permission::user`]
		pub user: Option<i64>,
		/// See [`Permission::action`]
		pub action: Option<i64>,
	}
	impl From<Permission> for DbPermission {
		#[inline]
		fn from(perm: Permission) -> Self {
			Self {
				library: perm.library as i64,
				user: perm.user.map(|n| n as i64),
				action: perm.action.some().map(|action| action as i64),
			}
		}
	}
	impl TryFrom<DbPermission> for Permission {
		type Error = <PermAction as TryFrom<i8>>::Error;

		#[inline]
		fn try_from(db_perm: DbPermission) -> Result<Self, Self::Error> {
			Ok(Self {
				library: db_perm.library as u64,
				user: db_perm.user.map(|n| n as u64),
				action: db_perm
					.action
					.map(|n| (n as i8).try_into())
					.transpose()?
					.into(),
			})
		}
	}
}
#[cfg(feature = "server_impls")]
pub use db_version::*;
