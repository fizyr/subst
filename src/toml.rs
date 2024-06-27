//! Support for variable substitution in TOML data.

use serde::de::DeserializeOwned;

use crate::VariableMap;

/// Parse a struct from TOML data, after performing variable substitution on string values.
///
/// This function first parses the data into a [`toml::Value`],
/// then performs variable substitution on all string values,
/// and then parses it further into the desired type.
pub fn from_slice<'a, T: DeserializeOwned, M>(data: &[u8], variables: &'a M) -> Result<T, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	from_str(std::str::from_utf8(data)?, variables)
}

/// Parse a struct from TOML data, after performing variable substitution on string values.
///
/// This function first parses the data into a [`toml::Value`],
/// then performs variable substitution on all string values,
/// and then parses it further into the desired type.
pub fn from_str<'a, T: DeserializeOwned, M>(data: &str, variables: &'a M) -> Result<T, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	let mut value: toml::Value = toml::from_str(data)?;
	substitute_string_values(&mut value, variables)?;
	Ok(T::deserialize(value)?)
}

/// Perform variable substitution on string values of a TOML value.
pub fn substitute_string_values<'a, M>(value: &mut toml::Value, variables: &'a M) -> Result<(), crate::Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	visit_string_values(value, |value| {
		*value = crate::substitute(value.as_str(), variables)?;
		Ok(())
	})
}

/// Error for parsing TOML with variable substitution.
#[derive(Debug)]
pub enum Error {
	/// The input contains invalid UTF-8.
	InvalidUtf8(std::str::Utf8Error),

	/// An error occurred while parsing TOML.
	Toml(toml::de::Error),

	/// An error occurred while performing variable substitution.
	Subst(crate::Error),
}

impl From<std::str::Utf8Error> for Error {
	#[inline]
	fn from(other: std::str::Utf8Error) -> Self {
		Self::InvalidUtf8(other)
	}
}

impl From<toml::de::Error> for Error {
	#[inline]
	fn from(other: toml::de::Error) -> Self {
		Self::Toml(other)
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
			Self::InvalidUtf8(e) => std::fmt::Display::fmt(e, f),
			Self::Toml(e) => std::fmt::Display::fmt(e, f),
			Self::Subst(e) => std::fmt::Display::fmt(e, f),
		}
	}
}

/// Recursively apply a function to all string values in a TOML value.
fn visit_string_values<F, E>(value: &mut toml::Value, fun: F) -> Result<(), E>
where
	F: Copy + Fn(&mut String) -> Result<(), E>,
{
	match value {
		toml::Value::Boolean(_) => Ok(()),
		toml::Value::Integer(_) => Ok(()),
		toml::Value::Float(_) => Ok(()),
		toml::Value::Datetime(_) => Ok(()),
		toml::Value::String(val) => fun(val),
		toml::Value::Array(seq) => {
			for value in seq {
				visit_string_values(value, fun)?;
			}
			Ok(())
		},
		toml::Value::Table(map) => {
			for (_key, value) in map.iter_mut() {
				visit_string_values(value, fun)?;
			}
			Ok(())
		},
	}
}

#[cfg(test)]
#[rustfmt::skip]
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
				"bar = \"$bar\"\n",
				"baz = \"$baz/with/stuff\"\n",
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
				"bar = \"aap\"\n",
				"baz = \"noot/with/stuff\"\n",
			),
			&crate::NoSubstitution,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}

	#[test]
	fn test_toml_in_var_is_not_parsed() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap\nbaz = \"mies\"");
		variables.insert("baz", "noot");
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(
			concat!(
				"bar = \"$bar\"\n",
				"baz = \"$baz\"\n",
			),
			&variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap\nbaz = \"mies\"");
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
				"bar = \"$bar\"\n",
				"baz = \"$baz/with/stuff\"\n",
			),
			variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}
}
