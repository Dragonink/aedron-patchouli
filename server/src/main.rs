//! Server application of *Aedron Patchouli*
#![warn(
	// Restriction (use `log` instead)
	clippy::print_stdout,
	clippy::print_stderr,
	// Restriction
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	unreachable_pub,
	unused,
	unused_crate_dependencies,
	unused_lifetimes,
	unused_tuple_struct_fields,
	clippy::dbg_macro,
	clippy::empty_structs_with_brackets,
	clippy::enum_glob_use,
	clippy::float_cmp_const,
	clippy::format_push_string,
	clippy::match_on_vec_items,
	clippy::mem_forget,
	clippy::missing_docs_in_private_items,
	clippy::mod_module_files,
	clippy::option_option,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::str_to_string,
	clippy::verbose_file_reads,
	// Suspicious
	noop_method_call,
	meta_variable_misuse,
	// Pedantic
	unused_qualifications,
	clippy::doc_link_with_quotes,
	clippy::doc_markdown,
	clippy::filter_map_next,
	clippy::float_cmp,
	clippy::inefficient_to_string,
	clippy::macro_use_imports,
	clippy::manual_let_else,
	clippy::map_unwrap_or,
	clippy::match_wildcard_for_single_variants,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	clippy::needless_continue,
	clippy::semicolon_if_nothing_returned,
	clippy::unnested_or_patterns,
	clippy::unused_self,
	// Style
	unused_import_braces,
	// Nursery
	clippy::empty_line_after_outer_attr,
	clippy::imprecise_flops,
	clippy::missing_const_for_fn,
	clippy::suboptimal_flops,
)]
#![deny(
	// Correctness
	pointer_structural_match,
	// Restriction
	keyword_idents,
	non_ascii_idents,
	missing_abi,
	unsafe_code,
	unsafe_op_in_unsafe_fn,
	unused_must_use,
	clippy::exit,
	clippy::lossy_float_literal,
)]
#![forbid(clippy::undocumented_unsafe_blocks)]

mod config;
mod db;
mod http;
mod plugins;
mod tls;

use crate::tls::TlsAddrIncoming;
use axum::{
	extract::{connect_info::Connected, FromRef},
	Server,
};
use colored::Colorize;
use config::Config;
use hyper::server::{accept::Accept, conn::AddrIncoming};
use plugins::PluginStore;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::{error::Error, fmt::Display, io, net::SocketAddr, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite};

/// Name of the server executable
const EXE_NAME: &str = env!("CARGO_BIN_NAME");

/// [`log`] target used to color the message according to the level
const LOG_HIGHLIGHT: &str = "_HIGHLIGHT";

/// Sets up the application's logger
///
/// The logger should output logs like:
/// ```log
/// 2023-08-19T14:01:10Z INFO [aedron-patchouli] Hello, world!
/// ```
///
/// On `unix` targets, the logs will be output to the syslog as well.
///
/// Also, the [panic hook](std::panic::set_hook) is set to output panic info through the logger.
fn setup_logger() -> Result<(), fern::InitError> {
	use colored::Color;
	use fern::{colors::ColoredLevelConfig, Dispatch, InitError};
	use log::LevelFilter;
	use time::{
		format_description::well_known::{
			iso8601::{Config, EncodedConfig, TimePrecision},
			Iso8601,
		},
		OffsetDateTime,
	};

	/// [`log`] target used by panics
	const LOG_PANIC: &str = "_PANIC";
	/// Format of the timestamps
	const TIME_FORMAT: EncodedConfig = Config::DEFAULT
		.set_time_precision(TimePrecision::Second {
			decimal_digits: None,
		})
		.encode();

	let colors = ColoredLevelConfig::default()
		.trace(Color::Magenta)
		.debug(Color::Green)
		.info(Color::Cyan);

	let mut logger = Dispatch::new()
		.level(log::STATIC_MAX_LEVEL)
		.level_for("tracing::span", LevelFilter::Off)
		.level_for("hyper", LevelFilter::Info)
		.level_for("tower_http::trace", LevelFilter::Off)
		.chain(
			Dispatch::new()
				.format(move |out, message, record| {
					let Ok(timestamp) = OffsetDateTime::now_utc().format(&Iso8601::<TIME_FORMAT>)
					else {
						unreachable!()
					};
					let target = record.target();
					let module = record.module_path().unwrap_or_default();

					let level_color = colors.get_color(&record.level());
					let highlight = target == LOG_HIGHLIGHT || target == LOG_PANIC;

					let level: Box<dyn Display> = if target == LOG_PANIC {
						Box::new("FATAL".bold().color(colors.error))
					} else {
						Box::new(colors.color(record.level()))
					};
					let target = if !highlight && target != module {
						format!("<{target}>")
					} else {
						format!("[{module}]")
					}
					.dimmed();
					out.finish(format_args!(
						"{timestamp} {level:5} {target} \x1B[1;{color}m{message}\x1B[0m",
						color = if highlight {
							level_color.to_fg_str()
						} else {
							"0"
						},
					));
				})
				.chain(std::io::stdout()),
		);
	#[cfg(unix)]
	{
		// If `unix`, output to syslog as well
		let syslog_formatter = syslog::Formatter3164 {
			facility: syslog::Facility::LOG_USER,
			hostname: None,
			process: EXE_NAME.to_owned(),
			pid: 0,
		};
		logger = logger.chain(
			Dispatch::new().chain(syslog::unix(syslog_formatter).map_err(|err| match err.0 {
				syslog::ErrorKind::Io(err) => InitError::Io(err),
				_ => unreachable!(),
			})?),
		);
	}
	logger.apply()?;

	// Make panics use the installed logger
	std::panic::set_hook(Box::new(move |panic_info| {
		let thread = std::thread::current();
		let thread = thread.name().unwrap_or("<unnamed>");

		let message = panic_info
			.payload()
			.downcast_ref::<&str>()
			.copied()
			.or_else(|| {
				panic_info
					.payload()
					.downcast_ref::<String>()
					.map(|s| s.as_str())
			})
			.unwrap_or(r"¯\_(ツ)_/¯");

		if let Some(location) = panic_info.location() {
			log::error!(target: LOG_PANIC, "Thread '{thread}' panicked at {}:{}:{} because {message}", location.file(), location.line(), location.column());
		} else {
			log::error!(target: LOG_PANIC, "Thread '{thread}' panicked because {message}");
		}
	}));

	Ok(())
}

