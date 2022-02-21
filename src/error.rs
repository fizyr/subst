/// An error that can occur during variable substitution.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Error {
	/// The private error details.
	inner: ErrorInner,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub(crate) enum ErrorInner {
	InvalidEscapeSequence {
		position: usize,
		character: Option<u8>,
	},
	MissingVariableName {
		position: usize,
		len: usize,
	},
	UnexpectedCharacter {
		position: usize,
		character: u8,
		expected: &'static str,
	},
	MissingClosingBrace {
		position: usize,
	},
	NoSuchVariable {
		position: usize,
		name: String,
	},
}

impl From<ErrorInner> for Error {
	fn from(inner: ErrorInner) -> Self {
		Self { inner }
	}
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match &self.inner {
			ErrorInner::InvalidEscapeSequence { position: _, character } => {
				if let Some(c) = character {
					write!(f, "Invalid escape sequence: \\{}", char::from(*c))
				} else {
					write!(f, "Invalid escape sequence: missing escape character")
				}
			}
			ErrorInner::MissingVariableName { position: _, len: _ } => {
				write!(f, "Missing variable name")
			},
			ErrorInner::UnexpectedCharacter { position: _, character, expected } => {
				write!(f, "Unexpected character: {:?}, expected {}", char::from(*character), expected)
			},
			ErrorInner::MissingClosingBrace { position: _ } => {
				write!(f, "Missing closing brace")
			},
			ErrorInner::NoSuchVariable { position: _, name } => {
				write!(f, "No such variable: ${name}")
			},
		}
	}
}

impl Error {
	/// Write source highlighting for the error location.
	///
	/// The highlighting ends with a newline.
	pub fn write_source_highlighting(&self, f: &mut impl std::fmt::Write, source: &[u8]) -> std::fmt::Result {
		let (line, start, len) = match &self.inner {
			ErrorInner::InvalidEscapeSequence { position, character } => {
				let line = get_line(source, *position);
				if character.is_some() {
					(line, *position, 2)
				} else {
					(line, *position, 1)
				}
			},
			ErrorInner::MissingVariableName { position, len } => {
				(get_line(source, *position), *position, *len)
			},
			ErrorInner::UnexpectedCharacter { position, character: _, expected: _ } => {
				(get_line(source, *position), *position, 1)
			},
			ErrorInner::MissingClosingBrace { position } => {
				(get_line(source, *position), *position, 1)
			},
			ErrorInner::NoSuchVariable { position, name } => {
				(get_line(source, *position), *position, name.len())
			},
		};
		let line = match line {
			Ok(line) if line.len() <= 60 => line,
			_ => return Ok(()),
		};
		write!(f, "  {}\n  ", line)?;
		write_underline(f, &line, start, start + len)?;
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

fn get_line(source: &[u8], position: usize) -> Result<String, std::str::Utf8Error> {
	let start = line_start(source, position);
	let end = line_end(source, position);
	let line = std::str::from_utf8(&source[start..end])?.trim();
	Ok(line.replace('\t', "    "))
}

fn write_underline(f: &mut impl std::fmt::Write, line: &str, start: usize, end: usize) -> std::fmt::Result {
	use unicode_width::UnicodeWidthStr;
	let spaces = line[..start].width();
	let carets = line[start..end].width();
	write!(f, "{}", " ".repeat(spaces))?;
	write!(f, "{}", "^".repeat(carets))?;
	Ok(())
}
