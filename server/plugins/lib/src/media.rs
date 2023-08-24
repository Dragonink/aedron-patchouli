//! Provides the types for media plugins

use crate::{ffi::*, Version};
pub use time::{Date, Time};

/// Version of the media plugin library
pub const PLUGLIB_VERSION: Version = Version {
	major: 0,
	minor: 1,
	patch: 0,
};

/// Signature of the `describe_media` function that media plugins must export
pub type DescribeMedia = extern "C" fn() -> Media;
/// Signature of the `extract_metadata` function that media plugins must export
pub type ExtractMetadata =
	extern "C" fn(path: FfiStr<'_>) -> FfiResult<FfiBoxedSlice<FfiOption<MetadataFieldValue>>, ()>;

/// Description of the media type provided by the plugin
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Media {
	/// Name of the media, for display purposes
	pub name: FfiStr<'static>,
	/// Identifier of the media, for data purposes
	pub ident: FfiStr<'static>,
	/// Metadata fields of the media
	pub fields: FfiBoxedSlice<MetadataField>,
}

/// Description of a metadata field
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MetadataField {
	/// Name of the field, for display purposes
	pub name: FfiStr<'static>,
	/// Identifier of the field, for data purposes
	pub ident: FfiStr<'static>,
	/// Data type of the field
	pub r#type: MetadataFieldType,
	/// Is the field a list of values?
	pub is_list: bool,
}

/// Data type of a [`MetadataField`]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataFieldType {
	/// 32-bit signed integer value
	Integer,
	/// 64-bit signed integer value
	BigInteger,
	/// 64-bit floating point value
	Real,
	/// String value
	Text,
	/// Binary data
	Blob,
	/// Boolean value
	Boolean,
	/// Date value
	Date,
	/// Time value
	Time,
}
#[cfg(feature = "server")]
impl MetadataFieldType {
	/// Returns the associated SQL type
	#[inline]
	pub const fn to_sql(&self) -> &'static str {
		match self {
			Self::Integer => "INTEGER",
			Self::BigInteger => "BIGINT",
			Self::Real => "REAL",
			Self::Text => "TEXT",
			Self::Blob => "BLOB",
			Self::Boolean => "BOOLEAN",
			Self::Date => "DATE",
			Self::Time => "TIME",
		}
	}
}

/// Data storage of a [`MetadataField`]
#[repr(C)]
#[derive(Debug, Clone)]
pub enum MetadataFieldValue {
	/// 32-bit signed integer value
	Integer(i32),
	/// 64-bit signed integer value
	BigInteger(i64),
	/// 64-bit floating point value
	Real(f64),
	/// String value
	Text(FfiString),
	/// Binary data
	Blob(FfiBoxedSlice<u8>),
	/// Boolean value
	Boolean(bool),
	/// Date value
	Date(i32),
	/// Time value
	Time(FfiTime),
	/// List of values
	List(FfiBoxedSlice<Self>),
}
impl From<i32> for MetadataFieldValue {
	#[inline]
	fn from(value: i32) -> Self {
		Self::Integer(value)
	}
}
impl From<i64> for MetadataFieldValue {
	#[inline]
	fn from(value: i64) -> Self {
		Self::BigInteger(value)
	}
}
impl From<f64> for MetadataFieldValue {
	#[inline]
	fn from(value: f64) -> Self {
		Self::Real(value)
	}
}
impl From<FfiString> for MetadataFieldValue {
	#[inline]
	fn from(value: FfiString) -> Self {
		Self::Text(value)
	}
}
impl TryFrom<Box<str>> for MetadataFieldValue {
	type Error = <FfiString as TryFrom<Box<str>>>::Error;

	#[inline]
	fn try_from(value: Box<str>) -> Result<Self, Self::Error> {
		FfiString::try_from(value).map(Self::from)
	}
}
impl TryFrom<&str> for MetadataFieldValue {
	type Error = <Self as TryFrom<Box<str>>>::Error;