/// Stores the server's state
#[derive(Debug, Clone, FromRef)]
struct AppState {
	/// Configuration of the server
	config: Arc<Config>,
	/// Pool of connections to the database
	db_pool: Pool<SqliteConnectionManager>,
	/// Stores all plugins
	plugins: Arc<PluginStore>,
}

/// Constructs and runs a [`Server`]
async fn serve<I: Accept>(incoming: I, state: AppState) -> hyper::Result<()>
where
	I::Conn: AsyncRead + AsyncWrite + Unpin + Send + 'static,
	I::Error: Error + Send + Sync + 'static,
	for<'conn> SocketAddr: Connected<&'conn I::Conn>,
{
	Server::builder(incoming)
		.serve(
			http::new_router()
				.with_state(state)
				.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.with_graceful_shutdown(graceful_shutdown())
		.await
}

#[tokio::main]
async fn main() {
	/// Inner [`main`] function used to [`Display`] the returned error
	#[inline]
	async fn _main() -> Result<(), Box<dyn Error>> {
		setup_logger()?;

		let config = config::build_config()?;
		log::trace!("{config:?}");
		let addr = SocketAddr::new(config.addr, config.port);
		let identity = match tls::read_identity(&config.tls.certificate, &config.tls.key) {
			Ok(identity) => Some(identity),
			Err(ref err) if err.kind() == io::ErrorKind::NotFound => None,
			Err(err) => {
				return Err(err.into());
			}
		};

		let db_pool = db::init()?;

		let plugins = PluginStore::load_plugins();
		plugins.update_database(&db_pool)?;
		plugins.load_media(&db_pool, &config.media);

		let state = AppState {
			config: Arc::new(config),
			db_pool,
			plugins: Arc::new(plugins),
		};

		log::info!(target: LOG_HIGHLIGHT, "Starting the server on {addr}");
		if let Some(identity) = identity {
			serve(TlsAddrIncoming::bind(&addr, identity)?, state).await
		} else {
			serve(AddrIncoming::bind(&addr)?, state).await
		}?;

		Ok(())
	}
	if let Err(err) = _main().await {
		panic!("{err}");
	}
}

/// Returns a [`Future`](std::future::Future) that resolves when the ⌃C signal is caught
///
/// Additionally, on `unix` targets, the SIGTERM signal is also awaited.
async fn graceful_shutdown() {
	use tokio::signal;
	#[cfg(unix)]
	use tokio::signal::unix::SignalKind;

	let ctrl_c = async {
		signal::ctrl_c()
			.await
			.expect("the ⌃C signal listener could not be installed");
	};

	#[cfg(unix)]
	let sig_term = async {
		signal::unix::signal(SignalKind::terminate())
			.expect("the SIGTERM signal listener could not be installed")
			.recv()
			.await;
	};
	#[cfg(not(unix))]
	let sig_term = std::future::pending();

	tokio::select! {
		_ = ctrl_c => {}
		_ = sig_term => {}
	}
}
