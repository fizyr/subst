//! Merge the runtime and compile-time environment.
//!
//! See [Issue #20](https://github.com/fizyr/subst/issues/20) for inspiration.
use std::borrow::Cow;

use subst::map::{fallback, map_value};

static STATIC_ENV: &[(&str, &str)] = {
	&[
		("CARGO", env!("CARGO")),
		("CARGO_PKG_VERSION", env!("CARGO_PKG_VERSION")),
		("CARGO_PKG_NAME", env!("CARGO_PKG_NAME")),
		("CARGO_PKG_DESCRIPTION", env!("CARGO_PKG_DESCRIPTION")),
		("CARGO_PKG_AUTHORS", env!("CARGO_PKG_AUTHORS")),
		("CARGO_PKG_HOMEPAGE", env!("CARGO_PKG_HOMEPAGE")),
		("CARGO_CRATE_NAME", env!("CARGO_CRATE_NAME")),
	]
};

pub fn main() {
	let template = subst::Template::from_str(include_str!("greeting.txt")).unwrap();

	println!("Substitution using Env:");
	println!(
		"{}",
		template
			.expand(&subst::Env)
			.expect_err("Env doesn't know anything about compile-time variables")
	);
	println!();

	println!("Substitution using STATIC_ENV:");
	println!(
		"{}",
		template
			.expand(STATIC_ENV)
			.expect_err("STATIC_ENV doesn't know anything about runtime variables.")
	);
	println!();

	println!("Substitution using Env, falling back to STATIC_ENV:");

	// `Env` returns `String`s, but `STATIC_ENV` returns `&str` references.
	let merged = fallback(
		map_value(subst::Env, Cow::<str>::Owned),
		map_value(STATIC_ENV, |&value| Cow::<str>::Borrowed(value)),
	);
	println!("{}", template.expand(&merged).unwrap());
}
