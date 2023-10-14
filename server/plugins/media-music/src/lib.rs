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
	unsafe_op_in_unsafe_fn,
	unused_must_use,
	clippy::exit,
	clippy::lossy_float_literal,
)]
#![forbid(clippy::undocumented_unsafe_blocks)]

use pluglib::{
	ffi::{new_ffistr, FfiBoxedSlice, FfiOption, FfiResult, FfiStr},
	media::*,
};
use serde::Deserialize;
use std::{
	io,
	process::{Command, Stdio},
};

make_plugin! {
	music "Music";
	title "Title": Text,
	artists "Artists": Text list,
}

/// Lists the types supported by the plugin
#[no_mangle]
pub extern "C" fn supported_types() -> FfiBoxedSlice<FfiStr<'static>> {
	Command::new("ffprobe")
		.args(["-v", "quiet", "-formats"])
		.stdin(Stdio::null())
		.stderr(Stdio::null())
		.output()
		.map(|out| out.stdout)
		.and_then(|out| {
			String::from_utf8(out).map_err(|err| io::Error::new(io::ErrorKind::Other, err))
		})
		.map(|data| {
			data.lines()
				.skip(4)
				.filter_map(|s| {
					let format = s.trim().split_ascii_whitespace().nth(1)?;
					/// Generates match branches for the given formats
					macro_rules! match_format {
						($(
							$format:expr => [$( $mime:literal ),+ $(,)?]
						),* $(,)?) => {
							match format {
								$(
									$format => Some(vec![$( new_ffistr!($mime) ),+]),
								)*
								_ => None,
							}
						};
					}
					match_format! {
						"acc" => ["audio/aac"],
						"adts" => ["audio/aac", "audio/aacp"],
						"caf" => ["audio/x-caf"],
						"flac" => ["audio/flac"],
						"matroska,webm" => ["audio/webm"],
						"mp3" => ["audio/mp3", "audio/mpeg"],
						"ogg" => ["audio/ogg"],
						"wav" => ["audio/wav", "audio/x-wav"],
					}
				})
				.flatten()
				.collect()
		})
		.unwrap_or_default()
}

/// Root structure of `ffprobe` output
#[derive(Deserialize)]
struct FfprobeData<'data> {
	#[serde(borrow)]
	format: FfprobeDataFormat<'data>,
}

/// Structure of [`FfprobeData.format`](FfprobeData#structfield.format)
#[derive(Deserialize)]
struct FfprobeDataFormat<'data> {
	#[serde(borrow)]
	tags: FfprobeDataFormatTags<'data>,
}

/// Structure of [`FfprobeDataFormat.tags`](FfprobeDataFormat#structfield.tags)
#[derive(Deserialize)]
struct FfprobeDataFormatTags<'data> {
	title: Option<&'data str>,
	artist: Option<&'data str>,
	#[serde(alias = "ARTISTS")]
	artists: Option<&'data str>,
}

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

		Ok(
			[data.format.tags.title.and_then(|s| s.try_into().ok()), None]
				.into_iter()
				.map(From::from)
				.collect(),
		)
	})()
	.into()
}
