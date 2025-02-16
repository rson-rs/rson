use std::collections::HashMap;
use std::{io, mem};

use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
    SerializeTupleVariant,
};
use serde::{Serialize, Serializer};

use self::error::{RsonSerError, RsonSerResult};
use self::formatter::{CompactFormatter, Formatter, PrettyFormatter};

pub mod error;
pub mod formatter;

pub fn to_string_compact<T: ?Sized + Serialize>(value: &T) -> RsonSerResult<String> {
    compact().to_string(value)
}

pub fn to_string_pretty<T: ?Sized + Serialize>(value: &T) -> RsonSerResult<String> {
    pretty().to_string(value)
}

pub fn builder<F: Formatter + Clone>(formatter: F) -> RsonSerBuilder<F> {
    RsonSerBuilder::with_formatter(formatter)
}

pub fn compact() -> RsonSerBuilder<CompactFormatter> {
    RsonSerBuilder::compact()
}

pub fn pretty<'a>() -> RsonSerBuilder<PrettyFormatter<'a>> {
    RsonSerBuilder::pretty()
}

pub struct RsonSerBuilder<F> {
    formatter: F,
    vars: HashMap<Vec<String>, String>,
}

impl RsonSerBuilder<CompactFormatter> {
    pub fn compact() -> Self {
        Self::with_formatter(CompactFormatter)
    }
}

impl RsonSerBuilder<PrettyFormatter<'_>> {
    pub fn pretty() -> Self {
        Self::with_formatter(PrettyFormatter::new())
    }
}

impl<F: Formatter + Clone> RsonSerBuilder<F> {
    pub fn with_formatter(formatter: F) -> Self {
        Self {
            formatter,
            vars: HashMap::new(),
        }
    }

    pub fn with_vars(
        mut self,
        vars: impl IntoIterator<Item = (impl IntoIterator<Item = impl Into<String>>, impl Into<String>)>,
    ) -> Self {
        self.add_vars(vars);
        self
    }

    pub fn add_vars(
        &mut self,
        vars: impl IntoIterator<Item = (impl IntoIterator<Item = impl Into<String>>, impl Into<String>)>,
    ) -> &mut Self {
        self.vars.extend(
            vars.into_iter()
                .map(|(location, value)| (location.into_iter().map(Into::into).collect(), value.into())),
        );
        self
    }

    pub fn to_string<T: ?Sized + Serialize>(&mut self, value: &T) -> RsonSerResult<String> {
        let mut buf = Vec::new();
        self.to_writer(&mut buf, value)?;

        Ok(unsafe { String::from_utf8_unchecked(buf) })
    }

    pub fn to_string_var<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
        var_name: impl AsRef<str>,
    ) -> RsonSerResult<String> {
        let mut buf = Vec::new();
        self.to_writer_var(&mut buf, value, var_name)?;

        Ok(unsafe { String::from_utf8_unchecked(buf) })
    }

    pub fn to_writer<T, W>(&mut self, writer: W, value: &T) -> RsonSerResult<()>
    where
        W: io::Write,
        T: ?Sized + Serialize,
    {
        let vars = mem::take(&mut self.vars);
        let mut ser = RsonSerializer::with_formatter(writer, self.formatter.clone()).with_vars(vars);
        ser.serialize(value)
    }

    pub fn to_writer_var<T, W>(&mut self, writer: W, value: &T, var_name: impl AsRef<str>) -> RsonSerResult<()>
    where
        W: io::Write,
        T: ?Sized + Serialize,
    {
        let vars = mem::take(&mut self.vars);
        let mut ser = RsonSerializer::with_formatter(writer, self.formatter.clone()).with_vars(vars);
        ser.begin_let(var_name.as_ref())?;
        ser.serialize(value)?;
        ser.end_let()
    }
}

