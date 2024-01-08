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
	let parser = TemplateParser {
		allow_variable_name_starting_with_number: true,
	};
	let template = parser.parse(source)?;
	let expander = TemplateExpander::new(variables, &from_str_to_bytes::<M::Value>);
	expander.expand_template(&template)
}

fn from_bytes_to_bytes<V: AsRef<[u8]>>(x: &V) -> &[u8] {
	x.as_ref()
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
	let parser = TemplateParser {
		allow_variable_name_starting_with_number: true,
	};
	let template = parser.parse_template_range(source, &(0..source.len()))?;
	let mut output = Vec::with_capacity(source.len() + source.len() / 10);
	let expander = TemplateExpander::new(variables, &from_bytes_to_bytes);
	expander.expand_template_impl(&template, &mut output)?;
	Ok(output)
}

#[derive(Debug, PartialEq, Eq)]
enum TemplatePart<'a> {
	Literal(LiteralTemplate<'a>),
	Variable(Variable<'a>),
	EscapedChar(EscapedCharTemplate),
}

#[derive(Debug, PartialEq, Eq)]
struct LiteralTemplate<'a> {
	text: &'a [u8],
}

trait ByteLength {
	fn size(&self) -> usize;
}

impl<'a> ByteLength for TemplatePart<'a> {
	fn size(&self) -> usize {
		match self {
			Self::Literal(l) => l.size(),
			Self::Variable(v) => v.size(),
			Self::EscapedChar(e) => e.size(),
		}
	}
}

impl<'a> ByteLength for LiteralTemplate<'a> {
	fn size(&self) -> usize {
		self.text.len()
	}
}

impl<'a> ByteLength for Variable<'a> {
	fn size(&self) -> usize {
		self.part_end - self.part_start
	}
}

impl ByteLength for EscapedCharTemplate {
	fn size(&self) -> usize {
		2
	}
}

#[derive(Debug, PartialEq, Eq)]
struct EscapedCharTemplate {
	name: u8,
}

impl TemplateParser {
	fn parse_template_one_step<'a>(
		&self,
		finger: usize,
		source: &'a [u8],
		range: &std::ops::Range<usize>,
	) -> Result<Option<TemplatePart<'a>>, Error> {
		if finger >= range.end {
			return Ok(None); // end of input is reached
		}

		let c = source.get(finger).unwrap();

		let part: TemplatePart = match c {
			b'$' => TemplatePart::Variable(self.parse_variable(source, finger)?),
			b'\\' => {
				let c = unescape_one(source, finger)?;
				TemplatePart::EscapedChar(EscapedCharTemplate { name: c })
			},
			_c0 => match memchr::memchr2(b'$', b'\\', &source[finger..range.end]) {
				Some(x) => TemplatePart::Literal(LiteralTemplate {
					text: &source[finger..finger + x],
				}),
				None => TemplatePart::Literal(LiteralTemplate {
					text: &source[finger..range.end],
				}),
			},
		};

		Ok(Some(part))
	}
}

/// Provides configurable expanding of template
#[derive(Debug)]
pub struct TemplateExpander<'a, M, F>
where
	M: VariableMap<'a> + ?Sized,
	F: Fn(&M::Value) -> &[u8],
{
	variables: &'a M,
	to_bytes: &'a F,
	silent_missing_variables: bool,
}

impl<'a, M, F> TemplateExpander<'a, M, F>
where
	M: VariableMap<'a> + ?Sized,
	F: Fn(&M::Value) -> &[u8],
{
	fn new(variables: &'a M, to_bytes: &'a F) -> Self {
		Self {
			variables,
			to_bytes,
			silent_missing_variables: false,
		}
	}
}

fn from_str_to_bytes<V: AsRef<str>>(x: &V) -> &[u8] {
	x.as_ref().as_bytes()
}

impl<'b, M, F> TemplateExpander<'b, M, F>
where
	M: VariableMap<'b> + ?Sized,
	F: Fn(&M::Value) -> &[u8],
{
	/// allows convenient activation/deactivation of setting
	pub fn set_silent_missing_variables(mut self, value: bool) -> Self {
		self.silent_missing_variables = value;
		self
	}
}

/// This class can be constructed by providing a template input string.
/// This input string is parsed into TemplateParts which are stored in memory.
/// With calling `expand` the template gets instantiated by substitution of the variables and escape sequences.
#[derive(Debug)]
pub struct Template<'a> {
	source: &'a [u8],
	parts: Vec<TemplatePart<'a>>,
}

impl<'a> Template<'a> {
	/// expands all the fields in the template and returns result
	pub fn expand<'b, M, F>(&self, config: &TemplateExpander<'b, M, F>) -> Result<String, Error>
	where
		M: VariableMap<'b> + ?Sized,
		F: Fn(&M::Value) -> &[u8],
	{
		config.expand_template_parts_vec(&self.parts, Some(self.source.len()))
	}
}

