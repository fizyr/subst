use super::VariableMap;

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
