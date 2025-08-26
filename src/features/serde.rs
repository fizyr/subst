use std::marker::PhantomData;

use serde::{
	de::{Error, Visitor},
	Deserialize,
	Deserializer,
	Serialize,
	Serializer,
};

use crate::{ByteTemplate, ByteTemplateBuf, Template, TemplateBuf};

struct TemplateVisitor<'de> {
	_lifetime: PhantomData<&'de ()>,
}

impl<'de> TemplateVisitor<'de> {
	const fn new() -> Self {
		Self { _lifetime: PhantomData }
	}
}

impl<'de> Visitor<'de> for TemplateVisitor<'de> {
	type Value = Template<'de>;

	fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
	where
		E: Error,
	{
		Template::from_str(v).map_err(E::custom)
	}

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a borrowed string")
	}
}

struct TemplateBufVisitor;

impl<'de> Visitor<'de> for TemplateBufVisitor {
	type Value = TemplateBuf;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a string")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: Error,
	{
		self.visit_string(v.to_owned())
	}

	fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
	where
		E: Error,
	{
		TemplateBuf::from_string(v).map_err(E::custom)
	}
}

struct ByteTemplateVisitor<'de> {
	_lifetime: PhantomData<&'de ()>,
}

impl<'de> ByteTemplateVisitor<'de> {
	const fn new() -> Self {
		Self { _lifetime: PhantomData }
	}
}

impl<'de> Visitor<'de> for ByteTemplateVisitor<'de> {
	type Value = ByteTemplate<'de>;

	fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
	where
		E: Error,
	{
		ByteTemplate::from_slice(v).map_err(E::custom)
	}

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a borrowed string")
	}
}

struct ByteTemplateBufVisitor;

impl<'de> Visitor<'de> for ByteTemplateBufVisitor {
	type Value = ByteTemplateBuf;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a string")
	}

	fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
	where
		E: Error,
	{
		self.visit_byte_buf(v.to_vec())
	}

	fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
	where
		E: Error,
	{
		ByteTemplateBuf::from_vec(v).map_err(E::custom)
	}
}

impl Serialize for Template<'_> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.source())
	}
}

impl Serialize for TemplateBuf {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.as_template().source())
	}
}

impl Serialize for ByteTemplate<'_> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_bytes(self.source())
	}
}

impl Serialize for ByteTemplateBuf {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_bytes(self.as_template().source())
	}
}

impl<'de> Deserialize<'de> for Template<'de> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(TemplateVisitor::new())
	}
}

impl<'de> Deserialize<'de> for TemplateBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_string(TemplateBufVisitor)
	}
}

impl<'de> Deserialize<'de> for ByteTemplate<'de> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_bytes(ByteTemplateVisitor::new())
	}
}

impl<'de> Deserialize<'de> for ByteTemplateBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_string(ByteTemplateBufVisitor)
	}
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use crate::{ByteTemplate, ByteTemplateBuf, Template, TemplateBuf};

	const STR_SOURCE: &str = "{hello}";
	const BYTE_SOURCE: &[u8] = b"{hello}";

	#[test]
	fn template_ser_de() {
		let template = Template::from_str(STR_SOURCE).unwrap();

		assert_tokens(&template, &[Token::BorrowedStr(STR_SOURCE)]);
	}

	#[test]
	fn template_buf_ser_de() {
		let template = TemplateBuf::from_string(STR_SOURCE.to_string()).unwrap();

		assert_tokens(&template, &[Token::String(STR_SOURCE)]);
	}

	#[test]
	fn byte_template_ser_de() {
		let template = ByteTemplate::from_slice(BYTE_SOURCE).unwrap();

		assert_tokens(&template, &[Token::BorrowedBytes(BYTE_SOURCE)]);
	}

	#[test]
	fn byte_template_buf_ser_de() {
		let template = ByteTemplateBuf::from_vec(BYTE_SOURCE.to_vec()).unwrap();

		assert_tokens(&template, &[Token::ByteBuf(BYTE_SOURCE)]);
	}
}
