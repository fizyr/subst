use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

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

/// [`VariableMap`] produced by [`fallback()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FallbackSubstitution<Base, Fallback> {
	base: Base,
	fallback: Fallback,
}

impl<'a, Value, Base, Fallback> VariableMap<'a> for FallbackSubstitution<Base, Fallback>
where
	Base: VariableMap<'a, Value = Value>,
	Fallback: VariableMap<'a, Value = Value>,
{
	type Value = Value;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.base.get(key).or_else(|| self.fallback.get(key))
	}
}

/// Creates a [`VariableMap`] that will first try to find values in `base`, and then attempt to
/// find values in `fallback`.
///
///
/// # Example
/// ```rust
/// # use subst::{fallback, VariableMap};
///
/// let contact_info = [("first_name", "John"), ("last_name", "Doe")];
/// let with_fallback = fallback(contact_info, [("middle_name", "<unknown>")]);
///
/// assert_eq!(with_fallback.get("first_name"), Some(&"John"));
/// assert_eq!(with_fallback.get("last_name"), Some(&"Doe"));
/// assert_eq!(with_fallback.get("middle_name"), Some(&"<unknown>"));
/// ```
pub const fn fallback<Base, Fallback>(base: Base, fallback: Fallback) -> FallbackSubstitution<Base, Fallback> {
	FallbackSubstitution { base, fallback }
}

/// [`VariableMap`] produced by [`from_fn()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FnSubstitution<F> {
	func: F,
}

impl<'a, F, V> VariableMap<'a> for FnSubstitution<F>
where
	F: 'a + Fn(&str) -> Option<V>,
{
	type Value = V;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		(self.func)(key)
	}
}

/// Creates a [`VariableMap`] that will first try to find values in `base`, and then attempt to
/// find values in `fallback`.
///
///
/// # Example
/// ```rust
/// # use subst::{from_fn, VariableMap};
///
/// let contact_info = from_fn(|key| match key {
///     "first_name" => Some("John"),
///     "last_name" => Some("Doe"),
///     _ => None,
/// });
///
/// assert_eq!(contact_info.get("first_name"), Some("John"));
/// assert_eq!(contact_info.get("last_name"), Some("Doe"));
/// assert_eq!(contact_info.get("middle_name"), None);
/// ```
pub const fn from_fn<F, V>(func: F) -> FnSubstitution<F>
where
	F: Fn(&str) -> Option<V>,
{
	FnSubstitution { func }
}

/// [`VariableMap`] produced by [`map_value()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MapSubstitution<M, F> {
	map: M,
	func: F,
}

impl<'a, M, F, V> VariableMap<'a> for MapSubstitution<M, F>
where
	M: VariableMap<'a>,
	F: Fn(M::Value) -> V,
{
	type Value = V;

	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.map.get(key).map(|value| (self.func)(value))
	}
}

/// Creates a [`VariableMap`] that will apply a function `func` to values found in `map`.
///
///
/// # Example
/// ```rust
/// # use subst::{map_value, VariableMap};
///
/// let contact_info = [("first_name", "John"), ("last_name", "Doe")];
///
/// let contact_info_capitalized = map_value(contact_info, |value| value.to_uppercase());
///
/// assert_eq!(contact_info_capitalized.get("first_name"), Some("JOHN".to_string()));
/// assert_eq!(contact_info_capitalized.get("last_name"), Some("DOE".to_string()));
/// assert_eq!(contact_info_capitalized.get("middle_name"), None);
/// ```
pub const fn map_value<'a, M, F, V>(map: M, func: F) -> MapSubstitution<M, F>
where
	M: VariableMap<'a>,
	F: Fn(M::Value) -> V,
{
	MapSubstitution { map, func }
}
