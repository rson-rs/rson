use std::borrow::Cow;
use std::{io, str};

use serde::de::{
    Deserialize, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::Deserializer as SerdeDeserializer;

/// Deserialization module.
pub use self::error::{Error, ParseError, Result};
use self::id::IdDeserializer;
use super::parse::Bytes;

mod error;
mod id;
#[cfg(test)]
mod tests;
mod value;

/// The RSON deserializer.
///
/// If you just want to simply deserialize a value,
/// you can use the `from_str` convenience function.
pub struct Deserializer<'de> {
    bytes: Bytes<'de>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Deserializer {
            bytes: Bytes::new(input.as_bytes()),
        }
    }

    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer {
            bytes: Bytes::new(input),
        }
    }

    pub fn remainder(&self) -> Cow<str> {
        String::from_utf8_lossy(&self.bytes.bytes())
    }

    /// Check if the remaining bytes are whitespace only,
    /// otherwise return an error.
    pub fn end(&mut self) -> Result<()> {
        self.bytes.skip_ws();

        if self.bytes.bytes().is_empty() {
            Ok(())
        } else {
            self.bytes.err(ParseError::TrailingCharacters)
        }
    }
}

/// A convenience function for reading data from a reader
/// and feeding into a deserializer
pub fn from_reader<R, T>(mut rdr: R) -> Result<T>
where
    R: io::Read,
    T: DeserializeOwned,
{
    let mut bytes = Vec::new();
    rdr.read_to_end(&mut bytes)?;
    let s = str::from_utf8(&bytes)?;
    from_str(s)
}

/// A convenience function for building a deserializer
/// and deserializing a value of type `T`.
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;

    deserializer.end()?;

    Ok(t)
}

