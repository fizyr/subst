use std::collections::{BTreeMap, HashMap};

/// Trait for types that can be used as a variable map.
pub trait VariableMap<'a> {
	/// The type returned by the [`get()`][Self::get] function.
	type Value;

	/// Get a value from the map.
	fn get(&'a self, key: &str) -> Option<Self::Value>;
}

/// A map that gives strings from the environment.
#[derive(Debug)]
pub struct Env;

impl<'a> VariableMap<'a> for Env {
	type Value = String;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		std::env::var(key).ok()
	}
}

/// A map that gives byte strings from the environment.
///
/// Only available on Unix platforms.
#[cfg(unix)]
#[derive(Debug)]
pub struct EnvBytes;

#[cfg(unix)]
impl<'a> VariableMap<'a> for EnvBytes {
	type Value = Vec<u8>;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		use std::os::unix::ffi::OsStringExt;
		let value = std::env::var_os(key)?;
		Some(value.into_vec())
	}
}

impl<'a, V: 'a> VariableMap<'a> for BTreeMap<&str, V> {
	type Value = &'a V;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a> VariableMap<'a> for BTreeMap<String, V> {
	type Value = &'a V;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a> VariableMap<'a> for HashMap<&str, V> {
	type Value = &'a V;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a> VariableMap<'a> for HashMap<String, V> {
	type Value = &'a V;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}
