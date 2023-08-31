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
	let mut output = Vec::with_capacity(source.len() + source.len() / 10);
	substitute_impl(&mut output, source.as_bytes(), 0..source.len(), variables, &|x| x.as_ref().as_bytes())?;
	// SAFETY: Both source and all variable values are valid UTF-8, so substitation result is also valid UTF-8.
	unsafe {
		Ok(String::from_utf8_unchecked(output))
	}
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
	let mut output = Vec::with_capacity(source.len() + source.len() / 10);
	substitute_impl(&mut output, source, 0..source.len(), variables, &|x| x.as_ref())?;
	Ok(output)
}

/// Substitute variables in a byte string.
///
/// This is the real implementation used by both [`substitute`] and [`substitute_bytes`].
/// The function accepts any type that implements [`VariableMap`], and a function to convert the value from the map into bytes.
fn substitute_impl<'a, M, F>(output: &mut Vec<u8>, source: &[u8], range: std::ops::Range<usize>, variables: &'a M, to_bytes: &F) -> Result<(), Error>
where
	M: VariableMap<'a> + ?Sized,
	F: Fn(&M::Value) -> &[u8],
{
	let mut finger = range.start;
	while finger < range.end {
		let next = match memchr::memchr2(b'$', b'\\', &source[finger..range.end]) {
			Some(x) => finger + x,
			None => break,
		};

		output.extend_from_slice(&source[finger..next]);
		if source[next] == b'\\' {
			output.push(unescape_one(source, next)?);
			finger = next + 2;
		} else {
			let variable = parse_variable(source, next)?;
			let value = variables.get(variable.name);
			match (&value, &variable.default) {
				(None, None) => return Err(error::NoSuchVariable {
					position: variable.name_start,
					name: variable.name.to_owned(),
				}.into()),
				(Some(value), _) => {
					output.extend_from_slice(to_bytes(value));
				}
				(None, Some(default)) => {
					substitute_impl(output, source, default.clone(), variables, to_bytes)?;
				}
			};
			finger = variable.end_position;
		}
	}

	output.extend_from_slice(&source[finger..range.end]);
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
	default: Option<std::ops::Range<usize>>,

	/// The end position of the entire variable in the source.
	end_position: usize,
}

/// Parse a variable from source at the given position.
///
/// The finger must be the position of the dollar sign in the source.
fn parse_variable(source: &[u8], finger: usize) -> Result<Variable, Error> {
	if finger == source.len() {
		return Err(error::MissingVariableName {
			position: finger,
			len: 1,
		}.into())
	}
	if source[finger + 1] == b'{' {
		parse_braced_variable(source, finger)
	} else {
		let name_end = match source[finger + 1..].iter().position(|&c| !c.is_ascii_alphanumeric() && c != b'_') {
			Some(0) => return Err(error::MissingVariableName {
				position: finger,
				len: 1,
			}.into()),
			Some(x) => finger + 1 + x,
			None => source.len(),
		};
		Ok(Variable {
			name: std::str::from_utf8(&source[finger + 1..name_end]).unwrap(),
			name_start: finger + 1,
			default: None,
			end_position: name_end,
		})
	}
}

/// Parse a braced variable in the form of "${name[:default]} from source at the given position.
///
/// The finger must be the position of the dollar sign in the source.
fn parse_braced_variable(source: &[u8], finger: usize) -> Result<Variable, Error> {
	let name_start = finger + 2;
	if name_start >= source.len() {
		return Err(error::MissingVariableName {
			position: finger,
			len: 2,
		}.into())
	}

	// Get the first sequence of alphanumeric characters and underscores for the variable name.
	let name_end = match source[name_start..].iter().position(|&c| !c.is_ascii_alphanumeric() && c != b'_') {
		Some(0) => return Err(error::MissingVariableName {
			position: finger,
			len: 2,
		}.into()),
		Some(x) => name_start + x,
		None => source.len(),
	};

	// If the name extends to the end, we're missing a closing brace.
	if name_end == source.len() {
		return Err(error::MissingClosingBrace {
			position: finger + 1,
		}.into())
	}

	// If there is a closing brace after the name, there is no default value and we're done.
	if source[name_end] == b'}' {
		return Ok(Variable {
			name: std::str::from_utf8(&source[name_start..name_end]).unwrap(),
			name_start,
			default: None,
			end_position: name_end + 1,
		});

	// If there is something other than a closing brace or colon after the name, it's an error.
	} else if source[name_end] != b':' {
		return Err(error::UnexpectedCharacter {
			position: name_end,
			character: get_maybe_char_at(source, name_end),
			expected: error::ExpectedCharacter { message: "a closing brace ('}') or colon (':')" },
		}.into());
	}

	// If there is no un-escaped closing brace, it's missing.
	let end = finger + find_non_escaped(b'}', &source[finger..])
		.ok_or(error::MissingClosingBrace {
			position: finger + 1,
		})?;

	Ok(Variable {
		name: std::str::from_utf8(&source[name_start..name_end]).unwrap(),
		name_start,
		default: Some(name_end + 1..end),
		end_position: end + 1,
	})
}

