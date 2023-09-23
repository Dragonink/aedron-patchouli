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
	unsafe_op_in_unsafe_fn,
	unused_must_use,
	clippy::exit,
	clippy::lossy_float_literal,
	clippy::undocumented_unsafe_blocks,
)]
#![forbid(unsafe_code)]

use kobold::prelude::*;
use log::Level;
use wasm_bindgen::prelude::*;

#[allow(missing_docs, clippy::missing_errors_doc, clippy::missing_panics_doc)]
#[wasm_bindgen(start)]
pub fn main() {
	console_log::init_with_level({
		#[cfg(debug_assertions)]
		{
			Level::Trace
		}
		#[cfg(not(debug_assertions))]
		{
			Level::Info
		}
	})
	.unwrap();

	kobold::start(view! {
		<h1>"Aedron Patchouli"</h1>
	});
}
