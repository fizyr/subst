use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

/// Trait for types that can be used as a variable map.
pub trait VariableMap<'a> {
	/// The type returned by the [`get()`][Self::get] function.
	type Value;

	/// Get a value from the map.
	fn get(&'a self, key: &str) -> Option<Self::Value>;
}

/// A "map" that never returns any values.
#[derive(Debug)]
pub struct NoSubstitution;

impl<'a> VariableMap<'a> for NoSubstitution {
	type Value = NeverValue;

	#[inline]
	fn get(&'a self, _key: &str) -> Option<Self::Value> {
		None
	}
}

/// Value returned by the [`NoSubstitution`] map.
#[derive(Debug)]
pub enum NeverValue {}

impl<T: ?Sized> AsRef<T> for NeverValue {
	#[inline]
	fn as_ref(&self) -> &T {
		match *self {}
	}
}

/// A map that gives strings from the environment.
#[derive(Debug)]
pub struct Env;

impl<'a> VariableMap<'a> for Env {
	type Value = String;

	#[inline]
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

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		use std::os::unix::ffi::OsStringExt;
		let value = std::env::var_os(key)?;
		Some(value.into_vec())
	}
}

impl<'a, V: 'a> VariableMap<'a> for BTreeMap<&str, V> {
	type Value = &'a V;

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a> VariableMap<'a> for BTreeMap<String, V> {
	type Value = &'a V;

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a, S: BuildHasher> VariableMap<'a> for HashMap<&str, V, S> {
	type Value = &'a V;

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a, S: BuildHasher> VariableMap<'a> for HashMap<String, V, S> {
	type Value = &'a V;

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}
