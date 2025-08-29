//! Maps and related utilities for variable substitution.

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

mod fallback;
pub use fallback::*;

mod fn_map;
pub use fn_map::*;

mod map_value;
pub use map_value::*;

/// Trait for types that can be used as a variable map.
pub trait VariableMap<'a> {
	/// The type returned by the [`get()`][Self::get] function.
	type Value;

	/// Get a value from the map.
	fn get(&'a self, key: &str) -> Option<Self::Value>;
}

/// Allow using key-value [`slice`]s as [`VariableMap`]s.
///
/// # Performance
///
/// For a few key-value pairs, where the keys and values are small,
/// this is should be reasonably performant.
///
/// However, for many numbers of key-value pairs, or when the keys or values are large,
/// you may get better performance from a [`HashMap`] or [`BTreeMap`].
///
/// # Example
/// ```rust
/// # use subst::VariableMap;
///
/// let contact_info = &[("first_name", "John"), ("last_name", "Doe")];
///
/// assert_eq!(contact_info.get("first_name"), Some(&"John"));
/// assert_eq!(contact_info.get("last_name"), Some(&"Doe"));
/// assert_eq!(contact_info.get("middle_name"), None);
/// ```
impl<'a, K, V> VariableMap<'a> for [(K, V)]
where
	K: Borrow<str>,
	V: 'a,
{
	type Value = &'a V;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.iter().find_map(|(k, v)| (k.borrow() == key).then_some(v))
	}
}

/// Allow using key-value [`arrays`](`array`) as [`VariableMap`]s.
///
/// Delegate to [impl](#impl-VariableMap<'a>-for-[(K,+V)]) of [`VariableMap`] for [`slices`](`slice`).
///
/// # Example
/// ```rust
/// # use subst::VariableMap;
///
/// let contact_info = [("first_name", "John"), ("last_name", "Doe")];
///
/// assert_eq!(contact_info.get("first_name"), Some(&"John"));
/// assert_eq!(contact_info.get("last_name"), Some(&"Doe"));
/// assert_eq!(contact_info.get("middle_name"), None);
/// ```
impl<'a, K, V, const N: usize> VariableMap<'a> for [(K, V); N]
where
	K: Borrow<str>,
	V: 'a,
{
	type Value = &'a V;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		VariableMap::get(self.as_slice(), key)
	}
}

/// Allow using key-value [`Vec`] as [`VariableMap`]s.
///
/// Delegate to [impl](#impl-VariableMap<'a>-for-[(K,+V)]) of [`VariableMap`] for [`slices`](`slice`).
///
/// # Example
/// ```rust
/// # use subst::VariableMap;
///
/// let contact_info = [("first_name", "John"), ("last_name", "Doe")];
///
/// assert_eq!(contact_info.get("first_name"), Some(&"John"));
/// assert_eq!(contact_info.get("last_name"), Some(&"Doe"));
/// assert_eq!(contact_info.get("middle_name"), None);
/// ```
impl<'a, K, V> VariableMap<'a> for Vec<(K, V)>
where
	K: Borrow<str>,
	V: 'a,
{
	type Value = &'a V;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		VariableMap::get(self.as_slice(), key)
	}
}

impl<'a, T> VariableMap<'a> for &'_ T
where
	T: ?Sized + VariableMap<'a>,
{
	type Value = <T as VariableMap<'a>>::Value;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		T::get(self, key)
	}
}

impl<'a, T> VariableMap<'a> for &'_ mut T
where
	T: ?Sized + VariableMap<'a>,
{
	type Value = <T as VariableMap<'a>>::Value;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		T::get(self, key)
	}
}

impl<'a, T> VariableMap<'a> for std::boxed::Box<T>
where
	T: ?Sized + VariableMap<'a>,
{
	type Value = <T as VariableMap<'a>>::Value;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		T::get(self, key)
	}
}

impl<'a, T> VariableMap<'a> for std::rc::Rc<T>
where
	T: ?Sized + VariableMap<'a>,
{
	type Value = <T as VariableMap<'a>>::Value;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		T::get(self, key)
	}
}

impl<'a, T> VariableMap<'a> for std::sync::Arc<T>
where
	T: ?Sized + VariableMap<'a>,
{
	type Value = <T as VariableMap<'a>>::Value;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		T::get(self, key)
	}
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