/// provide configurable template parsing features
#[derive(Debug)]
pub struct TemplateParser {
	allow_variable_name_starting_with_number: bool,
}

impl Default for TemplateParser {
	fn default() -> Self {
		Self {
			allow_variable_name_starting_with_number: true,
		}
	}
}

impl TemplateParser {
	/// Creates a new template from a string
	pub fn parse<'a>(&'a self, source: &'a str) -> Result<Template<'a>, Error> {
		Ok(Template {
			source: source.as_bytes(),
			parts: self.parse_template_range(source.as_bytes(), &(0..source.len()))?,
		})
	}

	fn parse_template_range<'a>(
		&self,
		source: &'a [u8],
		range: &std::ops::Range<usize>,
	) -> Result<Vec<TemplatePart<'a>>, Error> {
		let mut parts: Vec<TemplatePart<'a>> = Vec::<TemplatePart<'a>>::new();
		let mut finger = range.start;
		while let Some(part) = self.parse_template_one_step(finger, source, range)? {
			finger += part.size();
			parts.push(part);
		}

		Ok(parts)
	}

	/// parses a new template from a sequence of bytes
	pub fn parse_from_bytes<'a>(&'a self, source: &'a [u8]) -> Result<Template<'a>, Error> {
		Ok(Template {
			source,
			parts: self.parse_template_range(source, &(0..source.len()))?,
		})
	}
}

impl<'a, M, F> TemplateExpander<'a, M, F>
where
	M: VariableMap<'a> + ?Sized,
	F: Fn(&M::Value) -> &[u8],
{
	fn expand_template_part_variable(&self, variable: &Variable, output: &mut Vec<u8>) -> Result<(), Error> {
		let value = self.variables.get(variable.name);
		match (&value, &variable.default) {
			(None, None) => {
				if !self.silent_missing_variables {
					return Err(error::NoSuchVariable {
						position: variable.name_start,
						name: variable.name.to_owned(),
					}
					.into());
				}
			},
			(Some(value), _) => {
				output.extend_from_slice((*self.to_bytes)(value));
			},
			(None, Some(default)) => {
				self.expand_template_impl(default, output)?;
			},
		}

		Ok(())
	}

	fn expand_template_part_escaped_char(e: &EscapedCharTemplate, output: &mut Vec<u8>) -> Result<(), Error> {
		output.push(e.name);
		Ok(())
	}

	fn expand_template_part_literal(l: &LiteralTemplate, output: &mut Vec<u8>) -> Result<(), Error> {
		output.extend_from_slice(l.text);
		Ok(())
	}

	fn expand_template_part(&self, tp: &TemplatePart, output: &mut Vec<u8>) -> Result<(), Error> {
		match tp {
			TemplatePart::Literal(l) => Self::expand_template_part_literal(l, output)?,
			TemplatePart::Variable(v) => self.expand_template_part_variable(v, output)?,
			TemplatePart::EscapedChar(e) => Self::expand_template_part_escaped_char(e, output)?,
		}

		Ok(())
	}

	fn expand_template_impl(&self, t: &Vec<TemplatePart>, output: &mut Vec<u8>) -> Result<(), Error> {
		for part in t {
			self.expand_template_part(part, output)?;
		}

		Ok(())
	}

	/// takes a template and variable map to generate output
	fn expand_template_parts_vec(&self, t: &Vec<TemplatePart>, source_size: Option<usize>) -> Result<String, Error> {
		let output = self.evaluate_template_to_bytes(t, source_size)?;
		// SAFETY: Both source and all variable values are valid UTF-8, so substitation result is also valid UTF-8.
		unsafe { Ok(String::from_utf8_unchecked(output)) }
	}

	fn expand_template(&self, t: &Template) -> Result<String, Error> {
		self.expand_template_parts_vec(&t.parts, Some(t.source.len()))
	}

	fn evaluate_template_to_bytes(&self, t: &Vec<TemplatePart>, source_size: Option<usize>) -> Result<Vec<u8>, Error> {
		let source_size = if let Some(source_size) = source_size {
			source_size
		} else {
			0
		};
		let mut output = Vec::with_capacity(source_size + source_size / 10);
		self.expand_template_impl(t, &mut output)?;
		Ok(output)
	}
}

/// provides convenient access to one-by-one step-wise substitution
#[derive(Debug)]
pub struct VariableSubstituter<'a, M, F>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
	F: Fn(&M::Value) -> &[u8],
{
	parse_cfg: &'a TemplateParser,
	expand_cfg: &'a TemplateExpander<'a, M, F>,
}

