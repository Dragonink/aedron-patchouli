//! Provides the server's configuration

use config::{ConfigError, Environment, File};
use serde::Deserialize;
use std::{
	collections::HashMap,
	net::{IpAddr, Ipv4Addr},
	path::PathBuf,
};

/// Builds the server's configuration
#[inline]
pub(super) fn build_config() -> Result<Config, ConfigError> {
	config::Config::builder()
		.add_source(File::with_name("config").required(false))
		.add_source(Environment::with_prefix("AEPA"))
		.build()
		.and_then(|config| config.try_deserialize())
}

/// Root configuration structure
#[derive(Debug, Clone, Deserialize)]
pub(super) struct Config {
	/// Address to bind the server to
	#[serde(default = "Config::default_addr")]
	pub(super) addr: IpAddr,
	/// Port to bind the server to
	#[serde(default = "Config::default_port")]
	pub(super) port: u16,
	/// Configuration of media plugins
	#[serde(default)]
	pub(super) media: HashMap<String, MediaConfig>,
}
impl Config {
	/// Default value for [`addr`](Self#structfield.addr)
	#[inline]
	const fn default_addr() -> IpAddr {
		IpAddr::V4(Ipv4Addr::UNSPECIFIED)
	}

	/// Default value for [`port`](Self#structfield.port)
	#[inline]
	const fn default_port() -> u16 {
		2372
	}
}
impl Default for Config {
	#[inline]
	fn default() -> Self {
		Self {
			addr: Self::default_addr(),
			port: Self::default_port(),
			media: Default::default(),
		}
	}
}

/// Configuration of a single media plugin
#[derive(Debug, Default, Clone, Deserialize)]
pub(super) struct MediaConfig {
	/// Root directories containing the media files
	#[serde(default)]
	pub(super) paths: Vec<PathBuf>,
}