/// Get the prefix from the input that is valid UTF-8 as [`str`].
///
/// If the whole input is valid UTF-8, the whole input is returned.
/// If the first byte is already invalid UTF-8, an empty string is returned.
fn valid_utf8_prefix(input: &[u8]) -> &str {
	// The unwrap can not panic: we used `e.valid_up_to()` to get the valid UTF-8 slice.
	std::str::from_utf8(input)
		.or_else(|e| std::str::from_utf8(&input[..e.valid_up_to()]))
		.unwrap()
}

/// Get the character at a given index.
///
/// If the data at the given index contains a valid UTF-8 sequence,
/// returns a [`error::CharOrByte::Char`].
/// Otherwise, returns a [`error::CharOrByte::Byte`].
fn get_maybe_char_at(data: &[u8], index: usize) -> error::CharOrByte {
	let head = &data[index..];
	let head = &head[..head.len().min(4)];
	assert!(!head.is_empty(), "index out of bounds: data.len() is {} but index is {}", data.len(), index);

	let head = valid_utf8_prefix(head);
	if let Some(c) = head.chars().next() {
		error::CharOrByte::Char(c)
	} else {
		error::CharOrByte::Byte(data[index])
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
			return Some(finger + candidate)
		}
	}
	None
}

/// Unescape a single escape sequence in source at the given position.
///
/// The `position` must point to the backslash character in the source text.
///
/// Only valid escape sequences ('\$' '\{' '\}' and '\:') are accepted.
/// Invalid escape sequences cause an error to be returned.
fn unescape_one(source: &[u8], position: usize) -> Result<u8, Error> {
	if position == source.len() - 1 {
		return Err(error::InvalidEscapeSequence {
			position,
			character: None,
		}.into())
	}
	match source[position + 1] {
		b'\\' => Ok(b'\\'),
		b'$' => Ok(b'$'),
		b'{' => Ok(b'{'),
		b'}' => Ok(b'}'),
		b':' => Ok(b':'),
		_ => Err(error::InvalidEscapeSequence {
			position,
			character: Some(get_maybe_char_at(source, position + 1)),
		}.into())
	}
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;
	use assert2::{assert, check, let_assert};
	use super::*;

	#[test]
	fn test_get_maybe_char_at() {
		use error::CharOrByte::{Char, Byte};
		assert!(get_maybe_char_at(b"hello", 0) == Char('h'));
		assert!(get_maybe_char_at(b"he", 0) == Char('h'));
		assert!(get_maybe_char_at(b"hello", 1) == Char('e'));
		assert!(get_maybe_char_at(b"he", 1) == Char('e'));
		assert!(get_maybe_char_at(b"hello\x80", 1) == Char('e'));
		assert!(get_maybe_char_at(b"he\x80llo\x80", 1) == Char('e'));

		assert!(get_maybe_char_at(b"h\x79", 1) == Char('\x79'));
		assert!(get_maybe_char_at(b"h\x80llo", 1) == Byte(0x80));

		// The UTF-8 sequence for '❤' is [0xE2, 0x9D, 0xA4]".
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 0) == Char('h'));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 1) == Char('❤'));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 2) == Byte(0x9d));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 3) == Byte(0xA4));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 4) == Char('l'));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 5) == Char('l'));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 6) == Char('❤'));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 7) == Byte(0x9d));
		assert!(get_maybe_char_at("h❤ll❤".as_bytes(), 8) == Byte(0xA4));
	}

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
		assert!(e.to_string() == "Unexpected character: '\\xE2', expected a closing brace ('}') or colon (':')");
	}

	#[test]
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

	#[test]
	fn test_unicode_invalid_escape_sequence() {
		let mut variables = BTreeMap::new();
		variables.insert(String::from("aap"), String::from("noot"));

		let source = r"emoticon: \（ ^▽^ ）/";
		let_assert!(Err(e) = substitute(source, &variables));
		assert!(e.source_highlighting(source) == concat!(
				r"  emoticon: \（ ^▽^ ）/", "\n",
				r"            ^^^", "\n",
		));
	}
}
