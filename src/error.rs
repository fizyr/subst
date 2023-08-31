//! Module containing error details.

/// An error that can occur during variable substitution.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Error {
	/// The input string contains an invalid escape sequence.
	InvalidEscapeSequence(InvalidEscapeSequence),

	/// The input string contains a variable placeholder without a variable name (`"${}"`).
	MissingVariableName(MissingVariableName),

	/// The input string contains an unexpected character.
	UnexpectedCharacter(UnexpectedCharacter),

	/// The input string contains an unclosed variable placeholder.
	MissingClosingBrace(MissingClosingBrace),

	/// The input string contains a placeholder for a variable that is not in the variable map.
	NoSuchVariable(NoSuchVariable),
}

impl From<InvalidEscapeSequence> for Error {
	#[inline]
	fn from(other: InvalidEscapeSequence) -> Self {
		Self::InvalidEscapeSequence(other)
	}
}

impl From<MissingVariableName> for Error {
	#[inline]
	fn from(other: MissingVariableName) -> Self {
		Self::MissingVariableName(other)
	}
}

impl From<UnexpectedCharacter> for Error {
	#[inline]
	fn from(other: UnexpectedCharacter) -> Self {
		Self::UnexpectedCharacter(other)
	}
}

impl From<MissingClosingBrace> for Error {
	#[inline]
	fn from(other: MissingClosingBrace) -> Self {
		Self::MissingClosingBrace(other)
	}
}

impl From<NoSuchVariable> for Error {
	#[inline]
	fn from(other: NoSuchVariable) -> Self {
		Self::NoSuchVariable(other)
	}
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidEscapeSequence(e) => e.fmt(f),
			Self::MissingVariableName(e) => e.fmt(f),
			Self::UnexpectedCharacter(e) => e.fmt(f),
			Self::MissingClosingBrace(e) => e.fmt(f),
			Self::NoSuchVariable(e) => e.fmt(f),
		}
	}
}

/// A character or byte from the input.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CharOrByte {
	/// A unicode character.
	Char(char),

	/// A byte value.
	Byte(u8),
}

impl CharOrByte {
	/// Get the byte length of the character in the source.
	///
	/// For [`Self::Char`], this returns the UTF-8 lengths of the character.
	/// For [`Self::Byte`], this is always returns 1.
	#[inline]
	pub fn source_len(&self) -> usize {
		match self {
			Self::Char(c) => c.len_utf8(),
			Self::Byte(_) => 1,
		}
	}

	/// Return a printable version of `self` for error messages.
	///
	/// The returned value implements `Display` and is suitable for inclusion in error messages.
	pub fn quoted_printable(&self) -> impl std::fmt::Display {
		#[derive(Copy, Clone, Debug)]
		struct QuotedPrintable {
			inner: CharOrByte,
		}

		impl std::fmt::Display for QuotedPrintable {
			#[inline]
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self.inner {
					CharOrByte::Char(value) => write!(f, "{value:?}"),
					CharOrByte::Byte(value) => {
						if value.is_ascii() {
							write!(f, "{:?}", char::from(value))
						} else {
							write!(f, "'\\x{value:02X}'")
						}
					},
				}
			}
		}

		QuotedPrintable { inner: *self }
	}
}

impl std::fmt::Display for CharOrByte {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match *self {
			Self::Char(value) => write!(f, "{value}"),
			Self::Byte(value) => {
				if value.is_ascii() {
					write!(f, "{}", char::from(value))
				} else {
					write!(f, "0x{value:02X}")
				}
			},
		}
	}
}

/// The input string contains an invalid escape sequence.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct InvalidEscapeSequence {
	/// The byte offset within the input where the error occurs.
	///
	/// This points to the associated backslash character in the source text.
	pub position: usize,

	/// The character value of the invalid escape sequence.
	///
	/// If the unexpected character is not a valid UTF-8 sequence,
	/// this will simply hold the value of the first byte after the backslash character.
	///
	/// If a backslash character occurs at the end of the input, this field is set to `None`.
	pub character: Option<CharOrByte>,
}

impl std::error::Error for InvalidEscapeSequence {}

impl std::fmt::Display for InvalidEscapeSequence {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if let Some(c) = self.character {
			write!(f, "Invalid escape sequence: \\{}", c)
		} else {
			write!(f, "Invalid escape sequence: missing escape character")
		}
	}
}

/// The input string contains a variable placeholder without a variable name (`"${}"`).
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct MissingVariableName {
	/// The byte offset within the input where the error occurs.
	///
	/// This points to the `$` sign with a missing variable name in the input text.
	pub position: usize,

	/// The length of the variable placeholder in bytes.
	pub len: usize,
}

impl std::error::Error for MissingVariableName {}

impl std::fmt::Display for MissingVariableName {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Missing variable name")
	}
}

/// The input string contains an unexpected character.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct UnexpectedCharacter {
	/// The byte offset within the input where the error occurs.
	///
	/// This points to the unexpected character in the input text.
	pub position: usize,

	/// The unexpected character.
	///
	/// If the unexpected character is not a valid UTF-8 sequence,
	/// this will simply hold the value of the unexpected byte.
	pub character: CharOrByte,

	/// A human readable message about what was expected instead.
	pub expected: ExpectedCharacter,
}

