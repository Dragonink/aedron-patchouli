#![allow(unsafe_code)]
//! Provides the server's plugin features

mod media;

use crate::{config::MediaConfig, EXE_NAME};
use media::MediaPlugin;
use pluglib::Version;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::prelude::*;
use rusqlite::{
	types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
	Row, ToSql,
};
use std::{
	collections::{HashMap, HashSet},
	error::Error,
	fmt::{self, Debug, Display, Formatter},
	hash::{Hash, Hasher},
	path::{Path, PathBuf},
	str::FromStr,
};

/// Stores all plugins
#[derive(Debug, Default)]
pub(crate) struct PluginStore {
	/// Stores media plugins
	pub(crate) media: HashMap<String, MediaPlugin>,
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
	pub(super) fn update_database(
		&self,
		db_pool: &Pool<SqliteConnectionManager>,
	) -> Result<(), Box<dyn Error>> {
		let plugins = {
			let conn = db_pool.get()?;
			let mut stmt = conn.prepare("SELECT * FROM plugins")?;
			let plugins = stmt
				.query_map((), |row| DbPlugin::try_from(row))?
				.filter_map(|res| match res {
					Ok(db_plugin) => Some(db_plugin),
					Err(err) => {
						log::error!("{err}");
						None
					}
				})
				.collect::<HashSet<_>>();
			stmt.finalize()?;
			plugins
		};
		self.media.values().for_each(|plugin| {
			let db_plugin = plugin.into();
			let update_schema = if plugins.contains(&db_plugin) {
				let Some(old_plugin) = plugins.get(&db_plugin) else {
					unreachable!()
				};
				!db_plugin.version.is_compatible(&old_plugin.version)
			} else {
				true
			};
			if update_schema {
				if let Err(err) = plugin.update_database(db_pool, db_plugin) {
					log::error!("Could not insert {plugin} into the database: {err}");
				}
			}
		});

		Ok(())
	}

	/// Loads all media files
	#[inline]
	pub(super) fn load_media(
		&self,
		db_pool: &Pool<SqliteConnectionManager>,
		config: &HashMap<String, MediaConfig>,
	) {
		self.media
			.par_iter()
			.filter_map(|(name, plugin)| config.get(name).map(|config| (plugin, config)))
			.for_each(|(plugin, config)| {
				let conn = loop {
					if let Some(conn) = db_pool.try_get() {
						break conn;
					}
					std::thread::yield_now();
				};
				if let Err(err) = plugin.load_media(conn, config) {
					log::error!("Could not commit media of {plugin}: {err}");
				}
			});
	}
}

/// Kind of plugin
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum PluginKind {
	/// A [media plugin](MediaPlugin)
	Media,
}
impl FromStr for PluginKind {
	type Err = InvalidPluginKind;

	#[inline]
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"media" => Ok(Self::Media),
			_ => Err(InvalidPluginKind),
		}
	}
}
impl Display for PluginKind {
	#[inline]
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str(&format!("{self:?}").to_ascii_lowercase())
	}
}
impl FromSql for PluginKind {
	#[inline]
	fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
		value
			.as_str()
			.and_then(|s| s.parse().map_err(|err| FromSqlError::Other(Box::new(err))))
	}
}
impl ToSql for PluginKind {
	#[inline]
	fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
		Ok(ToSqlOutput::Borrowed(ValueRef::Text(
			match self {
				Self::Media => "media",
			}
			.as_bytes(),
		)))
	}
}

/// Structure of the `plugins` database table
///
/// The [`PartialEq`], [`Eq`] and [`Hash`] implementations
/// corresponds to the table's PRIMARY KEY.
#[derive(Debug, Clone)]
struct DbPlugin {
	/// Name of the plugin
	name: String,
	/// Kind of the plugin
	kind: PluginKind,
	/// Version of the plugin
	version: Version,
}
impl From<&MediaPlugin> for DbPlugin {
	#[inline]
	fn from(value: &MediaPlugin) -> Self {
		Self {
			name: value.name.clone().into_string(),
			kind: PluginKind::Media,
			version: value.version,
		}
	}
}
impl<'stmt> TryFrom<&'stmt Row<'stmt>> for DbPlugin {
	type Error = rusqlite::Error;

	#[inline]
	fn try_from(row: &'stmt Row) -> Result<Self, Self::Error> {
		Ok(Self {
			name: row.get("name")?,
			kind: row.get("kind")?,
			version: row.get("version")?,
		})
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
impl Display for DbPlugin {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{} plugin <{} {}>", self.kind, self.name, self.version)
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

/// Error that may occur in [`PluginKind::from_str`]
#[derive(Debug, Default)]
pub(crate) struct InvalidPluginKind;
impl Display for InvalidPluginKind {
	#[inline]
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.write_str("invalud PluginKind value")
	}
}
impl Error for InvalidPluginKind {
	#[inline]
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		None
	}
}

/// Trait for plugin structures
trait Plugin: Debug + Display + for<'p> TryFrom<&'p Path, Error = PluginLoadError>
where
	for<'this> &'this Self: Into<DbPlugin>,
{
	/// Updates the database with the plugin
	fn update_database(
		&self,
		db_pool: &Pool<SqliteConnectionManager>,
		db_plugin: DbPlugin,
	) -> Result<(), Box<dyn Error>>;
}
