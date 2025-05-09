use super::{EscapedByte, Literal, Part, Template, Variable};
use crate::error::{self, ParseError};

impl Template {
	/// Parse the template from a source slice starting at the given position.
	///
	/// You must pass the entire source slice and an offset,
	/// so that source positions in errors are correct.
	pub fn parse(source: &[u8], start: usize) -> Result<Self, ParseError> {
		let mut parts = Vec::with_capacity(1);
		let mut finger = start;
		while finger < source.len() {
			let next = match memchr::memchr2(b'$', b'\\', &source[finger..]) {
				Some(x) => finger + x,
				None => source.len(),
			};

			// If we found a non-empty string up to the first backslash or dollar,
			// then we have a piece of literal text.
			if next != finger {
				parts.push(Part::Literal(Literal { range: finger..next }));
			}

			// If we hit the end of the string, we're done.
			if next == source.len() {
				break;
			}

			// We found an escape sequence.
			if source[next] == b'\\' {
				let value = unescape_one(source, next)?;
				parts.push(Part::EscapedByte(EscapedByte { value }));
				finger = next + 2;

			// We found a variable substitution.
			} else {
				let (variable, end) = Variable::parse(source, next)?;
				finger = end;
				parts.push(Part::Variable(variable));
			}
		}

		Ok(Self { parts })
	}
}

impl Variable {
	/// Parse a variable from the source.
	///
	/// The finger must be the position of the dollar sign in the source.
	///
	/// Returns the parsed variable and the index of the byte after the variable.
	fn parse(source: &[u8], finger: usize) -> Result<(Self, usize), ParseError> {
		if finger + 1 >= source.len() {
			return Err(error::MissingVariableName {
				position: finger,
				len: 1,
			}
			.into());
		}
		if source[finger + 1] == b'{' {
			Self::parse_braced(source, finger)
		} else if source[finger + 1] == b'*' {
			let name_end = finger + 2;
			let variable = Variable {
				name: finger + 1..name_end,
				default: None,
			};
			Ok((variable, name_end))
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
					.into());
				},
				Some(x) => finger + 1 + x,
				None => source.len(),
			};
			let variable = Variable {
				name: finger + 1..name_end,
				default: None,
			};
			Ok((variable, name_end))
		}
	}

	/// Parse a braced variable in the form of "${name[:default]} from source at the given position.
	///
	/// The finger must be the position of the dollar sign in the source.
	///
	/// Returns the parsed variable and the index of the byte after the variable.
	fn parse_braced(source: &[u8], finger: usize) -> Result<(Self, usize), ParseError> {
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
			.position(|&c| !c.is_ascii_alphanumeric() && c != b'_' && c != b'*')
		{
			Some(0) => {
				return Err(error::MissingVariableName {
					position: finger,
					len: 2,
				}
				.into());
			},
			Some(x) => name_start + x,
			None => source.len(),
		};

		// If the name extends to the end, we're missing a closing brace.
		if name_end == source.len() {
			return Err(error::MissingClosingBrace { position: finger + 1 }.into());
		}

		if name_end - name_start > 1 && source[name_start..name_end].contains(&b'*') {
			return Err(error::MissingVariableName {
				position: finger,
				len: name_end - name_start,
			}
			.into());
		}

		// If there is a closing brace after the name, there is no default value and we're done.
		if source[name_end] == b'}' {
			let variable = Variable {
				name: name_start..name_end,
				default: None,
			};
			return Ok((variable, name_end + 1));

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

		// If there is no matching un-escaped closing brace, it's missing.
		let end = finger
			+ find_closing_brace(&source[finger..]).ok_or(error::MissingClosingBrace { position: finger + 1 })?;

		let variable = Variable {
			name: name_start..name_end,
			default: Some(Template::parse(&source[..end], name_end + 1)?),
		};
		Ok((variable, end + 1))
	}
}

/// Unescape a single escape sequence in source at the given position.
///
/// The `position` must point to the backslash character in the source text.
///
/// Only valid escape sequences ('\$' '\{' '\}' and '\:') are accepted.
/// Invalid escape sequences cause an error to be returned.
fn unescape_one(source: &[u8], position: usize) -> Result<u8, ParseError> {
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
		index,
	);

	let head = valid_utf8_prefix(head);
	if let Some(c) = head.chars().next() {
		error::CharOrByte::Char(c)
	} else {
		error::CharOrByte::Byte(data[index])
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

/// Find the closing brace of recursive substitutions.
fn find_closing_brace(haystack: &[u8]) -> Option<usize> {
	let mut finger = 0;
	// We need to count the first opening brace
	let mut nested = 0;
	while finger < haystack.len() {
		let next = memchr::memchr3(b'\\', b'{', b'}', &haystack[finger..])?;
		match haystack[finger + next] {
			b'\\' => {
				// If the backslash is the last character, there is no matching closing brace.
				if next + 1 == haystack.len() {
					return None;
				}

				// NOTE: We don't report errors for invalid escape sequences here.
				// They will be reported later by the parsing function,
				// unless another error occurs first.
				finger += next + 2;
			},
			b'{' => {
				// If the opening brace is the last character, there is no matching closing brace.
				if next == haystack.len() - 1 {
					return None;
				}

				// Increase the nesting level and continue.
				nested += 1;
				finger += next + 1;
			},
			b'}' => {
				// Decrease the nesting level and check if we're done.
				nested -= 1;
				if nested == 0 {
					return Some(finger + next);
				}
				finger += next + 1;
			},
			_ => unreachable!(),
		}
	}
	None
}

#[cfg(test)]
#[rustfmt::skip]
mod test {
	use super::*;
	use assert2::{assert, check};

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
	fn test_find_closing_brace() {
		check!(find_closing_brace(b"${foo}") == Some(5));
		check!(find_closing_brace(b"{\\{}foo") == Some(3));
		check!(find_closing_brace(b"{{}}foo $bar") == Some(3));
		check!(find_closing_brace(b"foo{\\}}bar") == Some(6));
	}
}
