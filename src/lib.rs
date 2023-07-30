//! Shell-like variable substitution for strings and byte strings.
//!
//! # Features
//!
//! * Perform substitution in `&str` or in `&[u8]`.
//! * Provide a custom map of variables or use environment variables.
//! * Short format: `"Hello $name!"`
//! * Long format: `"Hello ${name}!"`
//! * Default values: `"Hello ${name:person}!"`
//! * Recursive substitution in default values: `"${XDG_CONFIG_HOME:$HOME/.config}/my-app/config.toml"`
//! * Perform substitution on all string values in YAML data (optional, requires the `yaml` feature).
//!
//! Variable names can consist of alphanumeric characters and underscores.
//! They are allowed to start with numbers.
//!
//! # Examples
//!
//! The [`substitute()`][substitute] function can be used to perform substitution on a `&str`.
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
//! The variables can also be taken directly from the environment with the [`Env`][Env] map.
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
//! Substitution can also be done on byte strings using the [`substitute_bytes()`][substitute_bytes] function.
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
#![warn(missing_docs, missing_debug_implementations)]

use bstr::{BStr, ByteSlice, ByteVec};

pub mod error;
pub use error::Error;

mod map;
pub use map::*;

#[cfg(feature = "yaml")]
pub mod yaml;

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
	let mut output = Vec::with_capacity(source.len() + source.len() / 8);
	substitute_impl(&mut output, source.as_bytes(), 0, variables, |v| v.as_ref().as_bytes())?;
	// SAFETY: Both source and all variable values are valid UTF-8, so substitation result is also valid UTF-8.
	#[cfg(not(debug_assertions))]
	unsafe {
		Ok(String::from_utf8_unchecked(output))
	}
	#[cfg(debug_assertions)]
	Ok(String::from_utf8(output).unwrap())
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
	let mut output = Vec::with_capacity(source.len() + source.len() / 8);
	substitute_impl(&mut output, source, 0, variables, |v| v.as_ref())?;
	Ok(output)
}

/// Substitute variables in a byte string.
///
/// This is the real implementation used by both [`substitute`] and [`substitute_bytes`].
/// `source_start` is used for diagnostic purposes and
/// should be the position of the start of `source` relative to the original input.
fn substitute_impl<'a, M, F>(
	output: &mut Vec<u8>,
	mut source: &[u8],
	source_start: usize,
	variables: &'a M,
	value_to_bytes: F,
) -> Result<(), Error>
where
	M: VariableMap<'a> + ?Sized,
	F: Fn(&M::Value) -> &[u8] + Copy,
{
	while !source.is_empty() {
		let idx = match memchr::memchr2(b'$', b'\\', source) {
			Some(idx) => idx,
			None => break,
		};

		let (head, body) = source.split_at(idx);
		output.extend_from_slice(head);

		if body[0] == b'\\' {
			let escaped_char = unescape_one(body, source_start + idx)?;
			output.push_char(escaped_char);
			source = &body[escaped_char.len_utf8()..];
		} else {
			let variable = Variable::parse(body, source_start + idx)?;
			let value = variables.get(variable.name);
			match (value, variable.default) {
				(None, None) => {
					return Err(error::NoSuchVariable {
						position: source_start + variable.name_start,
						name: variable.name.to_owned(),
					}
					.into())
				},
				(Some(value), _) => {
					output.extend_from_slice(value_to_bytes(&value));
				},
				(None, Some(default)) => {
					substitute_impl(output, default, source_start + idx, variables, value_to_bytes)?;
				},
			};
			source = &body[variable.len..];
		}
	}

	output.extend_from_slice(source);
	Ok(())
}

/// A parsed variable.
#[derive(Debug)]
struct Variable<'a> {
	/// The name of the variable.
	name: &'a str,

	/// The start position of the name in the source.
	name_start: usize,

	/// The default value of the variable.
	default: Option<&'a BStr>,

	/// The length of the entire variable.
	len: usize,
}

impl<'a> Variable<'a> {
	/// Parse a variable from source.
	///
	/// `source_start` is used for diagnostic purposes and
	/// should be the position of the dollar sign in the original input.
	fn parse(source: &'a [u8], source_start: usize) -> Result<Self, Error> {
		let mut chars = source.char_indices();
		chars.next(); // '$'

		let (name_start, _, first_char_after_dollar) = match chars.next() {
			Some(t) if is_valid_name(t.2) || t.2 == '{' => t,
			_ => {
				return Err(error::MissingVariableName {
					position: source_start,
					len: 1,
				}
				.into())
			},
		};

		let name_end = match chars.find(|&(_, _, c)| !is_valid_name(c)) {
			Some((first_byte_after_name, _, _)) => first_byte_after_name,
			None => source.len(),
		};

		if first_char_after_dollar != '{' {
			Ok(Self {
				// Valid names are ASCII, so unwrap() is fine.
				name: std::str::from_utf8(&source[name_start..name_end]).unwrap(),
				name_start: source_start + name_start,
				default: None,
				len: name_end - name_start + 1,
			})
		} else {
			// For braced variables, skip the starting brace for `name_start`.
			let name_start = name_start + 1;
			if name_start == name_end {
				return Err(error::MissingVariableName {
					position: source_start,
					len: 2,
				}
				.into());
			}

			// Valid names are ASCII, so unwrap() is fine.
			let name = std::str::from_utf8(&source[name_start..name_end]).unwrap();

			let after_name = &source[name_end..];
			match after_name.chars().next() {
				// If there is a closing brace after the name, there is no default value and we're done.
				Some('}') => {
					return Ok(Self {
						name,
						name_start: source_start + name_start,
						default: None,
						len: name_end - name_start + 3,
					});
				}
				Some(':') => (),
				// If there is something other than a closing brace or colon after the name, it's an error.
				Some(other) => {
					return Err(error::UnexpectedCharacter {
						position: source_start + name_end,
						character: other,
						expected: error::ExpectedCharacter {
							message: "a closing brace ('}') or colon (':')",
						},
					}
					.into());
				},
				None => {
					return Err(error::MissingClosingBrace {
						position: source_start + 1,
					}
					.into());
				},
			}

			// If there is no un-escaped closing brace, it's missing.
			let default_end = name_end
				+ find_non_escaped(b'}', after_name).ok_or(error::MissingClosingBrace {
					position: source_start + 1,
				})?;

			Ok(Self {
				name,
				name_start: source_start + name_start,
				default: Some(source[name_end + 1..default_end].into()),
				len: default_end - name_start + 3,
			})
		}
	}
}

