//! Structs and server implementations to work with libraries

use crate::media::Media;
use const_format::concatcp;
use derive_try_from_primitive::TryFromPrimitive;
use enum_iterator::Sequence;
#[cfg(feature = "server_impls")]
use rocket::form::{FromForm, FromFormField};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// API endpoint for requests about libraries
pub const API_ENDPOINT: &str = concatcp!(super::API_BASE, "/libraries");

/// Type of media the library contains
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, Sequence, Deserialize, Serialize)]
#[cfg_attr(feature = "server_impls", derive(FromFormField))]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum LibraryKind {
	/// The library contains images
	Image = 0,
	/// The library contains music tracks
	Music = 1,
}
impl LibraryKind {
	/// Get the file extensions supported by the library type
	#[inline(always)]
	pub const fn extensions(&self) -> &'static [&'static str] {
		match self {
			Self::Image => &["jpg", "jpeg", "png"],
			Self::Music => &["mp3"],
		}
	}
}

/// Configuration of a library
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "server_impls", derive(FromForm))]
pub struct RawLibraryConfig {
	/// Displayed name
	#[cfg_attr(feature = "server_impls", field(validate = len(1..)))]
	pub name: String,
	/// Library type
	pub kind: LibraryKind,
	/// Paths used as the library's roots
	#[cfg_attr(feature = "server_impls", field(validate = len(1..)))]
	pub paths: Vec<String>,
}
impl From<LibraryConfig> for RawLibraryConfig {
	#[inline(always)]
	fn from(config: LibraryConfig) -> Self {
		Self {
			name: config.name,
			kind: config.kind,
			paths: config.paths,
		}
	}
}

/// [`RawLibraryConfig`] with database ID
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LibraryConfig {
	/// ID in database
	pub id: u64,
	/// Displayed name
	pub name: String,
	/// Library type
	pub kind: LibraryKind,
	/// Paths used as the library's roots
	pub paths: Vec<String>,
}

/// Partial info about a library
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PartialLibrary {
	/// ID in database
	pub id: u64,
	/// Displayed name
	pub name: String,
	/// Library type
	pub kind: LibraryKind,
}
impl From<LibraryConfig> for PartialLibrary {
	#[inline(always)]
	fn from(config: LibraryConfig) -> Self {
		Self {
			id: config.id,
			name: config.name,
			kind: config.kind,
		}
	}
}

/// Full data of a library
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Library<M: Media>
where
	M: Debug + Clone + Serialize,
{
	/// ID in database
	pub id: u64,
	/// Displayed name
	pub name: String,
	/// Library type
	pub kind: LibraryKind,
	/// Media contained in the library
	pub media: Vec<M>,
}
#[cfg(feature = "server_impls")]
impl<M: Media> Library<M>
where
	M: Debug + Clone + Serialize,
{
	/// Construct a new instance from a [`PartialLibrary`] and a vector of media
	#[inline(always)]
	pub fn new(lib: PartialLibrary, media: Vec<M>) -> Self {
		Self {
			id: lib.id,
			name: lib.name,
			kind: lib.kind,
			media,
		}
	}
}

#[cfg(feature = "server_impls")]
mod db_version {
	use super::*;

	/// Database version of [`RawLibraryConfig`]
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct DbRawLibraryConfig {
		/// See [`LibraryConfig::name`]
		pub name: String,
		/// See [`LibraryConfig::kind`]
		pub kind: i64,
		/// See [`LibraryConfig::paths`]
		pub paths: String,
	}
	impl DbRawLibraryConfig {
		/// Character used to separate paths in database
		pub const PATH_SEPARATOR: char = ':';

		/// Join a vector of paths into a single string using [`Self::PATH_SEPARATOR`]
		///
		/// This function is the inverse of [`Self::split_paths`].
		#[inline]
		pub fn join_paths(paths: Vec<String>) -> String {
			paths.join(&Self::PATH_SEPARATOR.to_string())
		}

