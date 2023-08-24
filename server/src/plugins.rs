#![allow(unsafe_code)]
//! Provides the server's plugin features

use crate::EXE_NAME;
use libloading::{Library, Symbol};
use pluglib::{
	media::{DescribeMedia, ExtractMetadata},
	PluginVersion, Version,
};
use sqlx::{pool::PoolConnection, types::Json, FromRow, QueryBuilder, Sqlite, Type};
use std::{
	collections::{HashMap, HashSet},
	error::Error,
	fmt::{self, Debug, Display, Formatter},
	hash::{Hash, Hasher},
	path::{Path, PathBuf},
};

/// Stores all plugins
#[derive(Debug, Default)]
pub(super) struct PluginStore {
	/// Stores media plugins
	pub(super) media: HashMap<Box<str>, MediaPlugin>,
}
impl PluginStore {
	/// Returns the directories to search plugins in
	fn get_plugin_dirs() -> Vec<PathBuf> {
		let mut dirs = Vec::new();

		/// Name of the plugins directory
		const PLUGINS_DIR: &str = "plugins";
		match std::env::current_exe() {
			Ok(path) => {
				dirs.push(
					path.parent()
						.map(ToOwned::to_owned)
						.unwrap_or_default()
						.join(PLUGINS_DIR),
				);
			}
			Err(err) => {
				log::warn!("Could not get the exe's directory: {err}");
			}
		}
		if let Some(data_dir) = std::env::var_os("XDG_DATA_HOME") {
			dirs.push(PathBuf::from(data_dir).join(EXE_NAME).join(PLUGINS_DIR));
		}
		if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
			dirs.extend(
				std::env::split_paths(&data_dirs).map(|path| path.join(EXE_NAME).join(PLUGINS_DIR)),
			);
		}

		dirs
	}

	/// Finds and loads all plugins
	pub(super) fn load_plugins() -> Self {
		let mut this = Self::default();

		log::debug!("Media plugin library {}", pluglib::media::PLUGLIB_VERSION);

		Self::get_plugin_dirs()
			.into_iter()
			.filter_map(|dir| match std::fs::read_dir(&dir) {
				Ok(dir) => Some(dir),
				Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => None,
				Err(err) => {
					log::warn!("Could not read {}: {err}", dir.display());
					None
				}
			})
			.flatten()
			.filter_map(|res| res.ok())
			.for_each(|entry| {
				let path = entry.path();
				let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into()) else {
					log::debug!("Could not extract the name from {}", path.display());
					return;
				};
				#[allow(clippy::single_match)]
				match path.extension().and_then(|s| s.to_str()) {
					Some("media") => match MediaPlugin::try_from(path.as_path()) {
						Ok(plugin) => {
							log::info!("Loaded {plugin}");
							this.media.insert(name, plugin);
						}
						Err(err) => {
							log::debug!("Could not load media plugin {name}: {err}");
						}
					},
					_ => {}
				}
			});

		this
	}

	/// Updates the database with the loaded plugins
	pub(super) async fn update_database(
		&self,
		mut conn: PoolConnection<Sqlite>,
	) -> sqlx::Result<()> {
		let mut plugins = sqlx::query_as::<_, DbPlugin>(r#"SELECT * FROM plugins"#)
			.persistent(false)
			.fetch_all(&mut *conn)
			.await?
			.into_iter()
			.collect::<HashSet<_>>();

		for plugin in self.media.values() {
			let db_plugin = plugin.into();
			let update_schema = if plugins.contains(&db_plugin) {
				let Some(old_plugin) = plugins.get(&db_plugin) else {
					unreachable!()
				};
				db_plugin.version.is_compatible(&old_plugin.version)
			} else {
				true
			};
			plugins.remove(&db_plugin);

			if update_schema {
				let media = match plugin.describe_media() {
					Ok(f) => f(),
					Err(err) => {
						log::warn!("Invalid {plugin}: {err}");
						continue;
					}
				};

				let table_ident = format!("{}_media", media.ident);
				let mut qb = QueryBuilder::new(format!(
					"DROP TABLE IF EXISTS {table_ident}; CREATE TABLE {table_ident}"
				));
				let mut cols = qb.separated(',');
				cols.push_unseparated('(')
					.push("path TEXT NOT NULL PRIMARY KEY");
				media.fields.iter().for_each(|field| {
					cols.push(format!(
						"{} {}",
						field.ident,
						if field.is_list {
							"TEXT NOT NULL DEFAULT (json_array())"
						} else {
							field.r#type.to_sql()
						}
					));
				});
				cols.push_unseparated(')');
				qb.build().persistent(false).execute(&mut *conn).await?;

				sqlx::query!(
					"INSERT INTO plugins(name, kind, version) VALUES (?, ?, ?)",
					db_plugin.name,
					db_plugin.kind,
					db_plugin.version
				)
				.execute(&mut *conn)
				.await?;

				log::debug!("Updated the database schema for {}", media.name);
			}
		}

		Ok(())
	}
}

