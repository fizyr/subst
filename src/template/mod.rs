use core::pin::Pin;

use crate::VariableMap;
use crate::error::{ExpandError, ParseError};
use crate::non_aliasing::NonAliasing;

mod raw;

/// A parsed string template that borrows the source string.
///
/// You can parse the template once and call [`Self::expand()`] multiple times.
/// This is generally more efficient than calling [`substitute()`][crate::substitute] multiple times on the same string.
///
/// This template borrows the source string.
/// You can use [`TemplateBuf`] if you need a template that owns the source string.
///
/// If you have a byte slice or vector instead of a string,
/// you can use [`ByteTemplate`] or [`ByteTemplateBuf`].
#[derive(Clone)]
pub struct Template<'a> {
	source: &'a str,
	raw: raw::Template,
}

impl std::fmt::Debug for Template<'_> {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Template").field(&self.source).finish()
	}
}

impl<'a> Template<'a> {
	/// Parse a template from a string slice.
	///
	/// The source is can contain variables to be substituted later,
	/// when you call [`Self::expand()`].
	///
	/// Variables have the form `$NAME`, `${NAME}` or `${NAME:default}`.
	/// A variable name can only consist of ASCII letters, digits and underscores.
	/// They are allowed to start with numbers.
	///
	/// You can escape dollar signs, backslashes, colons and braces with a backslash.
	#[inline]
	#[allow(clippy::should_implement_trait)]
	pub fn from_str(source: &'a str) -> Result<Self, ParseError> {
		Ok(Self {
			source,
			raw: raw::Template::parse(source.as_bytes(), 0)?,
		})
	}

	/// Get the original source string.
	#[inline]
	pub fn source(&self) -> &str {
		self.source
	}

	/// Expand the template.
	///
	/// This will substitute all variables in the template with the values from the given map.
	///
	/// You can pass either a [`HashMap`][std::collections::HashMap], [`BTreeMap`][std::collections::BTreeMap] or [`Env`][crate::Env] as the `variables` parameter.
	/// The maps must have [`&str`] or [`String`] keys, and the values must be [`AsRef<str>`].
	pub fn expand<'b, M>(&self, variables: &'b M) -> Result<String, ExpandError>
	where
		M: VariableMap<'b> + ?Sized,
		M::Value: AsRef<str>,
	{
		let mut output = Vec::with_capacity(self.source.len() + self.source.len() / 10);
		self.raw.expand(&mut output, self.source.as_bytes(), variables, &|x| {
			x.as_ref().as_bytes()
		})?;
		// SAFETY: Both source and all variable values are valid UTF-8, so substitation result is also valid UTF-8.
		unsafe { Ok(String::from_utf8_unchecked(output)) }
	}

	/// Transmute the lifetime of the source data.
	///
	/// # Safety:
	/// You must ensure that template and the source data are not used after the source data becomes invalid.
	unsafe fn transmute_lifetime<'b>(self) -> Template<'b> {
		std::mem::transmute(self)
	}
}

/// A parsed string template that owns the source string.
///
/// You can parse the template once and call [`Self::expand()`] multiple times.
/// This is generally more efficient than calling [`substitute()`][crate::substitute] multiple times on the same string.
///
/// This template owns the source string.
/// If you do not need ownership, you can also use [`Template`] to borrow it instead.
/// Depending on your application, that could prevent creating an unnecessary copy of the source data.
///
/// If you have a byte slice or vector instead of a string,
/// you can use [`ByteTemplate`] or [`ByteTemplateBuf`].
pub struct TemplateBuf {
	// SAFETY: To avoid dangling references, Template must be dropped before
	// source, therefore the template field must be precede the source field.
	//
	// SAFETY: We use NonAliasing<T> to avoid aliassing the source data directly.
	// We only re-create the reference to the source when we call template.inner().
	template: NonAliasing<Template<'static>>,
	source: Pin<String>,
}

impl Clone for TemplateBuf {
	fn clone(&self) -> Self {
		let source = self.source.clone();
		let raw = self.template.inner().raw.clone();

		let template = Template {
			raw,
			source: &*source,
		};
		// SAFETY: The str slice given to `template` must remain valid.
		// Since `String` keeps data on the heap, it remains valid when the `source` is moved.
		// We MUST ensure we do not modify, drop or overwrite `source`.
		let template = unsafe { template.transmute_lifetime() };
		let template = NonAliasing::new(template);
		Self {
			template,
			source,
		}
	}
}

impl std::fmt::Debug for TemplateBuf {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("TemplateBuf")
			.field(&self.template.inner().source())
			.finish()
	}
}

