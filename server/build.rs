use std::io::{self, Write};

fn main() -> io::Result<()> {
	use std::fs::File;

	println!("cargo:rerun-if-changed=Cargo.toml");

	// Retrieve variables for Makefile
	let mut dotenv = File::create("build.env")?;
	for var in ["CARGO_PKG_NAME"] {
		writeln!(
			&mut dotenv,
			"{var}={}",
			std::env::var(var).unwrap_or_else(|_| panic!("missing environment variable \"{var}\""))
		)?;
	}
	dotenv.flush()
}
