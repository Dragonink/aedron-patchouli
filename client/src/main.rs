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

mod components;

use components::App;
#[cfg(target_arch = "wasm32")]
use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[doc(hidden)]
#[global_allocator]
static ALLOC: AssumeSingleThreaded<FreeListAllocator> =
	// SAFETY: This application is single-threaded.
	unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };

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

#[allow(missing_docs, clippy::missing_panics_doc)]
#[wasm_bindgen(start)]
pub fn main() {
	setup_logger().unwrap();

	leptos::mount_to_body(|| leptos::view! { <App /> });
}
