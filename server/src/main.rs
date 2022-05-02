#![forbid(unsafe_code)]
#![deny(unused_must_use)]

#[macro_use]
extern crate rocket;
use aedron_patchouli_server_proc::*;
use futures::stream::StreamExt;
use rocket::{
	fairing::{self, Fairing, Info},
	Build, Orbit, Rocket,
};
use rocket_db_pools::{sqlx::SqlitePool, Database as IDatabase, Pool};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[macro_use]
mod log;
mod routes;
mod tasks;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
	#[serde(default = "Config::default_address")]
	address: IpAddr,
	#[serde(default = "Config::default_port")]
	port: u16,
}
impl Config {
	#[inline(always)]
	pub const fn default_address() -> IpAddr {
		use std::net::Ipv4Addr;

		IpAddr::V4(Ipv4Addr::UNSPECIFIED)
	}

	#[inline(always)]
	pub const fn default_port() -> u16 {
		2372 //NOTE: "AEPA" on a phone keyboard
	}
}
impl Default for Config {
	#[inline(always)]
	fn default() -> Self {
		Self {
			address: Self::default_address(),
			port: Self::default_port(),
		}
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
}

struct DatabaseManager;
#[rocket::async_trait]
impl Fairing for DatabaseManager {
	#[inline(always)]
	fn info(&self) -> Info {
		use rocket::fairing::Kind;

		Info {
			name: "Database Manager",
			kind: Kind::Ignite | Kind::Shutdown | Kind::Singleton,
		}
	}

	async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
		use aedron_patchouli_common::library::DbLibraryConfig;
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
				sqlx::query_scalar(&format!("PRAGMA {}", Database::DB_VERSION_PRAGMA))
					.persistent(false)
					.fetch_one(&mut db)
					.await
					.map(|val: i32| val as usize)
			};
		}

		// Check if APPLICATION_ID pragma is ours
		let app_id = try_result!(sqlx::query_scalar(&format!(
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
			sqlx::query(&format!(
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

	async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
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
		providers::{Format, Serialized, Toml},
		Figment,
	};
	use std::{path::Path, process};

	yansi::Paint::enable_windows_ascii();

	const STATIC_CONFIG: &str = include_static_config!("src/Rocket.toml");
	let mut figment = Figment::from(rocket::Config::default())
		.merge(Serialized::defaults(Config::default()))
		.merge(Toml::string(STATIC_CONFIG).nested());
	const USER_CONFIG_PATH: &str = "config.toml";
	if let Ok(md) = tokio::fs::metadata(USER_CONFIG_PATH).await {
		if md.is_file() {
			match Toml::from_path::<Config>(Path::new(USER_CONFIG_PATH)) {
				Ok(user_config) => {
					figment = figment.merge(Serialized::defaults(user_config));
				}
				Err(err) => {
					console_error!(&format!("Could not read `{USER_CONFIG_PATH}`"), "{err}");
					process::exit(1);
				}
			}
		}
	}

	routes::mount(
		rocket::custom(figment)
			.attach(Database::init())
			.attach(DatabaseManager),
	)
}