impl<'a, M, F> VariableSubstituter<'a, M, F>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
	F: Fn(&M::Value) -> &[u8],
{
	/// does one sub-step of substitute.
	pub fn substitute_one(&self, source: &str) -> Result<(usize, String), Error> {
		let next_part = self
			.parse_cfg
			.parse_template_one_step(0, source.as_bytes(), &(0..source.len()))?;

		if let Some(part) = next_part {
			let mut output = Vec::with_capacity(source.len() + source.len() / 10);
			self.expand_cfg.expand_template_part(&part, &mut output)?;
			// SAFETY: Both source and all variable values are valid UTF-8, so substitation result is also valid UTF-8.
			Ok((part.size(), unsafe { String::from_utf8_unchecked(output) }))
		} else {
			Ok((0, "".into()))
		}
	}
}

/// A parsed variable.
#[derive(Debug, PartialEq, Eq)]
struct Variable<'a> {
	/// template part start
	part_start: usize,

	/// The end position of the entire variable in the source.
	part_end: usize,

	/// The name of the variable.
	name: &'a str,

	/// The start position of the name in the source.
	name_start: usize,

	/// The default value of the variable.
	default: Option<Vec<TemplatePart<'a>>>,
}

impl TemplateParser {
	fn check_variable_name_for_validity(&self, finger: usize, name: &str) -> Result<(), Error> {
		if self.allow_variable_name_starting_with_number {
			return Ok(());
		}

		let c = name.as_bytes()[0];
		let starts_with_digit = c.is_ascii_digit();
		if starts_with_digit {
			let all_digits = name
				.as_bytes()
				.iter()
				.map(u8::is_ascii_digit)
				.reduce(|acc, e| acc && e)
				.unwrap();
			if all_digits {
				Ok(())
			} else {
				Err(Error::UnexpectedCharacter(error::UnexpectedCharacter {
					position: finger,
					character: error::CharOrByte::Byte(c),
					expected: error::ExpectedCharacter {
						message: "0..9 not allowed as variable name start!",
					},
				}))
			}
		} else {
			Ok(())
		}
	}

