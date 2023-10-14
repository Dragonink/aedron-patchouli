//! Provides the [`MediaPlugin`] struct

use super::{DbPlugin, Plugin, PluginKind, PluginLoadError};
use crate::config::MediaConfig;
use libloading::{Library, Symbol};
use pluglib::{
	media::{DescribeMedia, ExtractMetadata, Media, MetadataFieldValue, SupportedTypes},
	PluginVersion, Version,
};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rayon::prelude::*;
use rusqlite::ToSql;
use std::{
	collections::{HashMap, HashSet},
	error::Error,
	fmt::{self, Debug, Display, Formatter},
	path::Path,
	sync::{mpsc, Arc, Mutex},
	time::Instant,
};
use time::OffsetDateTime;
use walkdir::WalkDir;

/// Structure of a [media plugin](pluglib::media)
pub(crate) struct MediaPlugin {
	/// Dynamic library
	lib: Library,

	/// Name of the plugin
	pub(crate) name: Box<str>,
	/// Version of the plugin
	pub(crate) version: Version,

	/// Description of the media type provided by the plugin
	pub(crate) media: Media,
}
impl TryFrom<&Path> for MediaPlugin {
	type Error = PluginLoadError;

	fn try_from(path: &Path) -> Result<Self, Self::Error> {
		// SAFETY: Upheld by the plugin
		let lib = unsafe { Library::new(path)? };

		let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into()) else {
			unreachable!()
		};

		// SAFETY: Upheld by the plugin
		let pluglib_version = unsafe { &**lib.get::<*const Version>(b"PLUGLIB_VERSION\0")? };
		if !pluglib::media::PLUGLIB_VERSION.is_compatible(pluglib_version) {
			return Err(PluginLoadError::IncompatibleLibVersions {
				kind: PluginKind::Media,
				name,
				plugin: *pluglib_version,
			});
		}

		// SAFETY: Upheld by the plugin
		let plugin_version = unsafe { lib.get::<PluginVersion>(b"plugin_version\0")? };
		let version = plugin_version();

		// SAFETY: Upheld by the plugin
		let describe_plugin = unsafe { lib.get::<DescribeMedia>(b"describe_media\0")? };
		let media = describe_plugin();

		// SAFETY: Upheld by the plugin
		unsafe {
			lib.get::<SupportedTypes>(Self::SUPPORTED_TYPES)?;
			lib.get::<ExtractMetadata>(Self::EXTRACT_METADATA)?;
		}

		Ok(Self {
			lib,
			name,
			version,
			media,
		})
	}
}
impl MediaPlugin {
	/// Symbol of the [`SupportedTypes`] function
	const SUPPORTED_TYPES: &[u8] = b"supported_types\0";
	/// Symbol of the [`ExtractMetadata`] function
	const EXTRACT_METADATA: &[u8] = b"extract_metadata\0";

