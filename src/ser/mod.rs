use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::ser::{self, Serialize};

pub mod pretty;

#[cfg(test)]
mod tests;
mod value;

#[cfg(not(target_os = "windows"))]
const NEWLINE: &str = "\n";

#[cfg(target_os = "windows")]
const NEWLINE: &str = "\r\n";

/// Serializes `value` and returns it as string.
///
/// This function does not generate any newlines or nice formatting;
/// if you want that, you can use `pretty::to_string` instead.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut s = Serializer {
        output: String::new(),
        pretty: None,
        struct_names: false,
    };
    value.serialize(&mut s)?;
    Ok(s.output)
}

/// Serialization result.
pub type Result<T> = ::std::result::Result<T, Error>;

/// Serialization error.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// A custom error emitted by a serialized value.
    Message(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Error::Message(ref e) => write!(f, "Custom message: {}", e),
        }
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref e) => e,
        }
    }
}

struct Pretty {
    indent: usize,
}

/// The RSON serializer.
///
/// You can just use `to_string` for deserializing a value.
/// If you want it pretty-printed, take a look at the `pretty` module.
pub struct Serializer {
    output: String,
    pretty: Option<Pretty>,
    struct_names: bool,
}

impl Serializer {
    fn start_indent(&mut self) {
        if let Some(ref mut pretty) = self.pretty {
            pretty.indent += 1;
            self.output += NEWLINE;
        }
    }

    fn indent(&mut self) {
        if let Some(ref pretty) = self.pretty {
            self.output.extend((0..pretty.indent * 4).map(|_| " "));
        }
    }

    fn end_indent(&mut self) {
        if let Some(ref mut pretty) = self.pretty {
            pretty.indent -= 1;
            self.output.extend((0..pretty.indent * 4).map(|_| " "));
        }
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output += if v { "true" } else { "false" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        // TODO optimize
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.output += "'";
        if v == '\\' || v == '\'' {
            self.output.push('\\');
        }
        self.output.push(v);
        self.output += "'";
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output += "\"";
        for char in v.chars() {
            if char == '\\' || char == '"' {
                self.output.push('\\');
            }
            self.output.push(char);
        }
        self.output += "\"";
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.output += "None";

        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "Some(";
        value.serialize(&mut *self)?;
        self.output += ")";

        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        self.output += "()";

        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        if self.struct_names {
            self.output += name;

            Ok(())
        } else {
            self.serialize_unit()
        }
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<()> {
        self.output += variant;

        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if self.struct_names {
            self.output += name;
        }

        self.output += "(";
        value.serialize(&mut *self)?;
        self.output += ")";
        Ok(())
    }

    fn serialize_newtype_variant<T>(self, _: &'static str, _: u32, variant: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += variant;
        self.output += "(";
        value.serialize(&mut *self)?;
        self.output += ")";
        Ok(())
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output += "[";

        self.start_indent();

        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> {
        self.output += "(";

        Ok(self)
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        if self.struct_names {
            self.output += name;
        }

        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.output += variant;
        self.output += "(";

        self.start_indent();

        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.output += "{";

        self.start_indent();

        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Self::SerializeStruct> {
        if self.struct_names {
            self.output += name;
        }
        self.output += "{";

        self.start_indent();

        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.output += variant;
        self.output += "{";

        self.start_indent();

        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.indent();

        value.serialize(&mut **self)?;
        self.output += ",";

        if self.pretty.is_some() {
            self.output += NEWLINE;
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.end_indent();

        self.output += "]";
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        self.output += ",";

        if self.pretty.is_some() {
            self.output += " ";
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        if self.pretty.is_some() {
            self.output.pop();
            self.output.pop();
        }

        self.output += ")";

        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.indent();

        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ":";

        if self.pretty.is_some() {
            self.output += " ";
        }

        value.serialize(&mut **self)?;
        self.output += ",";

        if self.pretty.is_some() {
            self.output += NEWLINE;
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.end_indent();

        self.output += "}";
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.indent();

        self.output += key;
        self.output += ":";

        if self.pretty.is_some() {
            self.output += " ";
        }

        value.serialize(&mut **self)?;
        self.output += ",";

        if self.pretty.is_some() {
            self.output += NEWLINE;
        }

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.end_indent();

        self.output += "}";
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeStruct::end(self)
    }
}
