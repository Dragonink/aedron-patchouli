//! Client application of *Aedron Patchouli*
#![warn(
	// Restriction (lib)
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
	clippy::needless_raw_string_hashes,
	clippy::semicolon_if_nothing_returned,
	clippy::unnested_or_patterns,
	clippy::unused_self,
	// Style
	unused_import_braces,
	// Nursery
	clippy::empty_line_after_outer_attr,
	clippy::imprecise_flops,
	clippy::missing_const_for_fn,
	clippy::needless_pass_by_ref_mut,
	clippy::readonly_write_lock,
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

#[cfg(not(any(feature = "hydrate", feature = "ssr")))]
compile_error!("One of the `hydrate` and `ssr` features must be enabled");
#[cfg(all(feature = "hydrate", feature = "ssr"))]
compile_error!("The `hydrate` and `ssr` features are mutually exclusive");

pub mod components;

pub use crate::components::*;
pub use leptos;
#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};
pub use reqwest;
use reqwest::{
	header::{HeaderMap, HeaderValue, ACCEPT},
	ClientBuilder, RequestBuilder, Url,
};
#[cfg(feature = "hydrate")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[doc(hidden)]
#[global_allocator]
static ALLOC: AssumeSingleThreaded<FreeListAllocator> =
	// SAFETY: This application is single-threaded.
	unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

#[cfg(feature = "hydrate")]
/// Sets up the application's logger
///
/// The [panic hook](std::panic::set_hook) is set to output panic info through the logger.
fn setup_logger() -> Result<(), log::SetLoggerError> {
	console_log::init_with_level(
		log::STATIC_MAX_LEVEL
			.to_level()
			.unwrap_or_else(|| unreachable!()),
	)?;

	// Make panics use the installed logger
	std::panic::set_hook(Box::new(move |panic_info| {
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
			log::error!(
				"Application panicked at {}:{}:{} because {message}",
				location.file(),
				location.line(),
				location.column()
			);
		} else {
			log::error!("Application panicked because {message}");
		}

		if let Some(window) = web_sys::window() {
			_ = window.alert_with_message(
				"The application crashed!\nPlease report the error message printed in the console.",
			);
		}
	}));

	Ok(())
}

#[cfg(feature = "hydrate")]
#[doc(hidden)]
#[wasm_bindgen]
pub fn hydrate() {
	use leptos::*;

	setup_logger().unwrap();

	mount_to_body(move || {
		let mut builder = ClientBuilder::new();
		let Some(window) = web_sys::window() else {
			unreachable!()
		};
		let Ok(origin) = window.location().origin() else {
			unreachable!()
		};
		let Ok(base_url) = Url::parse(&origin) else {
			unreachable!()
		};
		provide_context(RequestClient::build(builder, base_url).unwrap());

		view! { <App /> }
	});
}

/// Wrapper around [`reqwest::Client`] that adds a base URL
#[derive(Debug, Clone)]
pub struct RequestClient {
	/// Wrapped client
	pub client: reqwest::Client,
	/// Base URL
	base_url: Url,
}
impl RequestClient {
	/// Constructs a new instance from a [`ClientBuilder`] and a base URL
	///
	/// # Errors
	/// See [`ClientBuilder::build`].
	///
	/// # Panics
	/// This function panics if the URL [cannot be a base](Url::cannot_be_a_base).
	pub fn build(mut builder: ClientBuilder, base_url: Url) -> reqwest::Result<Self> {
		debug_assert!(!base_url.cannot_be_a_base());

		let mut headers = HeaderMap::new();
		headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
		builder = builder.default_headers(headers);

		builder.build().map(|client| Self { client, base_url })
	}

	/// See [`reqwest::Client::get`]
	///
	/// # Panics
	/// This function panics if [`Url::join`] returns an error.
	#[inline]
	pub fn get(&self, url: &str) -> RequestBuilder {
		self.client.get(
			self.base_url
				.join(url)
				.unwrap_or_else(|_err| unreachable!()),
		)
	}
}
