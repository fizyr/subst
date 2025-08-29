use super::VariableMap;

/// [`VariableMap`] produced by [`from_fn()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FnMap<F> {
	func: F,
}

impl<'a, F, V> VariableMap<'a> for FnMap<F>
where
	F: 'a + Fn(&str) -> Option<V>,
{
	type Value = V;

	#[inline(always)]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		(self.func)(key)
	}
}

/// Creates a [`VariableMap`] that delegates to the given function.
///
/// # Example
/// ```rust
/// # use subst::map::{from_fn, VariableMap};
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
pub const fn from_fn<F, V>(func: F) -> FnMap<F>
where
	F: Fn(&str) -> Option<V>,
{
	FnMap { func }
}
