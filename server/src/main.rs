#![forbid(unsafe_code)]
#![deny(unused_must_use)]

#[macro_use]
extern crate rocket;
use aedron_patchouli_server_proc::*;
use const_format::formatcp;
use figment::{
	value::{Dict, Map},
	Metadata, Profile, Provider, Source,
};
use futures::stream::StreamExt;
use rocket::{Build, Orbit, Rocket};
use rocket_db_pools::{Database as IDatabase, Pool};
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use sqlx::SqlitePool;
use std::net::IpAddr;

#[macro_use]
mod log;
mod guards;
mod routes;
mod tasks;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
struct Config {
	#[serde(default = "Config::default_address")]
	address: IpAddr,
	#[serde(default = "Config::default_port")]
	port: u16,
	#[serde(default = "Config::default_secret_key", skip_serializing)]
	secret_key: String,

	#[serde(skip)]
	profile: Profile,
	#[serde(skip)]
	source: Option<Source>,
}
impl Config {
	#[inline]
	pub const fn default_address() -> IpAddr {
		use std::net::Ipv4Addr;

		IpAddr::V4(Ipv4Addr::UNSPECIFIED)
	}

	#[inline]
	pub const fn default_port() -> u16 {
		2372 // "AEPA" on a phone keyboard
	}

	#[inline]
	pub fn default_secret_key() -> String {
		use rocket::config::SecretKey;

		let s = String::from_utf8_lossy(&[b'0'; 64]);
		#[cfg(debug_assertions)]
		{
			let res: Result<SecretKey, serde::de::value::Error> =
				Deserialize::deserialize(s.as_ref().into_deserializer());
			assert!(res.unwrap().is_zero());
		}
		s.to_string()
	}
}
impl Default for Config {
	#[inline]
	fn default() -> Self {
		Self {
			address: Self::default_address(),
			port: Self::default_port(),
			secret_key: Self::default_secret_key(),

			profile: Default::default(),
			source: Default::default(),
		}
	}
}
impl Provider for Config {
	fn metadata(&self) -> Metadata {
		let mut meta = Metadata::named("Aedron Patchouli Config");
		meta.source = self.source.clone();
		meta
	}

	fn data(&self) -> figment::error::Result<Map<Profile, Dict>> {
		use figment::providers::Serialized;

		let mut map = Serialized::from(self, self.profile.clone()).data()?;
		if self.profile != Profile::Default && self.profile != "debug" {
			if let Some(map) = map.get_mut(&self.profile) {
				map.insert(
					rocket::Config::SECRET_KEY.to_string(),
					self.secret_key.as_str().into(),
				);
			}
		}
		Ok(map)
	}

	#[inline]
	fn profile(&self) -> Option<Profile> {
		Some(self.profile.clone())
	}
}

#[derive(Clone, IDatabase)]
#[database("db")]
#[repr(transparent)]
struct Database(SqlitePool);
impl Database {
	pub const APPLICATION_ID_PRAGMA: &'static str = "application_id";
	pub const DB_VERSION_PRAGMA: &'static str = "user_version";

	pub const APPLICATION_ID: u32 = u32::from_be_bytes(*b"aepa");

