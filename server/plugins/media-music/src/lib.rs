//! Media plugin for *Aedron Patchouli* that provides the music media type
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

use pluglib::{
	ffi::{FfiBoxedSlice, FfiOption, FfiResult, FfiStr},
	media::*,
};
use serde::Deserialize;
use std::process::{Command, Stdio};

make_plugin! {
	music "Music";
	title "Title": Text,
	artists "Artists": Text list,
}

#[allow(clippy::missing_docs_in_private_items)]
/// Root structure of `ffprobe` output
#[derive(Deserialize)]
struct FfprobeData<'data> {
	#[serde(borrow)]
	format: FfprobeDataFormat<'data>,
}

#[allow(clippy::missing_docs_in_private_items)]
/// Structure of [`FfprobeData.format`](FfprobeData#structfield.format)
#[derive(Deserialize)]
struct FfprobeDataFormat<'data> {
	#[serde(borrow)]
	tags: FfprobeDataFormatTags<'data>,
}

#[allow(clippy::missing_docs_in_private_items)]
/// Structure of [`FfprobeDataFormat.tags`](FfprobeDataFormat#structfield.tags)
#[derive(Deserialize)]
struct FfprobeDataFormatTags<'data> {
	title: Option<&'data str>,
	artist: Option<&'data str>,
	#[serde(alias = "ARTISTS")]
	artists: Option<&'data str>,
}

#[allow(unsafe_code)]
/// Extracts the metadata of the given media file
#[no_mangle]
pub extern "C" fn extract_metadata(
	path: FfiStr<'_>,
) -> FfiResult<FfiBoxedSlice<FfiOption<MetadataFieldValue>>, ()> {
	(|| {
		let output = Command::new("ffprobe")
			.args(["-v", "quiet", "-show_format", "-print_format", "json"])
			.arg(&*path)
			.stdin(Stdio::null())
			.stderr(Stdio::null())
			.output()
			.map_err(|_err| ())?;
		if !output.status.success() {
			return Err(());
		}
		let data = serde_json::from_slice::<FfprobeData>(&output.stdout).map_err(|_err| ())?;

		Ok([
			data.format.tags.title.and_then(|s| s.try_into().ok()),
			None,
			None,
		]
		.into_iter()
		.map(From::from)
		.collect())
	})()
	.into()
}
