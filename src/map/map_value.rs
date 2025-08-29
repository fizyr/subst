use super::VariableMap;

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
		let value = self.map.get(key)?;
		Some((self.func)(value))
	}
}

/// Creates a [`VariableMap`] that will apply a function `func` to values found in `map`.
///
///
/// # Example
/// ```rust
/// # use subst::map::{map_value, VariableMap};
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
