//! Plugin library for *Aedron Patchouli*
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
	unsafe_op_in_unsafe_fn,
	unused_must_use,
	clippy::exit,
	clippy::lossy_float_literal,
)]
#![forbid(clippy::undocumented_unsafe_blocks)]

#[cfg(feature = "server")]
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	fmt::{self, Display, Formatter},
	num::ParseIntError,
	str::FromStr,
};

pub mod ffi;
#[cfg(feature = "media")]
pub mod media;

/// [SemVer](https://semver.org) structure
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "server", derive(Serialize, Deserialize))]
pub struct Version {
	/// Major version
	///
	/// Incremented with API-breaking changes.
	pub major: u8,
	/// Minor version
	///
	/// Incremented with backward-compatible changes.
	pub minor: u8,
	/// Patch version
	///
	/// Incremented with backward-compatible fixes.
	pub patch: u8,
}
impl Version {
	/// Checks if this version is compatible with an other
	#[inline]
	pub fn is_compatible(&self, other: &Self) -> bool {
		if *self == Self::default() || *other == Self::default() {
			false
		} else if self.major == 0 && other.major == 0 {
			self.minor == other.minor
		} else {
			self.major == other.major
		}
	}
}
impl FromStr for Version {
	type Err = ParseIntError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut parts = s.split('.');
		/// Parses the next version part
		macro_rules! parse_part {
			() => {
				match parts.next() {
					Some(s) => s.parse()?,
					None => 0,
				}
			};
		}
		let major = parse_part!();
		let minor = parse_part!();
		let patch = parse_part!();

		Ok(Self {
			major,
			minor,
			patch,
		})
	}
}
impl PartialOrd for Version {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
impl Ord for Version {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering {
		self.major
			.cmp(&other.major)
			.then_with(|| self.minor.cmp(&other.minor))
			.then_with(|| self.patch.cmp(&other.patch))
	}
}
impl Display for Version {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "v{}.{}.{}", self.major, self.minor, self.patch)
	}
}

/// Signature of the `plugin_version` function that plugins must export
pub type PluginVersion = extern "C" fn() -> Version;
