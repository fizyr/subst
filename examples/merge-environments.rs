//! Merge the runtime and compile-time environment.
//!
//! See [Issue #20](https://github.com/fizyr/subst/issues/20) for inspiration.
use std::sync::LazyLock;

use subst::{fallback, map_value, Env, Template};

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

static TEMPLATE: LazyLock<Template> = LazyLock::new(|| {
	Template::from_str(
		r#"
Hello $USER!

It's nice to see you! Here is some information about the current environment:

\$CARGO:                 $CARGO
\$CARGO_PKG_NAME:        $CARGO_PKG_NAME
\$CARGO_PKG_VERSION:     $CARGO_PKG_VERSION
\$CARGO_PKG_DESCRIPTION: $CARGO_PKG_DESCRIPTION
\$CARGO_PKG_AUTHORS:     $CARGO_PKG_AUTHORS
\$CARGO_PKG_HOMEPAGE:    $CARGO_PKG_HOMEPAGE
\$CARGO_CRATE_NAME:      $CARGO_CRATE_NAME
\$PATH:                  $PATH
"#,
	)
	.unwrap()
});

pub fn main() {
	println!("Substitution using Env:");
	println!(
		"{}",
		TEMPLATE
			.expand(&Env)
			.expect_err("Env doesn't know anything about compile-time variables")
	);
	println!();

	println!("Substitution using STATIC_ENV:");
	println!(
		"{}",
		TEMPLATE
			.expand(STATIC_ENV)
			.expect_err("STATIC_ENV doesn't know anything about runtime variables.")
	);
	println!();

	println!("Substitution using Env, falling back to STATIC_ENV:");

	let merged = fallback(
		Env,
		// `Env` returns `String`s, but `STATIC_ENV` returns `&str` references.
		// We have to convert the `&str` references to valid `String`s,
		// so that `fallback` can merge them together and build a single variable map.
		map_value(STATIC_ENV, |value| (*value).to_owned()),
	);
	println!("{}", TEMPLATE.expand(&merged).unwrap());
}
