#![forbid(unsafe_code)]
#![deny(unused_must_use)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::{
	io::Read,
	path::{Path, PathBuf},
};
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
};

struct IncludeArgs {
	path: String,
	span: Span,
}
impl Parse for IncludeArgs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		use syn::LitStr;

		let span = input.span();
		let path: LitStr = input.parse()?;
		Ok(Self {
			path: path.value(),
			span,
		})
	}
}

fn join_crate_root<P: AsRef<Path>>(path: P) -> PathBuf {
	Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join(path)
}

#[proc_macro]
pub fn include_static_config(args: TokenStream) -> TokenStream {
	use std::{env, fs::File};

	let IncludeArgs { path, span } = parse_macro_input!(args as IncludeArgs);
	let path = join_crate_root(path);
	match File::open(path)
		.and_then(|mut file| {
			let mut buf = String::new();
			file.read_to_string(&mut buf)
				.map(|_| buf.replace("{version}", &env::var("CARGO_PKG_VERSION").unwrap()))
		})
		.map_err(|err| syn::Error::new(span, err))
	{
		Ok(config) => quote! { #config }.into(),
		Err(err) => err.into_compile_error().into(),
	}
}

#[proc_macro]
pub fn include_migrations(args: TokenStream) -> TokenStream {
	use std::fs;

	let IncludeArgs { path, span } = parse_macro_input!(args as IncludeArgs);
	let path = join_crate_root(path);
	match fs::read_dir(&path)
		.map(|dir| {
			let mut migrations = dir
				.filter_map(|file| {
					let file = file.ok()?;
					let file_name = file.file_name().into_string().ok()?;
					(file_name.starts_with("migration_v") && file_name.ends_with(".sql")).then(
						|| {
							let v: u32 = file_name
								.chars()
								.filter(|c| c.is_ascii_digit())
								.collect::<String>()
								.parse()
								.unwrap();
							(v, fs::read_to_string(file.path()).unwrap())
						},
					)
				})
				.collect::<Vec<_>>();
			migrations.sort_by_key(|(v, _)| *v);
			migrations
				.into_iter()
				.map(|(_, contents)| contents)
				.collect::<Vec<_>>()
		})
		.map_err(|err| syn::Error::new(span, err))
	{
		Ok(migrations) => quote! {
			&[#(#migrations),*]
		}
		.into(),
		Err(err) => err.into_compile_error().into(),
	}
}
