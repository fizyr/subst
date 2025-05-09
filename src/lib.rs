//! Shell-like variable substitution for strings and byte strings.
//!
//! # Features
//!
//! * Perform substitution in `&str` or in `&[u8]`.
//! * Provide a custom map of variables or use environment variables.
//!   * Support for `indexmap` (requires the `indexmap` feature).
//! * Short format: `"Hello $name!"`
//! * Long format: `"Hello ${name}!"`
//! * Default values: `"Hello ${name:person}!"`
//! * Recursive substitution in default values: `"${XDG_CONFIG_HOME:$HOME/.config}/my-app/config.toml"`
//! * Perform substitution on all string values in TOML, JSON or YAML data (optional, requires the `toml`, `json` or `yaml` feature).
//!
//! Variable names can consist of alphanumeric characters and underscores.
//! They are allowed to start with numbers.
//!
//! If you want to quickly perform substitution on a string, use [`substitute()`] or [`substitute_bytes()`].
//!
//! It is also possible to use one of the template types.
//! The templates parse the source string or bytes once, and can be expanded as many times as you want.
//! There are four different template types to choose from:
//! * [`Template`]: borrows the source string.
//! * [`TemplateBuf`]: owns the source string.
//! * [`ByteTemplate`]: borrows the source bytes.
//! * [`ByteTemplateBuf`]: owns the source bytes.
//!
//! # Examples
//!
//! The [`substitute()`] function can be used to perform substitution on a `&str`.
//! The variables can either be a [`HashMap`][std::collections::HashMap] or a [`BTreeMap`][std::collections::BTreeMap].
//!
//! ```
//! # fn main() -> Result<(), subst::Error> {
//! # use std::collections::HashMap;
//! let mut variables = HashMap::new();
//! variables.insert("name", "world");
//! assert_eq!(subst::substitute("Hello $name!", &variables)?, "Hello world!");
//! # Ok(())
//! # }
//! ```
//!
//! The variables can also be taken directly from the environment with the [`Env`] map.
//!
//! ```
//! # fn main() -> Result<(), subst::Error> {
//! # std::env::set_var("XDG_CONFIG_HOME", "/home/user/.config");
//! assert_eq!(
//!   subst::substitute("$XDG_CONFIG_HOME/my-app/config.toml", &subst::Env)?,
//!   "/home/user/.config/my-app/config.toml",
//! );
//! # Ok(())
//! # }
//! ```
//!
//! Substitution can also be done on byte strings using the [`substitute_bytes()`] function.
//!
//! ```
//! # fn main() -> Result<(), subst::Error> {
//! # use std::collections::HashMap;
//! let mut variables = HashMap::new();
//! variables.insert("name", b"world");
//! assert_eq!(subst::substitute_bytes(b"Hello $name!", &variables)?, b"Hello world!");
//! # Ok(())
//! # }
//! ```
//!
//! You can also parse a template once and expand it multiple times:
//!
//! ```
//! # fn main() -> Result<(), subst::Error> {
//! # use std::collections::HashMap;
//! let template = subst::Template::from_str("Welcome to our hair salon, $name!")?;
//! for name in ["Scrappy", "Coco"] {
//!   let variables: HashMap<_, _> = [("name", name)].into_iter().collect();
//!   let message = template.expand(&variables)?;
//!   println!("{}", message);
//! # assert_eq!(message, format!("Welcome to our hair salon, {name}!"));
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs, missing_debug_implementations)]
#![cfg_attr(feature = "doc-cfg", feature(doc_cfg))]

pub mod error;
pub use error::Error;

mod map;
pub use map::*;

mod template;
pub use template::*;

mod features;
#[allow(unused_imports)] // Might not re-export anything if all features are disabled.
pub use features::*;

mod non_aliasing;

/// Substitute variables in a string.
///
/// Variables have the form `$NAME`, `${NAME}` or `${NAME:default}`.
/// A variable name can only consist of ASCII letters, digits and underscores.
/// They are allowed to start with numbers.
///
/// You can escape dollar signs, backslashes, colons and braces with a backslash.
///
/// You can pass either a [`HashMap`][std::collections::HashMap], [`BTreeMap`][std::collections::BTreeMap] or [`Env`] as the `variables` parameter.
/// The maps must have [`&str`] or [`String`] keys, and the values must be [`AsRef<str>`].
pub fn substitute<'a, M>(source: &str, variables: &'a M) -> Result<String, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	let output = template::Template::from_str(source)?.expand(variables)?;
	Ok(output)
}

/// Substitute variables in a byte string.
///
/// Variables have the form `$NAME`, `${NAME}` or `${NAME:default}`.
/// A variable name can only consist of ASCII letters, digits and underscores.
/// They are allowed to start with numbers.
///
/// You can escape dollar signs, backslashes, colons and braces with a backslash.
///
/// You can pass either a [`HashMap`][std::collections::HashMap], [`BTreeMap`][std::collections::BTreeMap] as the `variables` parameter.
/// The maps must have [`&str`] or [`String`] keys, and the values must be [`AsRef<[u8]>`].
/// On Unix platforms, you can also use [`EnvBytes`].
pub fn substitute_bytes<'a, M>(source: &[u8], variables: &'a M) -> Result<Vec<u8>, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<[u8]>,
{
	let output = template::ByteTemplate::from_slice(source)?.expand(variables)?;
	Ok(output)
}

#[cfg(test)]
#[rustfmt::skip]
mod test {
	use super::*;
	use assert2::{assert, check, let_assert};
	use std::collections::BTreeMap;