/// Structure of a [media plugin](pluglib::media)
pub(super) struct MediaPlugin {
	/// Dynamic library
	lib: Library,

	/// Name of the plugin
	pub(super) name: Box<str>,
	/// Version of the plugin
	pub(super) version: Version,
}
impl MediaPlugin {
	/// Returns a description of the media type provided by the plugin
	#[inline]
	pub(super) fn describe_media(&self) -> Result<Symbol<'_, DescribeMedia>, libloading::Error> {
		// SAFETY: Upheld by plugin
		unsafe { self.lib.get(b"describe_media\0") }
	}

	/// Extracts the metadata of the given media
	pub(super) fn extract_metadata(
		&self,
	) -> Result<Symbol<'_, ExtractMetadata>, libloading::Error> {
		// SAFETY: Upheld by plugin
		unsafe { self.lib.get(b"extract_metadata\0") }
	}
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

		Ok(Self { lib, name, version })
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

/// Kind of plugin
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Type)]
#[sqlx(rename_all = "lowercase")]
pub(super) enum PluginKind {
	/// A [media plugin](MediaPlugin)
	Media,
}

/// Structure of the `plugins` database table
///
/// The [`PartialEq`], [`Eq`] and [`Hash`] implementations
/// corresponds to the table's PRIMARY KEY.
#[derive(Debug, Clone, FromRow)]
struct DbPlugin {
	/// Name of the plugin
	name: String,
	/// Kind of the plugin
	kind: PluginKind,
	/// Version of the plugin
	version: Json<Version>,
}
impl From<&MediaPlugin> for DbPlugin {
	#[inline]
	fn from(value: &MediaPlugin) -> Self {
		Self {
			name: value.name.clone().into_string(),
			kind: PluginKind::Media,
			version: Json(value.version),
		}
	}
}
impl PartialEq for DbPlugin {
	#[inline]
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name && self.kind == other.kind
	}
}
impl Eq for DbPlugin {}
impl Hash for DbPlugin {
	#[inline]
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.name.hash(state);
		self.kind.hash(state);
	}
}

/// Errors that may occur when loading a plugin
#[derive(Debug)]
pub(super) enum PluginLoadError {
	/// Error while loading the dynamic library
	LibLoading(libloading::Error),
	/// The library the plugin is linked to is not compatible with the server's library
	IncompatibleLibVersions {
		/// Kind of the plugin
		kind: PluginKind,
		/// Name of the plugin
		name: Box<str>,
		/// Version of the plugin library that the plugin links to
		plugin: Version,
	},
}
impl From<libloading::Error> for PluginLoadError {
	#[inline]
	fn from(err: libloading::Error) -> Self {
		Self::LibLoading(err)
	}
}
impl Display for PluginLoadError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::LibLoading(err) => Display::fmt(err, f),
			Self::IncompatibleLibVersions {
				kind,
				name,
				plugin,
			} => write!(f, "{kind:?} plugin <{name}> links to plugin library {plugin}, which is not compatible with the server's"),
		}
	}
}
impl Error for PluginLoadError {
	#[inline]
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::LibLoading(err) => Some(err),
			Self::IncompatibleLibVersions { .. } => None,
		}
	}
}
