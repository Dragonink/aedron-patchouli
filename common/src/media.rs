//! Structs and server implementations to work with media

use const_format::concatcp;
use serde::{Deserialize, Serialize};

/// API endpoint for requests about media
pub const API_ENDPOINT: &str = concatcp!(super::API_BASE, "/media");

/// Common data for media types
pub trait Media {
	/// Get the media's ID in database
	fn id(&self) -> u64;

	/// Get the media's title
	fn title(&self) -> &str;
}

/// Image media type
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MediaImage {
	/// ID in database
	pub id: u64,
	/// Image title
	pub title: String,
}
impl Media for MediaImage {
	#[inline]
	fn id(&self) -> u64 {
		self.id
	}

	#[inline]
	fn title(&self) -> &str {
		&self.title
	}
}

/// Music media type
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MediaMusic {
	/// ID in database
	pub id: u64,
	/// Music title
	pub title: String,
	/// Music artist
	pub artist: Option<String>,
	/// Music album
	pub album: Option<String>,
	/// Music track number
	pub track: Option<u16>,
}
impl Media for MediaMusic {
	#[inline]
	fn id(&self) -> u64 {
		self.id
	}

	#[inline]
	fn title(&self) -> &str {
		&self.title
	}
}

#[cfg(feature = "server_impls")]
mod db_version {
	use super::*;

	/// Database version of [`MediaImage`]
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct DbMediaImage {
		/// See [`MediaImage::id`]
		pub id: i64,
		/// See [`MediaImage::title`]
		pub title: String,
	}
	impl From<MediaImage> for DbMediaImage {
		#[inline]
		fn from(media: MediaImage) -> Self {
			Self {
				id: media.id as i64,
				title: media.title,
			}
		}
	}
	impl From<DbMediaImage> for MediaImage {
		#[inline]
		fn from(db_media: DbMediaImage) -> Self {
			Self {
				id: db_media.id as u64,
				title: db_media.title,
			}
		}
	}

	/// Database version of [`MediaMusic`]
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub struct DbMediaMusic {
		/// See [`MediaMusic::id`]
		pub id: i64,
		/// See [`MediaMusic::title`]
		pub title: String,
		/// See [`MediaMusic::artist`]
		pub artist: Option<String>,
		/// See [`MediaMusic::album`]
		pub album: Option<String>,
		/// See [`MediaMusic::track`]
		pub track: Option<i64>,
	}
	impl From<MediaMusic> for DbMediaMusic {
		#[inline]
		fn from(media: MediaMusic) -> Self {
			Self {
				id: media.id as i64,
				title: media.title,
				artist: media.artist,
				album: media.album,
				track: media.track.map(|n| n as i64),
			}
		}
	}
	impl From<DbMediaMusic> for MediaMusic {
		#[inline]
		fn from(db_media: DbMediaMusic) -> Self {
			Self {
				id: db_media.id as u64,
				title: db_media.title,
				artist: db_media.artist,
				album: db_media.album,
				track: db_media.track.map(|n| n as u16),
			}
		}
	}
}
#[cfg(feature = "server_impls")]
pub use db_version::*;
