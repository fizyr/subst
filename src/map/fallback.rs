use super::VariableMap;

/// [`VariableMap`] produced by [`fallback()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fallback<BaseMap, FallbackMap> {
	base: BaseMap,
	fallback: FallbackMap,
}

impl<'a, BaseMap, FallbackMap> VariableMap<'a> for Fallback<BaseMap, FallbackMap>
where
	BaseMap: VariableMap<'a>,
	FallbackMap: VariableMap<'a, Value = BaseMap::Value>,
{
	type Value = BaseMap::Value;

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
/// # use subst::map::{fallback, VariableMap};
///
/// let contact_info = [("first_name", "John"), ("last_name", "Doe")];
/// let with_fallback = fallback(contact_info, [("middle_name", "<unknown>")]);
///
/// assert_eq!(with_fallback.get("first_name"), Some(&"John"));
/// assert_eq!(with_fallback.get("last_name"), Some(&"Doe"));
/// assert_eq!(with_fallback.get("middle_name"), Some(&"<unknown>"));
/// ```
pub const fn fallback<Base, Fallback>(base: Base, fallback: Fallback) -> self::Fallback<Base, Fallback> {
	self::Fallback { base, fallback }
}
