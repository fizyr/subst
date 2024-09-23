mod expand;
mod parse;

pub use expand::Expand;

/// Raw template that doesn't know track the original source.
///
/// Internally, this keeps a bunch of offsets into the original source.
#[derive(Clone)]
pub struct Template {
	/// The individual parts that make up the template.
	parts: Vec<Part>,
}

/// One piece of a parsed template.
#[derive(Clone)]
pub enum Part {
	/// A literal string to be used verbatim from the original source.
	Literal(Literal),

	/// An escaped byte.
	EscapedByte(EscapedByte),

	/// A variable to be substituted at expansion time.
	Variable(Variable),
}

/// A literal string to be used verbatim from the original source.
#[derive(Clone)]
pub struct Literal {
	/// The range of the literal in the original source.
	///
	/// Will be copied verbatim to the output at expansion time.
	///
	/// The literal can not contain any escaped characters or variables.
	range: std::ops::Range<usize>,
}

/// An escaped byte.
#[derive(Clone)]
pub struct EscapedByte {
	/// The escaped byte.
	///
	/// Will be copied to the output at expansion time.
	value: u8,
}

/// A variable to be substituted at expansion time.
#[derive(Clone)]
pub struct Variable {
	/// The range in the source defining the name of the variable.
	///
	/// Used for look-up in the variable map at expansion time.
	name: std::ops::Range<usize>,

	/// Default value for the variable.
	///
	/// Will be used if the variable does not appear in the variable map at expansion time.
	default: Option<Template>,
}