impl TemplateBuf {
	/// Parse a template from a string.
	///
	/// This takes ownership of the string.
	///
	/// The source is can contain variables to be substituted later,
	/// when you call [`Self::expand()`].
	///
	/// Variables have the form `$NAME`, `${NAME}` or `${NAME:default}`.
	/// A variable name can only consist of ASCII letters, digits and underscores.
	/// They are allowed to start with numbers.
	///
	/// You can escape dollar signs, backslashes, colons and braces with a backslash.
	#[inline]
	pub fn from_string(source: String) -> Result<Self, ParseError> {
		let source = Pin::new(source);
		let template = Template::from_str(&*source)?;

		// SAFETY: The str slice given to `template` must remain valid.
		// Since `String` keeps data on the heap, it remains valid when the `source` is moved.
		// We MUST ensure we do not modify, drop or overwrite `source`.
		let template = unsafe { template.transmute_lifetime() };
		let template = NonAliasing::new(template);
		Ok(Self { source, template })
	}

	/// Consume the template to get the original source string.
	#[inline]
	pub fn into_source(self) -> String {
		// SAFETY: Drop `template` before moving `source` to avoid dangling reference.
		drop(self.template);
		Pin::into_inner(self.source)
	}

	/// Borrow the template.
	#[inline]
	#[allow(clippy::needless_lifetimes)]
	pub fn as_template<'a>(&'a self) -> &'a Template<'a> {
		self.template.inner()
	}

	/// Expand the template.
	///
	/// This will substitute all variables in the template with the values from the given map.
	///
	/// You can pass either a [`HashMap`][std::collections::HashMap], [`BTreeMap`][std::collections::BTreeMap] or [`Env`][crate::Env] as the `variables` parameter.
	/// The maps must have [`&str`] or [`String`] keys, and the values must be [`AsRef<str>`].
	pub fn expand<'b, M>(&self, variables: &'b M) -> Result<String, ExpandError>
	where
		M: VariableMap<'b> + ?Sized,
		M::Value: AsRef<str>,
	{
		self.as_template().expand(variables)
	}
}

impl<'a> From<&'a TemplateBuf> for &'a Template<'a> {
	#[inline]
	fn from(other: &'a TemplateBuf) -> Self {
		other.as_template()
	}
}

impl<'a> From<&'a TemplateBuf> for Template<'a> {
	#[inline]
	fn from(other: &'a TemplateBuf) -> Self {
		other.as_template().clone()
	}
}

impl From<&Template<'_>> for TemplateBuf {
	#[inline]
	fn from(other: &Template<'_>) -> Self {
		other.clone().into()
	}
}

impl From<Template<'_>> for TemplateBuf {
	#[inline]
	fn from(other: Template<'_>) -> Self {
		let source: Pin<String> = Pin::new(other.source.into());

		let template = Template {
			source: &*source,
			raw: other.raw,
		};

		// SAFETY: The slice given to `template` must remain valid.
		// Since `String` keeps data on the heap, it remains valid when the `source` is moved.
		// We MUST ensure we do not modify, drop or overwrite `source`.
		let template = unsafe { template.transmute_lifetime() };
		let template = NonAliasing::new(template);

		Self { source, template }
	}
}

/// A parsed byte template that borrows the source slice.
///
/// You can parse the template once and call [`Self::expand()`] multiple times.
/// This is generally more efficient than calling [`substitute()`][crate::substitute] multiple times on the same string.
///
/// This template borrows the source data.
/// You can use [`ByteTemplateBuf`] if you need a template that owns the source data.
///
/// If you have a string instead of a byte slice,
/// you can use [`Template`] or [`TemplateBuf`].
#[derive(Clone)]
pub struct ByteTemplate<'a> {
	source: &'a [u8],
	raw: raw::Template,
}

impl std::fmt::Debug for ByteTemplate<'_> {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("ByteTemplate")
			.field(&DebugByteString(self.source))
			.finish()
	}
}

impl<'a> ByteTemplate<'a> {
	/// Parse a template from a byte slice.
	///
	/// The source is can contain variables to be substituted later,
	/// when you call [`Self::expand()`].
	///
	/// Variables have the form `$NAME`, `${NAME}` or `${NAME:default}`.
	/// A variable name can only consist of ASCII letters, digits and underscores.
	/// They are allowed to start with numbers.
	///
	/// You can escape dollar signs, backslashes, colons and braces with a backslash.
	#[inline]
	pub fn from_slice(source: &'a [u8]) -> Result<Self, ParseError> {
		Ok(Self {
			source,
			raw: raw::Template::parse(source, 0)?,
		})
	}

