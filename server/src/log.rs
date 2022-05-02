use std::fmt::Display;
use yansi::Paint;

#[inline]
pub(crate) fn info<D: Display>(header: &str, body: D) {
	println!("{} {body}", Paint::blue(header).bold())
}
#[allow(unused_macros)]
macro_rules! console_info {
	($header:expr, $($fmt_arg:tt)*) => {
		$crate::log::info($header, format!($($fmt_arg)*))
	};
}
#[allow(unused_imports)]
pub(crate) use console_info;

#[inline]
pub(crate) fn log<D: Display>(header: &str, body: D) {
	println!("{} {body}", Paint::green(header).bold())
}
#[allow(unused_macros)]
macro_rules! console_log {
	($header:expr, $($fmt_arg:tt)*) => {
		$crate::log::log($header, format!($($fmt_arg)*))
	};
}
#[allow(unused_imports)]
pub(crate) use console_log;

#[inline]
pub(crate) fn warn<D: Display>(header: &str, body: D, file: &'static str, line: u32, column: u32) {
	eprintln!(
		"{} {body} ({})",
		Paint::yellow(header).bold(),
		Paint::new(format!("in {file}:{line}:{column}")).bold()
	)
}
#[allow(unused_macros)]
macro_rules! console_warn {
	($header:expr, $($fmt_arg:tt)*) => {
		$crate::log::warn($header, format!($($fmt_arg)*), file!(), line!(), column!())
	};
}
#[allow(unused_imports)]
pub(crate) use console_warn;

#[inline]
pub(crate) fn error<D: Display>(header: &str, body: D, file: &'static str, line: u32, column: u32) {
	eprintln!(
		"{} {body} ({})",
		Paint::red(header).bold(),
		Paint::new(&format!("in {file}:{line}:{column}")).bold()
	)
}
#[allow(unused_macros)]
macro_rules! console_error {
	($header:expr, $($fmt_arg:tt)*) => {
		$crate::log::error($header, format!($($fmt_arg)*), file!(), line!(), column!())
	};
}
#[allow(unused_imports)]
pub(crate) use console_error;