	const MIGRATIONS: &'static [&'static str] = include_migrations!("sql");

	async fn on_ignite(rocket: Rocket<Build>) -> rocket::fairing::Result {
		use aedron_patchouli_common::libraries::DbLibraryConfig;
		use either::Either;
		use futures::stream::FuturesUnordered;

		macro_rules! try_result {
			($res:expr) => {
				match $res {
					Ok(val) => val,
					Err(err) => {
						console_error!("Database error", "{err}");
						return Err(rocket);
					}
				}
			};
		}

		let mut db = try_result!(rocket.state::<Database>().unwrap().get().await);
		macro_rules! get_version {
			() => {
				sqlx::query_scalar(formatcp!("PRAGMA {}", Database::DB_VERSION_PRAGMA))
					.persistent(false)
					.fetch_one(&mut db)
					.await
					.map(|val: i32| val as usize)
			};
		}

		// Check if APPLICATION_ID pragma is ours
		let app_id = try_result!(sqlx::query_scalar(formatcp!(
			"PRAGMA {}",
			Database::APPLICATION_ID_PRAGMA
		))
		.persistent(false)
		.fetch_one(&mut db)
		.await
		.map(|val: i32| val as u32));
		if app_id == 0 {
			// If APPLICATION_ID pragma is null, check if schema has been modified
			let schema: i32 = try_result!(
				sqlx::query_scalar!("SELECT count(*) FROM sqlite_master")
					.persistent(false)
					.fetch_one(&mut db)
					.await
			);
			if schema != 0 {
				try_result!(Err("Database already initialized by another application"));
			}
		} else if app_id != Database::APPLICATION_ID {
			try_result!(Err("Database already initialized by another application"));
		}

		console_info!("Initializing", "database");
		try_result!(
			sqlx::query(formatcp!(
				"PRAGMA {key} = {val}",
				key = Database::APPLICATION_ID_PRAGMA,
				val = Database::APPLICATION_ID
			))
			.persistent(false)
			.execute(&mut db)
			.await
		);
		// Execute migrations
		let version = try_result!(get_version!());
		for migration in &Database::MIGRATIONS[version..] {
			try_result!(
				sqlx::query(migration)
					.persistent(false)
					.execute(&mut db)
					.await
			);
		}
		let new_version = try_result!(get_version!());
		if new_version > version {
			console_log!("Migrated", "database to v{new_version}");
		}
		// Create default admin user
		let user_count = try_result!(
			sqlx::query_scalar!("SELECT count(*) FROM users")
				.fetch_one(&mut db)
				.await
		);
		if user_count == 0 {
			const ADMIN_USERNAME: &str = "admin";
			const ADMIN_PASSWORD: &str = "admin";

			match tasks::hash_passwd(ADMIN_PASSWORD) {
				Ok(enc) => {
					try_result!(
						sqlx::query!(
							"INSERT INTO users (name, passwd) VALUES (?, ?)",
							ADMIN_USERNAME,
							enc
						)
						.execute(&mut db)
						.await
					);
				}
				Err(err) => {
					match err {
						Either::Left(err) => {
							console_error!("RNG error", "{err}");
						}
						Either::Right(err) => {
							console_error!("Crypto error", "{err}");
						}
					}
					return Err(rocket);
				}
			}
		}
		// Index library contents
		let libraries = try_result!(
			sqlx::query_as!(DbLibraryConfig, "SELECT * FROM libraries")
				.fetch_all(&mut db)
				.await
		)
		.into_iter()
		.filter_map(|db_config| {
			let name = db_config.name.clone();
			match db_config.try_into() {
				Ok(val) => Some(val),
				Err(err) => {
					console_warn!(&format!("Error in library {name}"), "{err}");
					None
				}
			}
		});
		let db = rocket.state::<Database>().unwrap();
		libraries
			.map(|config| async move { tasks::index_library(db, &config).await })
			.collect::<FuturesUnordered<_>>()
			.collect::<()>()
			.await;
		console_log!("Initialized", "database");

		Ok(rocket)
	}

	async fn on_shutdown(rocket: &Rocket<Orbit>) {
		macro_rules! try_result {
			($res:expr) => {
				match $res {
					Ok(val) => val,
					Err(err) => {
						console_warn!("Database error", "{err}");
						return;
					}
				}
			};
		}

		let mut db = try_result!(rocket.state::<Database>().unwrap().get().await);
		console_info!("Vacuuming", "database");
		try_result!(sqlx::query("VACUUM").execute(&mut db).await);
		console_log!("Vacuumed", "database");
	}
}

#[launch]
async fn rocket() -> _ {
	use figment::{
		providers::{Format, Toml},
		Figment,
	};
	use rocket::fairing::AdHoc;
	use std::{path::Path, process};
	use yansi::Paint;

	Paint::enable_windows_ascii();

	const STATIC_CONFIG: &str = include_static_config!("src/Rocket.toml");
	let rocket_config = rocket::Config::default();
	let aepa_config = Config {
		profile: rocket_config.profile.clone(),
		..Config::default()
	};
	let mut figment = Figment::from(rocket_config)
		.merge(aepa_config)
		.merge(Toml::string(STATIC_CONFIG).nested());
	const USER_CONFIG_PATH: &str = "config.toml";
	if let Ok(md) = tokio::fs::metadata(USER_CONFIG_PATH).await {
		if md.is_file() {
			match Toml::from_path::<Config>(Path::new(USER_CONFIG_PATH)) {
				Ok(mut user_config) => {
					user_config.profile = Profile::Global;
					user_config.source = Some(Source::File(USER_CONFIG_PATH.into()));
					figment = figment.merge(user_config);
				}
				Err(err) => {
					console_error!(formatcp!("Could not read `{USER_CONFIG_PATH}`"), "{err}");
					process::exit(1);
				}
			}
		}
	}

	routes::mount(
		rocket::custom(figment)
			.attach(AdHoc::on_shutdown("Database Vacuumer", |rocket| {
				Box::pin(Database::on_shutdown(rocket))
			}))
			.attach(Database::init())
			.attach(AdHoc::try_on_ignite(
				"Database Igniter",
				Database::on_ignite,
			))
			.attach(AdHoc::on_liftoff("Liftoff Announcer", |rocket| {
				Box::pin(async move {
					let config = rocket.config();
					console_log!(
						"LAUNCHED SERVER",
						"from {}",
						Paint::new(format!("http://{}:{}/", config.address, config.port))
							.underline()
					);
				})
			})),
	)
}