impl std::error::Error for UnexpectedCharacter {}

impl std::fmt::Display for UnexpectedCharacter {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Unexpected character: {}, expected {}", self.character.quoted_printable(), self.expected.message())
	}
}

/// A struct to describe what was expected instead of the unexpected character.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct ExpectedCharacter {
	/// A human readable message to describe what is expected.
	pub(crate) message: &'static str,
}

impl ExpectedCharacter {
	/// Get a human readable message to describe what was expected.
	#[inline]
	pub fn message(&self) -> &str {
		self.message
	}
}

/// The input string contains an unclosed variable placeholder.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct MissingClosingBrace {
	/// The byte offset within the input where the error occurs.
	///
	/// This points to the `{` character that is missing a closing brace.
	pub position: usize,
}

impl std::error::Error for MissingClosingBrace {}

impl std::fmt::Display for MissingClosingBrace {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Missing closing brace")
	}
}

/// The input string contains a placeholder for a variable that is not in the variable map.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct NoSuchVariable {
	/// The byte offset within the input where the error occurs.
	///
	/// This points to the first character of the name in the input text.
	pub position: usize,

	/// The name of the variable.
	pub name: String,
}

impl std::error::Error for NoSuchVariable {}

impl std::fmt::Display for NoSuchVariable {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "No such variable: ${}", self.name)
	}
}

impl Error {
	/// Get the range in the source text that contains the error.
	#[inline]
	pub fn source_range(&self) -> std::ops::Range<usize> {
		let (start, len) = match &self {
			Self::InvalidEscapeSequence(e) => {
				let char_len = e.character.map(|x| x.source_len()).unwrap_or(0);
				(e.position, 1 + char_len)
			},
			Self::MissingVariableName(e) => {
				(e.position, e.len)
			},
			Self::UnexpectedCharacter(e) => {
				(e.position, e.character.source_len())
			},
			Self::MissingClosingBrace(e) => {
				(e.position, 1)
			},
			Self::NoSuchVariable(e) => {
				(e.position, e.name.len())
			},
		};
		std::ops::Range {
			start,
			end: start + len,
		}
	}

	/// Get the line of source that contains the error.
	///
	/// # Panics
	/// May panic if the source text is not the original source that contains the error.
	#[inline]
	pub fn source_line<'a>(&self, source: &'a str) -> &'a str {
		let position = self.source_range().start;
		let start = line_start(source, position);
		let end = line_end(source, position);
		&source[start..end]
	}

	/// Write source highlighting for the error location.
	///
	/// The highlighting ends with a newline.
	///
	/// Note: this function doesn't print anything if the source line exceeds 60 characters in width.
	/// For more control over this behaviour, consider using [`Self::source_range()`] and [`Self::source_line()`] instead.
	#[inline]
	pub fn write_source_highlighting(&self, f: &mut impl std::fmt::Write, source: &str) -> std::fmt::Result {
		use unicode_width::UnicodeWidthStr;

		let range = self.source_range();
		let line = self.source_line(source);
		if line.width() > 60 {
			return Ok(())
		}
		write!(f, "  {}\n  ", line)?;
		write_underline(f, line, range)?;
		writeln!(f)
	}

	/// Get source highlighting for the error location as a string.
	///
	/// The highlighting ends with a newline.
	///
	/// Note: this function returns an empty string if the source line exceeds 60 characters in width.
	#[inline]
	pub fn source_highlighting(&self, source: &str) -> String {
		let mut output = String::new();
		self.write_source_highlighting(&mut output, source).unwrap();
		output
	}
}

fn line_start(source: &str, position: usize) -> usize {
	match source[..position].rfind(|c| c == '\n' || c == '\r') {
		Some(line_end) => line_end + 1,
		None => 0,
	}
}

fn line_end(source: &str, position: usize) -> usize {
	match source[position..].find(|c| c == '\n' || c == '\r') {
		Some(line_end) => position + line_end,
		None => source.len()
	}
}

fn write_underline(f: &mut impl std::fmt::Write, line: &str, range: std::ops::Range<usize>) -> std::fmt::Result {
	use unicode_width::UnicodeWidthStr;
	let spaces = line[..range.start].width();
	let carets = line[range].width();
	write!(f, "{}", " ".repeat(spaces))?;
	write!(f, "{}", "^".repeat(carets))?;
	Ok(())
}

#[cfg(test)]
mod test {
	use assert2::check;

	#[test]
	fn test_char_or_byte_quoated_printable() {
		use super::CharOrByte::{Byte, Char};
		check!(Byte(0x81).quoted_printable().to_string() == r"'\x81'");
		check!(Byte(0x79).quoted_printable().to_string() == r"'y'");
		check!(Char('\x79').quoted_printable().to_string() == r"'y'");

		check!(Byte(0).quoted_printable().to_string() == r"'\0'");
		check!(Char('\0').quoted_printable().to_string() == r"'\0'");
	}
}
