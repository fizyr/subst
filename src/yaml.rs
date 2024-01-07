//! Support for variable substitution in YAML data.

use serde::de::DeserializeOwned;

use crate::VariableMap;

/// Parse a struct from YAML data, after perfoming variable substitution on string values.
///
/// This function first parses the data into a [`serde_yaml::Value`],
/// then performs variable substitution on all string values,
/// and then parses it further into the desired type.
pub fn from_slice<'a, T: DeserializeOwned, M>(data: &[u8], variables: &'a M) -> Result<T, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	let mut value: serde_yaml::Value = serde_yaml::from_slice(data)?;
	substitute_string_values(&mut value, variables)?;
	Ok(serde_yaml::from_value(value)?)
}

/// Parse a struct from YAML data, after perfoming variable substitution on string values.
///
/// This function first parses the data into a [`serde_yaml::Value`],
/// then performs variable substitution on all string values,
/// and then parses it further into the desired type.
pub fn from_str<'a, T: DeserializeOwned, M>(data: &str, variables: &'a M) -> Result<T, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	let mut value: serde_yaml::Value = serde_yaml::from_str(data)?;
	substitute_string_values(&mut value, variables)?;
	Ok(serde_yaml::from_value(value)?)
}

/// Perform variable substitution on string values of a YAML value.
pub fn substitute_string_values<'a, M>(value: &mut serde_yaml::Value, variables: &'a M) -> Result<(), crate::Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	visit_string_values(value, |value| {
		*value = crate::substitute(value.as_str(), variables)?;
		Ok(())
	})
}

/// Error for parsing YAML with variable substitution.
#[derive(Debug)]
pub enum Error {
	/// An error occured while parsing YAML.
	Yaml(serde_yaml::Error),

	/// An error occured while performing variable substitution.
	Subst(crate::Error),
}

impl From<serde_yaml::Error> for Error {
	#[inline]
	fn from(other: serde_yaml::Error) -> Self {
		Self::Yaml(other)
	}
}

impl From<crate::Error> for Error {
	#[inline]
	fn from(other: crate::Error) -> Self {
		Self::Subst(other)
	}
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::Yaml(e) => std::fmt::Display::fmt(e, f),
			Error::Subst(e) => std::fmt::Display::fmt(e, f),
		}
	}
}

/// Recursively apply a function to all string values in a YAML value.
fn visit_string_values<F, E>(value: &mut serde_yaml::Value, fun: F) -> Result<(), E>
where
	F: Copy + Fn(&mut String) -> Result<(), E>,
{
	match value {
		serde_yaml::Value::Null => Ok(()),
		serde_yaml::Value::Bool(_) => Ok(()),
		serde_yaml::Value::Number(_) => Ok(()),
		serde_yaml::Value::String(val) => fun(val),
		serde_yaml::Value::Tagged(tagged) => visit_string_values(&mut tagged.value, fun),
		serde_yaml::Value::Sequence(seq) => {
			for value in seq {
				visit_string_values(value, fun)?;
			}
			Ok(())
		},
		serde_yaml::Value::Mapping(map) => {
			for (_key, value) in map.iter_mut() {
				visit_string_values(value, fun)?;
			}
			Ok(())
		},
	}
}

#[cfg(test)]
mod test {
	use std::collections::HashMap;

	use super::*;
	use assert2::{assert, let_assert};

	#[test]
	fn test_from_str() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap");
		variables.insert("baz", "noot");
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(
			concat!(
				"bar: $bar\n",
				"baz: $baz/with/stuff\n",
			),
			&variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}

	#[test]
	fn test_from_str_no_substitution() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap");
		variables.insert("baz", "noot");
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(
			concat!(
				"bar: aap\n",
				"baz: noot/with/stuff\n",
			),
			&crate::NoSubstitution,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}

	#[test]
	fn test_yaml_in_var_is_not_parsed() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap\nbaz: mies");
		variables.insert("baz", "noot");
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(
			concat!(
				"bar: $bar\n",
				"baz: $baz\n",
			),
			&variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap\nbaz: mies");
		assert!(parsed.baz == "noot");
	}

	#[test]
	fn test_tagged_values_are_substituted() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap\nbaz: mies");
		variables.insert("baz", "noot");
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(
			concat!(
				"bar: !!string $bar\n",
				"baz: $baz\n",
			),
			&variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap\nbaz: mies");
		assert!(parsed.baz == "noot");
	}

	#[test]
	fn test_dyn_variable_map() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap");
		variables.insert("baz", "noot");
		let variables: &dyn VariableMap<Value = &&str> = &variables;
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(
			concat!(
				"bar: $bar\n",
				"baz: $baz/with/stuff\n",
			),
			variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}
}