	#[test]
	fn test_substitute() {
		let mut map: BTreeMap<String, String> = BTreeMap::new();
		map.insert("name".into(), "world".into());
		check!(let Ok("Hello world!") = substitute("Hello $name!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${not_name:world}!", &map).as_deref());

		let mut map: BTreeMap<&str, &str> = BTreeMap::new();
		map.insert("name", "world");
		check!(let Ok("Hello world!") = substitute("Hello $name!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${not_name:world}!", &map).as_deref());
	}

	#[test]
	fn substitution_in_default_value() {
		let mut map = BTreeMap::new();
		map.insert("name", "world");
		check!(let Ok("Hello cruel world!") = substitute("Hello ${not_name:cruel $name}!", &map).as_deref());
	}

	#[test]
	fn recursive_substitution_in_default_value() {
		let mut map = BTreeMap::new();
		check!(let Ok("Hello cruel world!") = substitute("Hello ${a:cruel ${b:world}}!", &map).as_deref());
		check!(let Ok("Hello cruel round world!") = substitute("Hello ${a:cruel ${b:round ${c:world}}}!", &map).as_deref());

		map.insert("c", "planet");
		check!(let Ok("Hello cruel round planet!") = substitute("Hello ${a:cruel ${b:round ${c:world}}}!", &map).as_deref());

		map.insert("b", "sphere");
		check!(let Ok("Hello cruel sphere!") = substitute("Hello ${a:cruel ${b:round ${c:world}}}!", &map).as_deref());

		map.insert("a", "spaceship");
		check!(let Ok("Hello spaceship!") = substitute("Hello ${a:cruel ${b:round ${c:world}}}!", &map).as_deref());
	}

	#[test]
	fn test_substitute_bytes() {
		let mut map: BTreeMap<String, Vec<u8>> = BTreeMap::new();
		map.insert("name".into(), b"world"[..].into());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello $name!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${not_name:world}!", &map).as_deref());

		let mut map: BTreeMap<&str, &[u8]> = BTreeMap::new();
		map.insert("name", b"world");
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello $name!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${not_name:world}!", &map).as_deref());
	}

	#[test]
	fn test_invalid_escape_sequence() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = r"Hello \world!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Invalid escape sequence: \w");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello \world!", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello \❤❤";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Invalid escape sequence: \❤");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello \❤❤", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello world!\";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Invalid escape sequence: missing escape character");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello world!\", "\n",
				r"              ^", "\n",
		));
	}

	#[test]
	fn test_missing_variable_name() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = r"Hello $!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello $!", "\n",
				r"        ^", "\n",
		));

		let source = r"Hello ${}!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello ${}!", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello ${:fallback}!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello ${:fallback}!", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello 　$❤";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello 　$❤", "\n",
				r"          ^", "\n",
		));
		let source = r"Hello 　$";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello 　$", "\n",
				r"          ^", "\n",
		));
	}

	#[test]
	fn test_unexpected_character() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name)!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Unexpected character: ')', expected a closing brace ('}') or colon (':')");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name)!\n",
				"              ^\n",
		));

		let source = "Hello ${name❤";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Unexpected character: '❤', expected a closing brace ('}') or colon (':')");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name❤\n",
				"              ^\n",
		));

		let source = b"\xE2\x98Hello ${name\xE2\x98";
		let_assert!(Err(e) = substitute_bytes(source, &map));
		assert!(e.to_string() == "Unexpected character: '\\xE2', expected a closing brace ('}') or colon (':')");
	}

	#[test]
	fn test_missing_closing_brace() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Missing closing brace");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name\n",
				"         ^\n",
		));

		let source = "Hello ${name:fallback";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Missing closing brace");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name:fallback\n",
				"         ^\n",
		));
	}

	#[test]
	fn test_substitute_no_such_variable() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name}!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "No such variable: $name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name}!\n",
				"          ^^^^\n",
		));

		let source = "Hello $name!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "No such variable: $name");
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				"  Hello $name!\n",
				"         ^^^^\n",
		));
	}

	#[test]
	fn test_dyn_variable_map() {
		let mut variables = BTreeMap::new();
		variables.insert(String::from("aap"), String::from("noot"));
		let variables: &dyn VariableMap<Value = &String> = &variables;

		let_assert!(Ok(expanded) = substitute("one ${aap}", variables));
		assert!(expanded == "one noot");
	}

	#[test]
	fn test_unicode_invalid_escape_sequence() {
		let mut variables = BTreeMap::new();
		variables.insert(String::from("aap"), String::from("noot"));

		let source = r"emoticon: \（ ^▽^ ）/";
		let_assert!(Err(e) = substitute(source, &variables));
		#[rustfmt::skip]
		assert!(e.source_highlighting(source) == concat!(
				r"  emoticon: \（ ^▽^ ）/", "\n",
				r"            ^^^", "\n",
		));
	}

	#[test]
	fn test_vec_map() {
		let test_vec = vec!["xxxx", "foo", "bar"];
		let test_slice = &test_vec[..];
		let_assert!(Err(e) = substitute("${99}", &test_slice));
		assert!(e.to_string() == "No such variable: $99");
		assert_eq!(Ok("foo bar"), substitute("${*}", &test_slice).as_deref());
		assert_eq!(Ok("xxxx"), substitute("${0}", &test_slice).as_deref());
		assert_eq!(Ok("foo"), substitute("${1}", &test_slice).as_deref());
		assert_eq!(Ok("bar"), substitute("${2}", &test_slice).as_deref());
		assert_eq!(Ok("foo bar"), substitute("$*", &test_slice).as_deref());
		assert_eq!(Ok("foo"), substitute("$1", &test_slice).as_deref());
		assert_eq!(Ok("bar"), substitute("$2", &test_slice).as_deref());
	}
}
