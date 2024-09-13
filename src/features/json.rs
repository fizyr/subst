//! Support for variable substitution in JSON data.

use serde::de::DeserializeOwned;

use crate::VariableMap;

/// Parse a struct from JSON data, after performing variable substitution on string values.
///
/// This function first parses the data into a [`serde_json::Value`],
/// then performs variable substitution on all string values,
/// and then parses it further into the desired type.
pub fn from_slice<'a, T: DeserializeOwned, M>(data: &[u8], variables: &'a M) -> Result<T, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	let mut value: serde_json::Value = serde_json::from_slice(data)?;
	substitute_string_values(&mut value, variables)?;
	Ok(T::deserialize(value)?)
}

/// Parse a struct from JSON data, after performing variable substitution on string values.
///
/// This function first parses the data into a [`serde_json::Value`],
/// then performs variable substitution on all string values,
/// and then parses it further into the desired type.
pub fn from_str<'a, T: DeserializeOwned, M>(data: &str, variables: &'a M) -> Result<T, Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	let mut value: serde_json::Value = serde_json::from_str(data)?;
	substitute_string_values(&mut value, variables)?;
	Ok(T::deserialize(value)?)
}

/// Perform variable substitution on string values of a JSON value.
pub fn substitute_string_values<'a, M>(value: &mut serde_json::Value, variables: &'a M) -> Result<(), crate::Error>
where
	M: VariableMap<'a> + ?Sized,
	M::Value: AsRef<str>,
{
	visit_string_values(value, |value| {
		*value = crate::substitute(value.as_str(), variables)?;
		Ok(())
	})
}

/// Error for parsing JSON with variable substitution.
#[derive(Debug)]
pub enum Error {
	/// An error occurred while parsing JSON.
	Json(serde_json::Error),

	/// An error occurred while performing variable substitution.
	Subst(crate::Error),
}

impl From<serde_json::Error> for Error {
	#[inline]
	fn from(other: serde_json::Error) -> Self {
		Self::Json(other)
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
			Self::Json(e) => std::fmt::Display::fmt(e, f),
			Self::Subst(e) => std::fmt::Display::fmt(e, f),
		}
	}
}

/// Recursively apply a function to all string values in a JSON value.
fn visit_string_values<F, E>(value: &mut serde_json::Value, fun: F) -> Result<(), E>
where
	F: Copy + Fn(&mut String) -> Result<(), E>,
{
	match value {
		serde_json::Value::Null => Ok(()),
		serde_json::Value::Bool(_) => Ok(()),
		serde_json::Value::Number(_) => Ok(()),
		serde_json::Value::String(val) => fun(val),
		serde_json::Value::Array(seq) => {
			for value in seq {
				visit_string_values(value, fun)?;
			}
			Ok(())
		},
		serde_json::Value::Object(map) => {
			for value in map.values_mut() {
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
		let_assert!(Ok(parsed) = from_str(r#"
			{
				"bar": "$bar",
				"baz": "$baz/with/stuff"
			}"#,
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
		let_assert!(Ok(parsed) = from_str(r#"
			{
				"bar": "aap",
				"baz": "noot/with/stuff"
			}"#,
			&crate::NoSubstitution,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}

	#[test]
	fn test_json_in_var_is_not_parsed() {
		#[derive(Debug, serde::Deserialize)]
		struct Struct {
			bar: String,
			baz: String,
		}

		let mut variables = HashMap::new();
		variables.insert("bar", "aap\nbaz = \"mies\"");
		variables.insert("baz", "noot");
		#[rustfmt::skip]
		let_assert!(Ok(parsed) = from_str(r#"
			{
				"bar": "$bar",
				"baz": "$baz"
			}"#,
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
		let_assert!(Ok(parsed) = from_str(r#"
			{
				"bar": "$bar",
				"baz": "$baz/with/stuff"
			}"#,
			variables,
		));

		let parsed: Struct = parsed;
		assert!(parsed.bar == "aap");
		assert!(parsed.baz == "noot/with/stuff");
	}
}