	/// Get the original source slice.
	#[inline]
	pub fn source(&self) -> &[u8] {
		self.source
	}

	/// Expand the template.
	///
	/// This will substitute all variables in the template with the values from the given map.
	///
	/// You can pass either a [`HashMap`][std::collections::HashMap], [`BTreeMap`][std::collections::BTreeMap] or [`Env`][crate::Env] as the `variables` parameter.
	/// The maps must have [`&str`] or [`String`] keys, and the values must be [`AsRef<[u8]>`].
	pub fn expand<'b, M>(&self, variables: &'b M) -> Result<Vec<u8>, ExpandError>
	where
		M: VariableMap<'b> + ?Sized,
		M::Value: AsRef<[u8]>,
	{
		let mut output = Vec::with_capacity(self.source.len() + self.source.len() / 10);
		self.raw.expand(&mut output, self.source, variables, &|x| x.as_ref())?;
		Ok(output)
	}

	/// Transmute the lifetime of the source data.
	///
	/// # Safety:
	/// You must ensure that template and the source data are not used after the source data becomes invalid.
	unsafe fn transmute_lifetime<'b>(self) -> ByteTemplate<'b> {
		std::mem::transmute(self)
	}
}

/// A parsed byte template that owns the source vector.
///
/// You can parse the template once and call [`Self::expand()`] multiple times.
/// This is generally more efficient than calling [`substitute()`][crate::substitute] multiple times on the same string.
///
/// This template takes ownership of the source data.
/// If you do not need ownership, you can also use [`ByteTemplate`] to borrow it instead.
/// Depending on your application, that could prevent creating an unnecessary copy of the source data.
///
/// If you have a string instead of a byte slice,
/// you can use [`Template`] or [`TemplateBuf`].
pub struct ByteTemplateBuf {
	// SAFETY: To avoid dangling references, Template must be dropped before
	// source, therefore the template field must be precede the source field.
	//
	// SAFETY: We use NonAliasing<T> to avoid aliassing the source data directly.
	// We only re-create the reference to the source when we call template.inner().
	template: NonAliasing<ByteTemplate<'static>>,

	source: Pin<Vec<u8>>,
}

impl Clone for ByteTemplateBuf {
	fn clone(&self) -> Self {
		let source = self.source.clone();
		let raw = self.template.inner().raw.clone();

		let template = ByteTemplate {
			raw,
			source: &*source,
		};

		// SAFETY: The slice given to `template` must remain valid.
		// Since `Pin<Vec<u8>>` keeps data on the heap, it remains valid when the `source` is moved.
		// We MUST ensure we do not modify, drop or overwrite `source`.
		let template = unsafe { template.transmute_lifetime() };
		let template = NonAliasing::new(template);

		Self {
			template,
			source,
		}
	}
}

impl std::fmt::Debug for ByteTemplateBuf {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("ByteTemplateBuf")
			.field(&DebugByteString(self.as_template().source()))
			.finish()
	}
}

impl ByteTemplateBuf {
	/// Parse a template from a vector of bytes.
	///
	/// The source is can contain variables to be substituted later,
	/// when you call [`Self::expand()`].
	///
	/// Variables have the form `$NAME`, `${NAME}` or `${NAME:default}`.
	/// A variable name can only consist of ASCII letters, digits and underscores.
	/// They are allowed to start with numbers.
	///
	/// You can escape dollar signs, backslashes, colons and braces with a backslash.
	#[inline]
	pub fn from_vec(source: Vec<u8>) -> Result<Self, ParseError> {
		let source = Pin::new(source);
		let template = ByteTemplate::from_slice(&*source)?;

		// SAFETY: The slice given to `template` must remain valid.
		// Since `Vec` keeps data on the heap, it remains valid when the `source` is moved.
		// We MUST ensure we do not modify, drop or overwrite `source`.
		let template = unsafe { template.transmute_lifetime() };
		let template = NonAliasing::new(template);

		Ok(Self { source, template })
	}

	/// Consume the template to get the original source vector.
	#[inline]
	pub fn into_source(self) -> Vec<u8> {
		// SAFETY: Drop `template` before moving `source` to avoid dangling reference.
		drop(self.template);
		Pin::into_inner(self.source)
	}

