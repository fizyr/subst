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
	fn from(other: InvalidEscapeSequence) -> Self {
		Self::InvalidEscapeSequence(other)
	}
}

impl From<MissingVariableName> for Error {
	fn from(other: MissingVariableName) -> Self {
		Self::MissingVariableName(other)
	}
}

impl From<UnexpectedCharacter> for Error {
	fn from(other: UnexpectedCharacter) -> Self {
		Self::UnexpectedCharacter(other)
	}
}

impl From<MissingClosingBrace> for Error {
	fn from(other: MissingClosingBrace) -> Self {
		Self::MissingClosingBrace(other)
	}
}

impl From<NoSuchVariable> for Error {
	fn from(other: NoSuchVariable) -> Self {
		Self::NoSuchVariable(other)
	}
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
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

/// The input string contains an invalid escape sequence.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct InvalidEscapeSequence {
	/// The byte offset within the input where the error occurs.
	///
	/// This points to the associated backslash character in the source text.
	pub position: usize,

	/// The byte value of the invalid escape sequence.
	pub character: Option<u8>,
}

impl std::error::Error for InvalidEscapeSequence {}

impl std::fmt::Display for InvalidEscapeSequence {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if let Some(c) = self.character {
			write!(f, "Invalid escape sequence: \\{}", char::from(c))
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

	/// The byte value of the unexpected character.
	///
	/// For multi-byte UTF-8 sequences, this only gives the value of the start byte.
	/// You can use the `position` to get the full UTF-8 sequence from the original input string.
	pub character: u8,

	/// A human readable message about what was expected instead.
	pub expected: ExpectedCharacter,
}

impl std::error::Error for UnexpectedCharacter {}

impl std::fmt::Display for UnexpectedCharacter {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Unexpected character: {:?}, expected {}", char::from(self.character), self.expected.message())
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
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "No such variable: ${}", self.name)
	}
}

impl Error {
	/// Get the range in the source text that contains the error.
	pub fn source_range(&self) -> std::ops::Range<usize> {
		let (start, len) = match &self {
			Self::InvalidEscapeSequence(e) => {
				if e.character.is_some() {
					(e.position, 2)
				} else {
					(e.position, 1)
				}
			},
			Self::MissingVariableName(e) => {
				(e.position, e.len)
			},
			Self::UnexpectedCharacter(e) => {
				(e.position, 1)
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
	pub fn source_line<'a>(&self, source: &'a [u8]) -> &'a [u8] {
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
	pub fn write_source_highlighting(&self, f: &mut impl std::fmt::Write, source: &[u8]) -> std::fmt::Result {
		use unicode_width::UnicodeWidthStr;

		let range = self.source_range();
		let line = self.source_line(source);
		let line = match std::str::from_utf8(line) {
			Ok(line) => line,
			Err(_) => return Err(std::fmt::Error),
		};
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
	pub fn source_highlighting(&self, source: &[u8]) -> String {
		let mut output = String::new();
		self.write_source_highlighting(&mut output, source).unwrap();
		output
	}
}

fn line_start(source: &[u8], position: usize) -> usize {
	match source[..position].iter().rposition(|&c| c == b'\n' || c == b'\r') {
		Some(line_end) => line_end + 1,
		None => 0,
	}
}

fn line_end(source: &[u8], position: usize) -> usize {
	match source[position..].iter().position(|&c| c == b'\n' || c == b'\r') {
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