pub struct RsonSerializer<W, F = CompactFormatter> {
    writer: W,
    formatter: F,
    vars: HashMap<Vec<String>, String>,
    current_location: Vec<String>,
}

impl<W: io::Write> RsonSerializer<W> {
    #[inline]
    pub fn new(writer: W) -> Self {
        RsonSerializer::with_formatter(writer, CompactFormatter)
    }
}

impl<'a, W: io::Write> RsonSerializer<W, PrettyFormatter<'a>> {
    #[inline]
    pub fn pretty(writer: W) -> Self {
        Self::with_formatter(writer, PrettyFormatter::new())
    }
}

impl<W: io::Write, F: Formatter> RsonSerializer<W, F> {
    #[inline]
    pub fn with_formatter(writer: W, formatter: F) -> Self {
        Self {
            writer,
            formatter,
            vars: HashMap::new(),
            current_location: Vec::new(),
        }
    }

    #[inline]
    pub fn with_vars(mut self, vars: impl IntoIterator<Item = (Vec<String>, String)>) -> Self {
        self.vars.extend(vars);
        self
    }

    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }

    #[inline]
    pub fn begin_let(&mut self, var_name: &str) -> RsonSerResult<()> {
        self.formatter.begin_let(&mut self.writer, var_name)?;
        Ok(())
    }

    #[inline]
    pub fn end_let(&mut self) -> RsonSerResult<()> {
        self.formatter.end_let(&mut self.writer)?;
        Ok(())
    }

    #[inline]
    pub fn find_var_name(&self) -> Option<&String> {
        self.vars.get(&self.current_location)
    }

    #[inline]
    pub fn maybe_write_var(&mut self) -> RsonSerResult<bool> {
        if let Some(var) = self.vars.get(&self.current_location) {
            self.writer.write_all(var.as_bytes())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[inline]
    pub fn serialize<T: ?Sized + Serialize>(&mut self, value: &T) -> RsonSerResult<()> {
        if !self.maybe_write_var()? {
            value.serialize(self)?;
        }
        Ok(())
    }

    #[inline]
    pub fn serialize_name(&mut self, name: impl AsRef<[u8]>) -> RsonSerResult<()> {
        self.writer.write_all(name.as_ref())?;
        Ok(())
    }

    #[inline]
    pub fn get_struct_fields_to_skip(&self) -> (Vec<String>, Option<String>) {
        let mut location = self.current_location.clone();
        location.push("..".to_string());

        for (key, value) in &self.vars {
            if key.starts_with(location.as_slice()) {
                return (
                    key.iter().skip(location.len()).cloned().collect(),
                    if value.is_empty() { None } else { Some(value.clone()) },
                );
            }
        }
        (Default::default(), None)
    }
}

impl<'a, W, F> Serializer for &'a mut RsonSerializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    type SerializeSeq = RsonCompoundSerializer<'a, W, F>;
    type SerializeTuple = RsonCompoundSerializer<'a, W, F>;
    type SerializeTupleStruct = RsonCompoundSerializer<'a, W, F>;
    type SerializeTupleVariant = RsonCompoundSerializer<'a, W, F>;
    type SerializeMap = RsonCompoundSerializer<'a, W, F>;
    type SerializeStruct = RsonCompoundSerializer<'a, W, F>;
    type SerializeStructVariant = RsonCompoundSerializer<'a, W, F>;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_bool(&mut self.writer, value)?;
        Ok(())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_int(&mut self.writer, i64::from(value))?;
        Ok(())
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_int(&mut self.writer, i64::from(value))?;
        Ok(())
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_int(&mut self.writer, i64::from(value))?;
        Ok(())
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_int(&mut self.writer, i64::from(value))?;
        Ok(())
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_i128(&mut self.writer, value)?;
        Ok(())
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_uint(&mut self.writer, u64::from(value))?;
        Ok(())
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_uint(&mut self.writer, u64::from(value))?;
        Ok(())
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_uint(&mut self.writer, u64::from(value))?;
        Ok(())
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_uint(&mut self.writer, u64::from(value))?;
        Ok(())
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_u128(&mut self.writer, value)?;
        Ok(())
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_f32(&mut self.writer, value)?;
        Ok(())
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_f64(&mut self.writer, value)?;
        Ok(())
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        write!(self.writer, "{value:?}")?;
        Ok(())
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.formatter.before_begin_string(&mut self.writer)?;
        write!(self.writer, "{value:?}")?;
        self.formatter.after_end_string(&mut self.writer)?;
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_byte_array(&mut self.writer, value)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_none(&mut self.writer)?;
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.formatter.begin_some(&mut self.writer)?;
        value.serialize(&mut *self)?;
        self.formatter.end_some(&mut self.writer)?;
        Ok(())
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_unit(&mut self.writer)?;
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_name(name)?;
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.formatter.write_variant_name(&mut self.writer, name, variant)?;
        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_name(name)?;
        self.formatter.begin_tuple(&mut self.writer)?;
        value.serialize(&mut *self)?;
        self.formatter.end_tuple(&mut self.writer)?;
        Ok(())
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_unit_variant(name, variant_index, variant)?;
        self.formatter.begin_tuple(&mut self.writer)?;
        value.serialize(&mut *self)?;
        self.formatter.end_tuple(&mut self.writer)?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.formatter.begin_array(&mut self.writer)?;
        Ok(RsonCompoundSerializer::new(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.formatter.begin_tuple(&mut self.writer)?;
        Ok(RsonCompoundSerializer::new(self))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_name(name)?;
        self.formatter.begin_tuple(&mut self.writer)?;
        Ok(RsonCompoundSerializer::new(self))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_unit_variant(name, variant_index, variant)?;
        self.formatter.begin_tuple(&mut self.writer)?;
        Ok(RsonCompoundSerializer::new(self))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.formatter.before_begin_map(&mut self.writer)?;
        self.formatter.begin_struct(&mut self.writer)?;
        Ok(RsonCompoundSerializer::new(self))
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        self.formatter.before_begin_struct(&mut self.writer, name)?;
        self.formatter.begin_struct(&mut self.writer)?;

        let (to_skip_fields, rest_var) = self.get_struct_fields_to_skip();
        Ok(RsonCompoundSerializer::new(self).with_rest_struct(to_skip_fields, rest_var))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.serialize_unit_variant(name, variant_index, variant)?;
        self.formatter.before_begin_struct(&mut self.writer, "")?;
        self.formatter.begin_struct(&mut self.writer)?;

        let (to_skip_fields, rest_var) = self.get_struct_fields_to_skip();
        Ok(RsonCompoundSerializer::new(self).with_rest_struct(to_skip_fields, rest_var))
    }
}

pub struct RsonCompoundSerializer<'a, W: 'a, F: 'a> {
    ser: &'a mut RsonSerializer<W, F>,
    current_element_index: usize,
    to_skip_struct_fields: Vec<String>,
    rest_struct_var: Option<String>,
}