	/// Borrow the template.
	#[inline]
	#[allow(clippy::needless_lifetimes)]
	pub fn as_template<'a>(&'a self) -> &'a ByteTemplate<'a> {
		self.template.inner()
	}

	/// Expand the template.
	///
	/// This will substitute all variables in the template with the values from the given map.
	///
	/// You can pass either a [`HashMap`][std::collections::HashMap], [`BTreeMap`][std::collections::BTreeMap] or [`Env`][crate::Env] as the `variables` parameter.
	/// The maps must have [`&str`] or [`String`] keys, and the values must be [`AsRef<[u8]>`].
	pub fn expand<'b, M>(&self, variables: &'b M) -> Result<Vec<u8>, ExpandError>
	where
		M: VariableMap<'b> + ?Sized,
		M::Value: AsRef<[u8]>,
	{
		self.as_template().expand(variables)
	}
}

impl<'a> From<&'a ByteTemplateBuf> for &'a ByteTemplate<'a> {
	#[inline]
	fn from(other: &'a ByteTemplateBuf) -> Self {
		other.as_template()
	}
}

impl<'a> From<&'a ByteTemplateBuf> for ByteTemplate<'a> {
	#[inline]
	fn from(other: &'a ByteTemplateBuf) -> Self {
		other.as_template().clone()
	}
}

impl From<&ByteTemplate<'_>> for ByteTemplateBuf {
	#[inline]
	fn from(other: &ByteTemplate<'_>) -> Self {
		other.clone().into()
	}
}

impl From<ByteTemplate<'_>> for ByteTemplateBuf {
	#[inline]
	fn from(other: ByteTemplate<'_>) -> Self {
		let source: Vec<u8> = other.source.into();
		let source = Pin::new(source);

		let template = ByteTemplate {
			source: &*source,
			raw: other.raw,
		};

		// SAFETY: The slice given to `template` must remain valid.
		// Since `Pin<Vec<u8>>` keeps data on the heap, it remains valid when the `source` is moved.
		// We MUST ensure we do not modify, drop or overwrite `source`.
		let template = unsafe { template.transmute_lifetime() };
		let template = NonAliasing::new(template);

		Self { source, template }
	}
}

struct DebugByteString<'a>(&'a [u8]);

impl std::fmt::Debug for DebugByteString<'_> {
	#[inline]
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Ok(data) = std::str::from_utf8(self.0) {
			write!(f, "b{:?}", data)
		} else {
			std::fmt::Debug::fmt(self.0, f)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use assert2::{assert, check, let_assert};
	use std::collections::BTreeMap;

	#[test]
	fn test_clone_template_buf() {
		let mut map: BTreeMap<String, String> = BTreeMap::new();
		map.insert("name".into(), "world".into());
		let source = "Hello ${name}!";
		let_assert!(Ok(buf1) = TemplateBuf::from_string(source.into()));
		let buf2 = buf1.clone();
		let mut string = buf1.into_source();
		string.as_mut()[..5].make_ascii_uppercase();
		check!(let Ok("Hello world!") = buf2.expand(&map).as_deref());
		assert!(buf2.as_template().source() == source);
		assert!(buf2.into_source() == source);
	}

	#[test]
	fn test_clone_byte_template_buf() {
		let mut map: BTreeMap<String, String> = BTreeMap::new();
		map.insert("name".into(), "world".into());
		let source = b"Hello ${name}!";
		let_assert!(Ok(buf1) = ByteTemplateBuf::from_vec(source.into()));
		let buf2 = buf1.clone();
		let mut string = buf1.into_source();
		string.as_mut_slice()[..5].make_ascii_uppercase();
		check!(let Ok(b"Hello world!") = buf2.expand(&map).as_deref());
		assert!(buf2.as_template().source() == source);
		assert!(buf2.into_source() == source);
	}

	#[test]
	fn test_move_template_buf() {
		#[inline(never)]
		fn check_template(buf: TemplateBuf) {
			let mut map: BTreeMap<String, String> = BTreeMap::new();
			map.insert("name".into(), "world".into());
			assert!(buf.as_template().source() == "Hello ${name}!");
			let_assert!(Ok(expanded) = buf.as_template().expand(&map));
			assert!(expanded == "Hello world!");
		}

		let source = "Hello ${name}!";
		let_assert!(Ok(buf1) = TemplateBuf::from_string(source.into()));
		check_template(buf1);
	}

	#[test]
	fn test_move_byte_template_buf() {
		#[inline(never)]
		fn check_template(buf: ByteTemplateBuf) {
			let mut map: BTreeMap<String, String> = BTreeMap::new();
			map.insert("name".into(), "world".into());
			assert!(buf.as_template().source() == b"Hello ${name}!");
			let_assert!(Ok(expanded) = buf.as_template().expand(&map));
			assert!(expanded == b"Hello world!");
		}

		let source = b"Hello ${name}!";
		let_assert!(Ok(buf1) = ByteTemplateBuf::from_vec(source.into()));
		check_template(buf1);
	}
}
