use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::Error;

use crate::{ByteTemplate, ByteTemplateBuf, Template, TemplateBuf};

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
		let source: &'de str = Deserialize::deserialize(deserializer)?;
		Self::from_str(source).map_err(D::Error::custom)
	}
}

impl<'de> Deserialize<'de> for TemplateBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let source: String = Deserialize::deserialize(deserializer)?;
		Self::from_string(source).map_err(D::Error::custom)
	}
}

impl<'de> Deserialize<'de> for ByteTemplate<'de> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let source: &'de [u8] = Deserialize::deserialize(deserializer)?;
		Self::from_slice(source).map_err(D::Error::custom)
	}
}

impl<'de> Deserialize<'de> for ByteTemplateBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ByteBufVisitor;
		impl<'de> serde::de::Visitor<'de> for ByteBufVisitor {
			type Value = Vec<u8>;
			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "bytes")
			}
			fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
				Ok(v.to_vec())
			}
			fn visit_byte_buf<E: Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
				Ok(v)
			}
		}

		let source = deserializer.deserialize_byte_buf(ByteBufVisitor)?;
		Self::from_vec(source).map_err(D::Error::custom)
	}
}

#[cfg(test)]
mod test {
	use crate::{ByteTemplate, ByteTemplateBuf, Template, TemplateBuf};
	use assert2::let_assert;
	use serde_test::{assert_tokens, Token};

	const SOURCE: &str = "Hello $name";

	#[test]
	fn template_ser_de() {
		let_assert!(Ok(template) = Template::from_str(SOURCE));
		assert_tokens(&template, &[Token::BorrowedStr(SOURCE)]);
	}

	#[test]
	fn template_buf_ser_de() {
		let_assert!(Ok(template) = TemplateBuf::from_string(SOURCE.into()));
		assert_tokens(&template, &[Token::String(SOURCE)]);
	}

	#[test]
	fn byte_template_ser_de() {
		let_assert!(Ok(template) = ByteTemplate::from_slice(SOURCE.as_bytes()));
		assert_tokens(&template, &[Token::BorrowedBytes(SOURCE.as_bytes())]);
	}

	#[test]
	fn byte_template_buf_ser_de() {
		let_assert!(Ok(template) = ByteTemplateBuf::from_vec(SOURCE.as_bytes().to_vec()));
		serde_test::assert_tokens(&template, &[Token::ByteBuf(SOURCE.as_bytes())]);
	}
}