impl<'a, W, F> RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    fn new(ser: &'a mut RsonSerializer<W, F>) -> Self {
        Self {
            ser,
            current_element_index: 0,
            to_skip_struct_fields: Vec::new(),
            rest_struct_var: None,
        }
    }

    fn with_rest_struct(mut self, to_skip_fields: Vec<String>, rest_var: Option<String>) -> Self {
        self.to_skip_struct_fields = to_skip_fields;
        self.rest_struct_var = rest_var;
        self
    }

    fn locate_next_seq_element(&mut self) {
        self.locate_end_seq_element();
        self.ser.current_location.push(self.current_element_index.to_string());
        self.current_element_index += 1;
    }

    fn locate_end_seq_element(&mut self) {
        if self.current_element_index > 0 {
            self.ser.current_location.pop();
        }
    }

    fn locate_next_map_key<T>(&mut self, key: &T) -> RsonSerResult<()>
    where
        T: ?Sized + Serialize,
    {
        self.locate_end_map_pair();
        self.ser.current_location.push(to_string_compact(key)?);
        self.current_element_index += 1;
        Ok(())
    }

    fn locate_next_map_value(&mut self) {
        self.ser.current_location.push(":".to_string());
    }

    fn locate_end_map_pair(&mut self) {
        if self.current_element_index > 0 {
            if let Some(last) = self.ser.current_location.last() {
                if last == ":" {
                    self.ser.current_location.pop();
                }
            }
            self.ser.current_location.pop();
        }
    }

    fn locate_next_struct_key(&mut self, key: &str) {
        self.locate_end_seq_element();
        self.ser.current_location.push(key.to_string());
        self.current_element_index += 1;
    }
}