/// Find the first non-escaped occurrence of a character.
fn find_non_escaped(needle: u8, haystack: &[u8]) -> Option<usize> {
	let mut finger = 0;
	while finger < haystack.len() {
		let candidate = memchr::memchr2(b'\\', needle, &haystack[finger..])?;
		if haystack[finger + candidate] == b'\\' {
			if candidate == haystack.len() - 1 {
				return None;
			}
			finger += candidate + 2;
		} else {
			return Some(finger + candidate);
		}
	}
	None
}

/// Unescape a single escape sequence in source at the given position.
///
/// `source_start` is used for diagnostic purposes and
/// should be the position of the backslash in the original input.
///
/// Only valid escape sequences ('\$' '\{' '\}' and '\:') are accepted.
/// Invalid escape sequences cause an error to be returned.
fn unescape_one(source: &[u8], source_start: usize) -> Result<char, Error> {
	let mut chars = source.chars();
	let _backslash = chars.next();
	debug_assert_eq!(_backslash, Some('\\'));

	match chars.next() {
		Some('\\') => Ok('\\'),
		Some('$') => Ok('$'),
		Some('{') => Ok('{'),
		Some('}') => Ok('}'),
		Some(':') => Ok(':'),
		Some(other) => Err(error::InvalidEscapeSequence {
			position: source_start,
			character: Some(other),
		}
		.into()),
		None => Err(error::InvalidEscapeSequence {
			position: source_start,
			character: None,
		}
		.into()),
	}
}

/// Variable names consist of alphanumeric characters and underscores.
fn is_valid_name(c: char) -> bool {
	c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;
	use assert2::{assert, check, let_assert};
	use super::*;

	#[test]
	fn test_find_non_escaped() {
		check!(find_non_escaped(b'$', b"$foo") == Some(0));
		check!(find_non_escaped(b'$', b"\\$foo$") == Some(5));
		check!(find_non_escaped(b'$', b"foo $bar") == Some(4));
		check!(find_non_escaped(b'$', b"foo \\$$bar") == Some(6));
	}

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
		let mut map: BTreeMap<String, String> = BTreeMap::new();
		map.insert("name".into(), "world".into());
		check!(let Ok("Hello cruel world!") = substitute("Hello ${not_name:cruel $name}!", &map).as_deref());
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
		map.insert("name", b"\x87");
		check!(let Ok(b"Hello \x87!") = substitute_bytes(b"Hello $name!", &map).as_deref());
		check!(let Ok(b"Hello \x87!") = substitute_bytes(b"Hello ${name}!", &map).as_deref());
		check!(let Ok(b"Hello \x87!") = substitute_bytes(b"Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok(b"Hello \x9F!") = substitute_bytes(b"Hello ${not_name:\x9F}!", &map).as_deref());
	}

	#[test]
	#[rustfmt::skip]
	fn test_invalid_escape_sequence() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = r"Hello \world!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Invalid escape sequence: \w");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello \world!", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello \❤❤";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Invalid escape sequence: \❤");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello \❤❤", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello world!\";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Invalid escape sequence: missing escape character");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello world!\", "\n",
				r"              ^", "\n",
		));
	}

	#[test]
	#[rustfmt::skip]
	fn test_missing_variable_name() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = r"Hello $!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello $!", "\n",
				r"        ^", "\n",
		));

		let source = r"Hello ${}!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello ${}!", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello ${:fallback}!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello ${:fallback}!", "\n",
				r"        ^^", "\n",
		));

		let source = r"Hello 　$❤";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == r"Missing variable name");
		assert!(e.source_highlighting(source) == concat!(
				r"  Hello 　$❤", "\n",
				r"          ^", "\n",
		));
	}

	#[test]
	#[rustfmt::skip]
	fn test_unexpected_character() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name)!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Unexpected character: ')', expected a closing brace ('}') or colon (':')");
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name)!\n",
				"              ^\n",
		));

		let source = "Hello ${name❤";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Unexpected character: '❤', expected a closing brace ('}') or colon (':')");
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name❤\n",
				"              ^\n",
		));

		let source = b"\xE2\x98Hello ${name\xE2\x98";
		let_assert!(Err(e) = substitute_bytes(source, &map));
		assert!(e.to_string() == "Unexpected character: '�', expected a closing brace ('}') or colon (':')");
	}

	#[test]
	#[rustfmt::skip]
	fn test_missing_closing_brace() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Missing closing brace");
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name\n",
				"         ^\n",
		));

		let source = "Hello ${name:fallback";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "Missing closing brace");
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name:fallback\n",
				"         ^\n",
		));
	}

	#[test]
	#[rustfmt::skip]
	fn test_substitute_no_such_variable() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name}!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "No such variable: $name");
		assert!(e.source_highlighting(source) == concat!(
				"  Hello ${name}!\n",
				"          ^^^^\n",
		));

		let source = "Hello $name!";
		let_assert!(Err(e) = substitute(source, &map));
		assert!(e.to_string() == "No such variable: $name");
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
}
