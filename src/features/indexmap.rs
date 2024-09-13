use indexmap::IndexMap;

use crate::VariableMap;

impl<'a, V: 'a> VariableMap<'a> for IndexMap<&str, V> {
	type Value = &'a V;

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

impl<'a, V: 'a> VariableMap<'a> for IndexMap<String, V> {
	type Value = &'a V;

	#[inline]
	fn get(&'a self, key: &str) -> Option<Self::Value> {
		self.get(key)
	}
}

#[cfg(test)]
#[rustfmt::skip]
mod test {
	use indexmap::IndexMap;
	use assert2::check;

	use crate::{substitute, substitute_bytes};

	#[test]
	fn test_substitute() {
		let mut map: IndexMap<String, String> = IndexMap::new();
		map.insert("name".into(), "world".into());
		check!(let Ok("Hello world!") = substitute("Hello $name!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${not_name:world}!", &map).as_deref());

		let mut map: IndexMap<&str, &str> = IndexMap::new();
		map.insert("name", "world");
		check!(let Ok("Hello world!") = substitute("Hello $name!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok("Hello world!") = substitute("Hello ${not_name:world}!", &map).as_deref());
	}

	#[test]
	fn test_substitute_bytes() {
		let mut map: IndexMap<String, Vec<u8>> = IndexMap::new();
		map.insert("name".into(), b"world"[..].into());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello $name!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${not_name:world}!", &map).as_deref());

		let mut map: IndexMap<&str, &[u8]> = IndexMap::new();
		map.insert("name", b"world");
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello $name!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${name:not-world}!", &map).as_deref());
		check!(let Ok(b"Hello world!") = substitute_bytes(b"Hello ${not_name:world}!", &map).as_deref());
	}
}