	#[inline]
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		Self::try_from(Box::<str>::from(value))
	}
}
impl From<FfiBoxedSlice<u8>> for MetadataFieldValue {
	#[inline]
	fn from(value: FfiBoxedSlice<u8>) -> Self {
		Self::Blob(value)
	}
}
impl From<Box<[u8]>> for MetadataFieldValue {
	#[inline]
	fn from(value: Box<[u8]>) -> Self {
		Self::from(FfiBoxedSlice::from(value))
	}
}
impl From<&[u8]> for MetadataFieldValue {
	#[inline]
	fn from(value: &[u8]) -> Self {
		Self::from(Box::<[u8]>::from(value))
	}
}
impl From<bool> for MetadataFieldValue {
	#[inline]
	fn from(value: bool) -> Self {
		Self::Boolean(value)
	}
}
impl From<Date> for MetadataFieldValue {
	#[inline]
	fn from(value: Date) -> Self {
		Self::Date(value.to_julian_day())
	}
}
impl From<FfiTime> for MetadataFieldValue {
	#[inline]
	fn from(value: FfiTime) -> Self {
		Self::Time(value)
	}
}
impl From<Time> for MetadataFieldValue {
	#[inline]
	fn from(value: Time) -> Self {
		Self::from(FfiTime::from(value))
	}
}
impl From<FfiBoxedSlice<MetadataFieldValue>> for MetadataFieldValue {
	#[inline]
	fn from(list: FfiBoxedSlice<MetadataFieldValue>) -> Self {
		Self::List(list)
	}
}

#[doc(hidden)]
#[macro_export]
macro_rules! new_metadata_field {
	($ident:ident $name:literal : $type:ident) => {
		$crate::media::new_metadata_field!($name, $ident, $type, false)
	};
	($ident:ident $name:literal : $type:ident list) => {
		$crate::media::new_metadata_field!($name, $ident, $type, true)
	};
	($name:literal, $ident:ident, $type:ident, $is_list:expr) => {
		$crate::media::MetadataField {
			name: $crate::ffi::new_ffistr!($name),
			ident: $crate::ffi::new_ffistr!(::core::stringify!($ident)),
			r#type: $crate::media::MetadataFieldType::$type,
			is_list: $is_list,
		}
	};
}
/// Utility macro that creates a media plugin
#[macro_export]
macro_rules! make_plugin {
	(
		$media_ident:ident $media_name:literal ;
		$( $field_ident:ident $field_name:literal : $( $field_type:ident )+ ),* $(,)?
	) => {
		$crate::media::assert_plugin!();

		/// Version of the plugin library
		#[no_mangle]
		pub static PLUGLIB_VERSION: $crate::Version = $crate::media::PLUGLIB_VERSION;

		/// Returns the plugin's version
		#[no_mangle]
		pub extern "C" fn plugin_version() -> $crate::Version {
			::core::env!("CARGO_PKG_VERSION").parse().unwrap_or_default()
		}

		/// Returns a description of the media type provided by the plugin
		#[no_mangle]
		pub extern "C" fn describe_media() -> $crate::media::Media {
			$crate::media::Media {
				name: $crate::ffi::new_ffistr!($media_name),
				ident: $crate::ffi::new_ffistr!(::core::stringify!($media_ident)),
				fields: [
					$( $crate::media::new_metadata_field!($field_ident $field_name : $( $field_type )+) ),*
				].into_iter().collect(),
			}
		}
	};
}

/// Asserts that the media plugin export the correct symbols
///
/// You need not use this macro if you created your plugin with [`make_plugin`].
#[macro_export]
macro_rules! assert_plugin {
	() => {
		#[doc(hidden)]
		mod asserts {
			use super::*;

			static _ASSERT_PLUGLIB_VERSION: $crate::Version = PLUGLIB_VERSION;

			const _ASSERT_PLUGIN_VERSION: $crate::PluginVersion = plugin_version;

			const _ASSERT_DESCRIBE_MEDIA: $crate::media::DescribeMedia = describe_media;

			const _ASSERT_EXTRACT_METADATA: $crate::media::ExtractMetadata = extract_metadata;
		}
	};
}

pub use {assert_plugin, make_plugin, new_metadata_field};