impl<'de, 'a> SerdeDeserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume_ident("true") {
            return visitor.visit_bool(true);
        } else if self.bytes.consume_ident("false") {
            return visitor.visit_bool(false);
        } else if self.bytes.check_ident("Some") {
            return self.deserialize_option(visitor);
        } else if self.bytes.consume_ident("None") {
            return visitor.visit_none();
        } else if self.bytes.consume("()") {
            return visitor.visit_unit();
        }

        if self.bytes.identifier().is_ok() {
            self.bytes.skip_ws();

            return self.deserialize_struct("", &[], visitor);
        }

        match self.bytes.peek_or_eof()? {
            b'{' => self.deserialize_map(visitor),
            b'(' => self.deserialize_tuple(0, visitor),
            b'[' => self.deserialize_seq(visitor),
            b'0'..=b'9' | b'+' | b'-' | b'.' => self.deserialize_f64(visitor),
            b'"' => self.deserialize_string(visitor),
            b'\'' => self.deserialize_char(visitor),
            other => self.bytes.err(ParseError::UnexpectedByte(other as char)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.bytes.bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.bytes.signed_integer()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.bytes.signed_integer()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.bytes.signed_integer()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.bytes.signed_integer()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.bytes.unsigned_integer()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.bytes.unsigned_integer()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.bytes.float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.bytes.float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.bytes.char()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        use super::parse::ParsedStr;

        match self.bytes.string()? {
            ParsedStr::Allocated(s) => visitor.visit_string(s),
            ParsedStr::Slice(s) => visitor.visit_str(s),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("Some") && {
            self.bytes.skip_ws();
            self.bytes.consume("(")
        } {
            self.bytes.skip_ws();

            let v = visitor.visit_some(&mut *self)?;

            self.bytes.skip_ws();

            if self.bytes.consume(")") {
                Ok(v)
            } else {
                self.bytes.err(ParseError::ExpectedOptionEnd)
            }
        } else if self.bytes.consume("None") {
            visitor.visit_none()
        } else {
            self.bytes.err(ParseError::ExpectedOption)
        }
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("()") {
            visitor.visit_unit()
        } else {
            self.bytes.err(ParseError::ExpectedUnit)
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume(name) {
            visitor.visit_unit()
        } else {
            self.deserialize_unit(visitor)
        }
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.bytes.consume(name);

        self.bytes.skip_ws();

        if self.bytes.consume("(") {
            let value = visitor.visit_newtype_struct(&mut *self)?;
            self.bytes.comma();

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                self.bytes.err(ParseError::ExpectedStructEnd)
            }
        } else {
            self.bytes.err(ParseError::ExpectedStruct)
        }
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("[") {
            let value = visitor.visit_seq(CommaSeparated::new(b']', &mut self, 0))?;
            self.bytes.comma();

            if self.bytes.consume("]") {
                Ok(value)
            } else {
                self.bytes.err(ParseError::ExpectedArrayEnd)
            }
        } else {
            self.bytes.err(ParseError::ExpectedArray)
        }
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("(") {
            let value = visitor.visit_seq(CommaSeparated::new(b')', &mut self, 0))?;
            self.bytes.comma();

            if self.bytes.consume(")") {
                Ok(value)
            } else {
                self.bytes.err(ParseError::ExpectedArrayEnd)
            }
        } else {
            self.bytes.err(ParseError::ExpectedArray)
        }
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.bytes.consume(name);
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.bytes.consume("{") {
            let value = visitor.visit_map(CommaSeparated::new(b'}', &mut self, 0))?;
            self.bytes.comma();

            if self.bytes.consume("}") {
                Ok(value)
            } else {
                self.bytes.err(ParseError::ExpectedMapEnd)
            }
        } else {
            self.bytes.err(ParseError::ExpectedMap)
        }
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.bytes.consume(name);

        self.bytes.skip_ws();

        if self.bytes.consume("{") {
            let value = visitor.visit_map(CommaSeparated::new(b'}', &mut self, Flags::IS_STRUCT))?;
            self.bytes.comma();

            if self.bytes.consume("}") {
                Ok(value)
            } else {
                self.bytes.err(ParseError::ExpectedStructEnd)
            }
        } else {
            self.bytes.err(ParseError::ExpectedStruct)
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(Enum::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.bytes.identifier()?)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

type Flags = u8;

trait CommaSeparatedFlag {
    const IS_MAP: u8 = 0b00000001;
    const IS_STRUCT: u8 = 0b00000010;
    const HAD_COMMA: u8 = 0b00000100;

    fn is_map(&self) -> bool;
    fn is_struct(&self) -> bool;
    fn had_comma(&self) -> bool;
}

impl CommaSeparatedFlag for Flags {
    fn is_map(&self) -> bool {
        self & Self::IS_MAP > 0
    }

    fn is_struct(&self) -> bool {
        self & Self::IS_STRUCT > 0
    }

    fn had_comma(&self) -> bool {
        self & Self::HAD_COMMA > 0
    }
}

struct CommaSeparated<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    terminator: u8,
    flags: Flags,
}

impl<'a, 'de> CommaSeparated<'a, 'de> {
    fn new(terminator: u8, de: &'a mut Deserializer<'de>, flags: u8) -> Self {
        CommaSeparated {
            de,
            terminator,
            flags: flags | Flags::HAD_COMMA,
        }
    }

    fn err<T>(&self, kind: ParseError) -> Result<T> {
        self.de.bytes.err(kind)
    }

    fn has_element(&mut self) -> Result<bool> {
        self.de.bytes.skip_ws();

        Ok(self.flags.had_comma() && self.de.bytes.peek_or_eof()? != self.terminator)
    }
}

impl<'de, 'a> SeqAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.has_element()? {
            let res = seed.deserialize(&mut *self.de)?;

            self.flags |= if self.de.bytes.comma() { Flags::HAD_COMMA } else { 0 };

            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

impl<'de, 'a> MapAccess<'de> for CommaSeparated<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.has_element()? {
            if !self.flags.is_map() && !self.flags.is_struct() {
                if self.de.bytes.is_identifier()? {
                    self.flags |= Flags::IS_STRUCT;
                } else {
                    self.flags |= Flags::IS_MAP;
                }
            }
            if self.flags.is_struct() {
                seed.deserialize(&mut IdDeserializer::new(&mut *self.de)).map(Some)
            } else {
                seed.deserialize(&mut *self.de).map(Some)
            }
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws();

        if self.de.bytes.consume(":") {
            self.de.bytes.skip_ws();

            let res = seed.deserialize(&mut *self.de)?;

            self.flags |= if self.de.bytes.comma() { Flags::HAD_COMMA } else { 0 };

            Ok(res)
        } else {
            self.err(ParseError::ExpectedMapColon)
        }
    }
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let value = seed.deserialize(&mut *self.de)?;

        Ok((value, self))
    }
}

impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.bytes.skip_ws();

        if self.de.bytes.consume("(") {
            let val = seed.deserialize(&mut *self.de)?;

            self.de.bytes.comma();

            if self.de.bytes.consume(")") {
                Ok(val)
            } else {
                self.de.bytes.err(ParseError::ExpectedStructEnd)
            }
        } else {
            self.de.bytes.err(ParseError::ExpectedStruct)
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.bytes.skip_ws();

        self.de.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.bytes.skip_ws();

        self.de.deserialize_struct("", fields, visitor)
    }
}