		/// Split a string into a vector of paths using [`Self::PATH_SEPARATOR`]
		///
		/// This function is the inverse of [`Self::join_paths`].
		#[inline]
		pub fn split_paths(paths: String) -> Vec<String> {
			paths
				.split(Self::PATH_SEPARATOR)
				.map(|s| s.to_string())
				.collect()
		}
	}
	impl From<RawLibraryConfig> for DbRawLibraryConfig {
		#[inline]
		fn from(config: RawLibraryConfig) -> Self {
			Self {
				name: config.name,
				kind: config.kind as i64,
				paths: Self::join_paths(config.paths),
			}
		}
	}
	impl TryFrom<DbRawLibraryConfig> for RawLibraryConfig {
		type Error = <LibraryKind as TryFrom<u8>>::Error;

		#[inline]
		fn try_from(db_config: DbRawLibraryConfig) -> Result<Self, Self::Error> {
			Ok(Self {
				name: db_config.name,
				kind: (db_config.kind as u8).try_into()?,
				paths: DbRawLibraryConfig::split_paths(db_config.paths),
			})
		}
	}
	impl From<DbLibraryConfig> for DbRawLibraryConfig {
		#[inline(always)]
		fn from(db_config: DbLibraryConfig) -> Self {
			Self {
				name: db_config.name,
				kind: db_config.kind,
				paths: db_config.paths,
			}
		}
	}

	/// Database version of [`LibraryConfig`]
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct DbLibraryConfig {
		/// See [`LibraryConfig::id`]
		pub id: i64,
		/// See [`LibraryConfig::name`]
		pub name: String,
		/// See [`LibraryConfig::kind`]
		pub kind: i64,
		/// See [`LibraryConfig::paths`]
		pub paths: String,
	}
	impl From<LibraryConfig> for DbLibraryConfig {
		#[inline]
		fn from(config: LibraryConfig) -> Self {
			Self {
				id: config.id as i64,
				name: config.name,
				kind: config.kind as i64,
				paths: DbRawLibraryConfig::join_paths(config.paths),
			}
		}
	}
	impl TryFrom<DbLibraryConfig> for LibraryConfig {
		type Error = <LibraryKind as TryFrom<u8>>::Error;

		fn try_from(db_config: DbLibraryConfig) -> Result<Self, Self::Error> {
			Ok(Self {
				id: db_config.id as u64,
				name: db_config.name,
				kind: (db_config.kind as u8).try_into()?,
				paths: DbRawLibraryConfig::split_paths(db_config.paths),
			})
		}
	}

	/// Database version of [`PartialLibrary`]
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct DbPartialLibrary {
		/// See [`PartialLibrary::id`]
		pub id: i64,
		/// See [`PartialLibrary::name`]
		pub name: String,
		/// See [`PartialLibrary::kind`]
		pub kind: i64,
	}
	impl From<PartialLibrary> for DbPartialLibrary {
		#[inline(always)]
		fn from(lib: PartialLibrary) -> Self {
			Self {
				id: lib.id as i64,
				name: lib.name,
				kind: lib.kind as i64,
			}
		}
	}
	impl TryFrom<DbPartialLibrary> for PartialLibrary {
		type Error = <LibraryKind as TryFrom<u8>>::Error;

		#[inline]
		fn try_from(db_lib: DbPartialLibrary) -> Result<Self, Self::Error> {
			Ok(Self {
				id: db_lib.id as u64,
				name: db_lib.name,
				kind: (db_lib.kind as u8).try_into()?,
			})
		}
	}
	impl From<DbLibraryConfig> for DbPartialLibrary {
		#[inline(always)]
		fn from(db_config: DbLibraryConfig) -> Self {
			Self {
				id: db_config.id,
				name: db_config.name,
				kind: db_config.kind,
			}
		}
	}
}
#[cfg(feature = "server_impls")]
pub use db_version::*;
