use super::{Part, Template, Variable};
use crate::error::{self, ExpandError};
use crate::VariableMap;

/// Common `expand` prototype, e.g., for variables and templates
pub trait Expand {
	/// Expand into the output vector.
	fn expand<'a, M, F>(
		&self,
		output: &mut Vec<u8>,
		source: &[u8],
		variables: &'a M,
		to_bytes: &F,
	) -> Result<(), ExpandError>
	where
		M: VariableMap<'a> + ?Sized,
		F: Fn(&M::Value) -> &[u8];
}

impl Expand for Template {
	/// Expand the template into the output vector.
	fn expand<'a, M, F>(
		&self,
		output: &mut Vec<u8>,
		source: &[u8],
		variables: &'a M,
		to_bytes: &F,
	) -> Result<(), ExpandError>
	where
		M: VariableMap<'a> + ?Sized,
		F: Fn(&M::Value) -> &[u8],
	{
		// Expand all parts one by one.
		for part in &self.parts {
			match part {
				Part::Literal(x) => output.extend_from_slice(&source[x.range.clone()]),
				Part::EscapedByte(x) => output.push(x.value),
				Part::Variable(x) => x.expand(output, source, variables, to_bytes)?,
			}
		}
		Ok(())
	}
}

impl Expand for Variable {
	/// Expand the variable into the output vector.
	fn expand<'a, M, F>(
		&self,
		output: &mut Vec<u8>,
		source: &[u8],
		variables: &'a M,
		to_bytes: &F,
	) -> Result<(), ExpandError>
	where
		M: VariableMap<'a> + ?Sized,
		F: Fn(&M::Value) -> &[u8],
	{
		// Names were already checked to match a restricted set of valid characters, so they are guaranteed to be valid UTF-8.
		let name = std::str::from_utf8(&source[self.name.clone()]).unwrap();

		// If the variable appears in the map, use the value from the map.
		if let Some(value) = variables.get(name) {
			output.extend_from_slice(to_bytes(&value));
			Ok(())
		// Otherwise, use the default value, if given in the template.
		} else if let Some(default) = &self.default {
			default.expand(output, source, variables, to_bytes)
		// Else, raise an error.
		} else {
			Err(ExpandError::NoSuchVariable(error::NoSuchVariable {
				position: self.name.start,
				name: name.to_owned(),
			}))
		}
	}
}