	/// Lists the types supported by the plugin
	#[inline]
	pub(super) fn supported_types(&self) -> Symbol<'_, SupportedTypes> {
		// SAFETY: Upheld by plugin
		unsafe {
			self.lib
				.get(Self::SUPPORTED_TYPES)
				.unwrap_or_else(|_err| unreachable!())
		}
	}

	/// Extracts the metadata of the given media
	#[inline]
	pub(super) fn extract_metadata(&self) -> Symbol<'_, ExtractMetadata> {
		// SAFETY: Upheld by plugin
		unsafe {
			self.lib
				.get(Self::EXTRACT_METADATA)
				.unwrap_or_else(|_err| unreachable!())
		}
	}

	/// Returns the identifier of the database table
	pub(crate) fn table_ident(&self) -> String {
		format!("media_{}", self.media.ident)
	}

	/// Loads media files using this plugin
	///
	/// # Panics
	/// This function panics if a [`libloading::Error`] occurs.
	pub(super) fn load_media(
		&self,
		mut conn: PooledConnection<SqliteConnectionManager>,
		config: &MediaConfig,
	) -> rusqlite::Result<()> {
		let extract_metadata = self.extract_metadata();
		let supported_types = self.supported_types();

		// List supported types
		let supported_types = supported_types();
		let supported_types = supported_types
			.iter()
			.map(|s| s.to_str())
			.collect::<HashSet<_>>();
		log::debug!("Supported MIME types by {self}: {supported_types:?}");

		// List previously cached media
		let cached_media = {
			let mut stmt = conn.prepare(&format!(
				"SELECT path, mtime FROM {table}",
				table = self.table_ident(),
			))?;
			let rows = stmt.query_map((), |row| Ok((row.get(0)?, row.get(1)?)))?;
			let ret = rows.collect::<rusqlite::Result<HashMap<String, OffsetDateTime>>>()?;
			stmt.finalize()?;
			ret
		};
		log::debug!("{cached_media:?}");
		let cached_count = cached_media.len();
		log::debug!("{cached_count} {} media are cached", self.media.name);
		let cached_media = Arc::new(Mutex::new(cached_media));

		// Prepare database update
		let transaction = conn.transaction()?;

		let mut fields = vec!["path", "mtime"];
		fields.extend(self.media.fields.iter().map(|field| field.ident.to_str()));
		let mut value_binds = vec!["?"; 2];
		value_binds.extend(self.media.fields.iter().map(|field| {
			if field.is_list {
				"ifnull(?, json_array())"
			} else {
				"?"
			}
		}));
		let mut stmt = transaction.prepare(&format!(
			"INSERT INTO {table}({fields}) VALUES ({value_binds})",
			table = self.table_ident(),
			fields = fields.join(", "),
			value_binds = value_binds.join(", "),
		))?;

		// List all media
		let (tx, rx) = mpsc::channel();
		let start = Instant::now();
		config.paths.par_iter().for_each_with(
			(Arc::clone(&cached_media), tx),
			|(cached_media, tx), path| {
				log::info!(
					"Searching {:?} for {} media...",
					path.display(),
					self.media.name
				);
				WalkDir::new(path)
					.follow_links(true)
					.into_iter()
					.filter_entry(|entry| {
						#[cfg(unix)]
						{
							entry
								.file_name()
								.to_str()
								.map_or(false, |s| !s.starts_with('.'))
						}
						#[cfg(not(unix))]
						{
							true
						}
					})
					.par_bridge()
					.filter_map(|res| {
						let entry = res.ok()?;
						if entry.metadata().ok()?.is_dir() {
							return None;
						}
						if let Some(mime) = entry.file_name().to_str().and_then(mime_db::lookup) {
							if supported_types.contains(mime) {
								return Some(entry);
							}
						}
						None
					})
					.filter_map(|entry| {
						let mtime = entry
							.metadata()
							.map_err(From::from)
							.and_then(|meta| meta.modified())
							.expect("the last modification time of a file should be available");
						let mut path = entry.into_path().into_os_string().into_string().ok()?;

						if cached_media
							.lock()
							.unwrap()
							.remove(path.as_str())
							.map(|cached_mtime| OffsetDateTime::from(mtime) <= cached_mtime)
							.unwrap_or_default()
						{
							log::debug!("Skipping {path:?}");
							return None;
						}

						path.push('\0');
						let metadata = extract_metadata(
							path.as_str()
								.try_into()
								.unwrap_or_else(|_err| unreachable!()),
						);
						path.pop();
						match metadata.into() {
							Ok(data) => {
								log::trace!("Extracted metadata from {path:?}: {data:?}");
								Some((path, mtime, data))
							}
							Err(()) => {
								log::warn!("Could not extract metadata from {path:?}");
								None
							}
						}
					})
					.for_each(|(path, mtime, data)| {
						let mut values: Vec<Box<dyn ToSql + Send + Sync>> = vec![
							Box::new(path.clone()),
							Box::new(OffsetDateTime::from(mtime)),
						];
						values.extend(data.into_iter().cloned().map(|value| {
							Box::new(Option::<MetadataFieldValue>::from(value))
								as Box<dyn ToSql + Send + Sync>
						}));
						tx.send((path, values))
							.unwrap_or_else(|_err| unreachable!());
					});
			},
		);

		// Update database
		let cached_media = cached_media.lock().unwrap();
		let added_count = rx
			.into_iter()
			.map(|(path, values)| {
				stmt.execute(rusqlite::params_from_iter(values))
					.unwrap_or_else(|err| {
						log::trace!("Could not insert media {path:?}: {err}");
						0
					})
			})
			.sum::<usize>();
		stmt.finalize()?;
		let removed_count = transaction.execute(
			&format!(
				"DELETE FROM {table} WHERE path IN ({})",
				vec!["?"; cached_media.len()].join(", "),
				table = self.table_ident(),
			),
			rusqlite::params_from_iter(cached_media.keys()),
		)?;
		log::info!(
			"Added {added_count}, kept {}, removed {removed_count} {} media in {:.3}s",
			cached_count - removed_count,
			self.media.name,
			start.elapsed().as_secs_f32(),
		);

		transaction.commit()
	}
}
impl Debug for MediaPlugin {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{self} ({:?})", self.lib)
	}
}
impl Display for MediaPlugin {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "media plugin <{} {}>", self.name, self.version)
	}
}
impl Plugin for MediaPlugin {
	fn update_database(
		&self,
		db_pool: &Pool<SqliteConnectionManager>,
		db_plugin: DbPlugin,
	) -> Result<(), Box<dyn Error>> {
		let mut conn = db_pool.get()?;
		let transaction = conn.transaction()?;

		let mut fields = vec![
			"path TEXT NOT NULL PRIMARY KEY ON CONFLICT REPLACE".to_owned(),
			"mtime TEXT NOT NULL".to_owned(),
		];
		fields.extend(self.media.fields.iter().map(|field| {
			format!(
				"{} {}",
				field.ident,
				if field.is_list {
					"TEXT NOT NULL DEFAULT (json_array())"
				} else {
					field.r#type.to_sql()
				}
			)
		}));

		transaction.execute_batch(
			format!(
				"
					DROP TABLE IF EXISTS {table};
					CREATE TABLE {table} ({}) STRICT, WITHOUT ROWID;
				",
				fields.join(","),
				table = self.table_ident(),
			)
			.trim(),
		)?;
		transaction.execute(
			"INSERT INTO plugins(name, kind, version) VALUES (:name, :kind, :version)",
			rusqlite::named_params! {
				":name": db_plugin.name,
				":kind": db_plugin.kind,
				":version": db_plugin.version,
			},
		)?;

		transaction.commit().map_err(From::from)
	}
}