impl<'a, W, F> SerializeSeq for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.ser
            .formatter
            .begin_array_value(&mut self.ser.writer, self.current_element_index == 0)?;

        self.locate_next_seq_element();
        if !self.ser.maybe_write_var()? {
            value.serialize(&mut *self.ser)?;
        }

        self.ser.formatter.end_array_value(&mut self.ser.writer)?;
        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.ser.formatter.end_array(&mut self.ser.writer)?;
        self.locate_end_seq_element();
        Ok(())
    }
}

impl<'a, W, F> SerializeTuple for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.ser.formatter.end_tuple(&mut self.ser.writer)?;
        self.locate_end_seq_element();
        Ok(())
    }
}

impl<'a, W, F> SerializeTupleStruct for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeTuple::end(self)
    }
}

impl<'a, W, F> SerializeTupleVariant for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeTuple::end(self)
    }
}

impl<'a, W, F> SerializeMap for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.ser
            .formatter
            .begin_struct_key(&mut self.ser.writer, self.current_element_index == 0)?;

        self.locate_next_map_key(key)?;
        if !self.ser.maybe_write_var()? {
            key.serialize(&mut *self.ser)?;
        }

        self.ser.formatter.end_struct_key(&mut self.ser.writer)?;
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.ser.formatter.begin_struct_value(&mut self.ser.writer)?;

        self.locate_next_map_value();
        if !self.ser.maybe_write_var()? {
            value.serialize(&mut *self.ser)?;
        }
        self.ser.formatter.end_struct_value(&mut self.ser.writer)?;
        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.ser.formatter.end_struct(&mut self.ser.writer)?;
        self.locate_end_map_pair();
        Ok(())
    }
}

impl<'a, W, F> SerializeStruct for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if self.to_skip_struct_fields.iter().any(|field| field == key) {
            return Ok(());
        }

        self.ser
            .formatter
            .begin_struct_key(&mut self.ser.writer, self.current_element_index == 0)?;

        self.locate_next_struct_key(key);

        let var_name = self.ser.find_var_name();
        if var_name.map(|name| name != key).unwrap_or(true) {
            self.ser.serialize_name(key)?;
            self.ser.formatter.end_struct_key(&mut self.ser.writer)?;
            self.ser.formatter.begin_struct_value(&mut self.ser.writer)?;
        }

        if !self.ser.maybe_write_var()? {
            value.serialize(&mut *self.ser)?;
        }
        self.ser.formatter.end_struct_value(&mut self.ser.writer)?;
        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if !self.to_skip_struct_fields.is_empty() {
            self.ser.formatter.rest_struct(
                &mut self.ser.writer,
                self.current_element_index == 0,
                self.rest_struct_var.as_deref().unwrap_or(""),
            )?;
        }
        self.ser.formatter.end_struct(&mut self.ser.writer)?;
        self.locate_end_seq_element();
        Ok(())
    }
}

impl<'a, W, F> SerializeStructVariant for RsonCompoundSerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = RsonSerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeStruct::end(self)
    }
}
