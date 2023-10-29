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
pub(crate) fn build_config() -> Result<Config, ConfigError> {
	config::Config::builder()
		.add_source(File::with_name("config").required(false))
		.add_source(Environment::with_prefix("AEPA"))
		.build()
		.and_then(|config| config.try_deserialize())
}

/// Root configuration structure
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Config {
	/// Address to bind the server to
	#[serde(default = "Config::default_addr")]
	pub(crate) addr: IpAddr,
	/// Port to bind the server to
	#[serde(default = "Config::default_port")]
	pub(crate) port: u16,
	/// Configuration of the TLS
	#[serde(default)]
	pub(crate) tls: TlsConfig,
	/// Configuration of media plugins
	#[serde(default)]
	pub(crate) media: HashMap<String, MediaConfig>,
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
			tls: Default::default(),
			media: Default::default(),
		}
	}
}

/// Configuration of the TLS
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TlsConfig {
	/// TLS certificate file
	#[serde(default = "TlsConfig::default_certificate")]
	pub(crate) certificate: PathBuf,
	/// TLS private key file
	#[serde(default = "TlsConfig::default_key")]
	pub(crate) key: PathBuf,
	/// Additional [subject alternative names](https://en.wikipedia.org/wiki/Subject_Alternative_Name)
	#[serde(default)]
	pub(crate) san: Vec<String>,
}
impl TlsConfig {
	/// Default value for [`certificate`](Self#structfield.certificate)
	#[inline]
	fn default_certificate() -> PathBuf {
		PathBuf::from("certificate.crt")
	}

	/// Default value for [`key`](Self#structfield.key)
	#[inline]
	fn default_key() -> PathBuf {
		PathBuf::from("private.key")
	}
}
impl Default for TlsConfig {
	#[inline]
	fn default() -> Self {
		Self {
			certificate: Self::default_certificate(),
			key: Self::default_key(),
			san: Default::default(),
		}
	}
}

/// Configuration of a single media plugin
#[derive(Debug, Default, Clone, Deserialize)]
pub(crate) struct MediaConfig {
	/// Root directories containing the media files
	#[serde(default)]
	pub(crate) paths: Vec<PathBuf>,
}