	/// Parse a variable from source at the given position.
	///
	/// The finger must be the position of the dollar sign in the source.
	fn parse_variable<'a>(&self, source: &'a [u8], finger: usize) -> Result<Variable<'a>, Error> {
		if finger == source.len() {
			return Err(error::MissingVariableName {
				position: finger,
				len: 1,
			}
			.into());
		}
		if source[finger + 1] == b'{' {
			self.parse_braced_variable(source, finger)
		} else {
			let name_end = match source[finger + 1..]
				.iter()
				.position(|&c| !c.is_ascii_alphanumeric() && c != b'_')
			{
				Some(0) => {
					return Err(error::MissingVariableName {
						position: finger,
						len: 1,
					}
					.into())
				},
				Some(x) => finger + 1 + x,
				None => source.len(),
			};
			let name = std::str::from_utf8(&source[finger + 1..name_end]).unwrap();
			self.check_variable_name_for_validity(finger, name)?;
			Ok(Variable {
				name,
				name_start: finger + 1,
				default: None,
				part_start: finger,
				part_end: name_end,
			})
		}
	}

	/// Parse a braced variable in the form of "${name[:default]} from source at the given position.
	///
	/// The finger must be the position of the dollar sign in the source.
	fn parse_braced_variable<'a>(&self, source: &'a [u8], finger: usize) -> Result<Variable<'a>, Error> {
		let name_start = finger + 2;
		if name_start >= source.len() {
			return Err(error::MissingVariableName {
				position: finger,
				len: 2,
			}
			.into());
		}

		// Get the first sequence of alphanumeric characters and underscores for the variable name.
		let name_end = match source[name_start..]
			.iter()
			.position(|&c| !c.is_ascii_alphanumeric() && c != b'_')
		{
			Some(0) => {
				return Err(error::MissingVariableName {
					position: finger,
					len: 2,
				}
				.into())
			},
			Some(x) => name_start + x,
			None => source.len(),
		};

		// If the name extends to the end, we're missing a closing brace.
		if name_end == source.len() {
			return Err(error::MissingClosingBrace { position: finger + 1 }.into());
		}

		// If there is a closing brace after the name, there is no default value and we're done.
		if source[name_end] == b'}' {
			let name = std::str::from_utf8(&source[name_start..name_end]).unwrap();
			self.check_variable_name_for_validity(finger, name)?;
			return Ok(Variable {
				name,
				name_start,
				default: None,
				part_start: finger,
				part_end: name_end + 1,
			});

		// If there is something other than a closing brace or colon after the name, it's an error.
		} else if source[name_end] != b':' {
			return Err(error::UnexpectedCharacter {
				position: name_end,
				character: get_maybe_char_at(source, name_end),
				expected: error::ExpectedCharacter {
					message: "a closing brace ('}') or colon (':')",
				},
			}
			.into());
		}

		// If there is no un-escaped closing brace, it's missing.
		let end = finger
			+ find_non_escaped(b'}', &source[finger..]).ok_or(error::MissingClosingBrace { position: finger + 1 })?;

		let default_value = self.parse_template_range(source, &(name_end + 1..end))?;

		let name = std::str::from_utf8(&source[name_start..name_end]).unwrap();
		self.check_variable_name_for_validity(finger, name)?;
		Ok(Variable::<'a> {
			name,
			name_start,
			default: Some(default_value),
			part_start: finger,
			part_end: end + 1,
		})
	}
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
	assert!(
		!head.is_empty(),
		"index out of bounds: data.len() is {} but index is {}",
		data.len(),
		index
	);

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
			return Some(finger + candidate);
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
		}
		.into());
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
		}
		.into()),
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::{assert, check, let_assert};
	use std::collections::BTreeMap;

	#[test]
	fn test_get_maybe_char_at() {
		use error::CharOrByte::{Byte, Char};
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
		assert_eq!(
			Ok("Hello cruel world!"),
			substitute("Hello ${not_name:cruel $name}!", &map).as_deref()
		);
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
	fn test_substitute_silent_missing_variable() {
		let map: BTreeMap<String, String> = BTreeMap::new();

		let source = "Hello ${name}. Hello ${name2:World}!";
		let parser = TemplateParser {
			allow_variable_name_starting_with_number: true,
		};
		let template = parser.parse(source).unwrap();
		let expander = TemplateExpander::new(&map, &from_str_to_bytes).set_silent_missing_variables(true);

		let result = expander.expand_template(&template).unwrap();
		assert_eq!(result, "Hello . Hello World!");
	}

	#[test]
	fn test_dyn_variable_map() {
		let mut variables = BTreeMap::new();
		variables.insert(String::from("aap"), String::from("noot"));
		let variables: &dyn VariableMap<Value = &String> = &variables;

		let_assert!(Ok(expanded) = substitute("one ${aap}", variables));
		assert_eq!(expanded, "one noot");
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
	fn test_substitute_one_step_variable_and_escape_sequence() {
		let mut variables = BTreeMap::new();
		variables.insert(String::from("NAME"), String::from("subst"));
		let subst = VariableSubstituter {
			parse_cfg: &TemplateParser::default(),
			expand_cfg: &TemplateExpander::new(&variables, &from_str_to_bytes),
		};

		let source = r"hello $NAME. Nice\$to meet you $NAME.";
		assert!(subst.substitute_one(source).unwrap() == (6, "hello ".into()));
		assert_eq!(subst.substitute_one(&source[6..]).unwrap(), (5, "subst".into()));
		assert_eq!(subst.substitute_one(&source[6 + 5..]).unwrap(), (6, ". Nice".into()));
		assert_eq!(subst.substitute_one(&source[6 + 5 + 6..]).unwrap(), (2, "$".into()));
		assert_eq!(
			subst.substitute_one(&source[6 + 5 + 6 + 2..]).unwrap(),
			(12, "to meet you ".into())
		);
		assert_eq!(
			subst.substitute_one(&source[6 + 5 + 6 + 2 + 12..]).unwrap(),
			(5, "subst".into())
		);
		assert_eq!(
			subst.substitute_one(&source[6 + 5 + 6 + 2 + 12 + 5..]).unwrap(),
			(1, ".".into())
		);
		assert_eq!(
			subst.substitute_one(&source[6 + 5 + 6 + 2 + 12 + 5 + 1..]).unwrap(),
			(0, "".into())
		);
		assert_eq!(
			subst.substitute_one(&source[6 + 5 + 6 + 2 + 12 + 5 + 1..]).unwrap(),
			(0, "".into())
		);
	}

	#[test]
	fn test_substitute_one_step_invalid_variable_name_due_to_number_at_begin() {
		let mut variables = BTreeMap::new();
		variables.insert(String::from("NAME"), String::from("subst"));
		variables.insert(String::from("1"), String::from("hello"));
		variables.insert(String::from("100"), String::from("world"));
		let subst = VariableSubstituter {
			parse_cfg: &TemplateParser {
				allow_variable_name_starting_with_number: false,
			},
			expand_cfg: &TemplateExpander::new(&variables, &from_str_to_bytes),
		};

		subst.substitute_one("$NAME").unwrap();
		subst.substitute_one("$1NAME").unwrap_err();
		subst.substitute_one("$123NAME").unwrap_err();
		subst.substitute_one("$1").unwrap();
		subst.substitute_one("$100").unwrap();
	}
}
