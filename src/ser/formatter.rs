use std::io;

pub trait Formatter {
    #[inline]
    fn begin_let<W>(&mut self, writer: &mut W, var_name: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"let ")?;
        writer.write_all(var_name.as_bytes())?;
        writer.write_all(b"=")
    }

    #[inline]
    fn end_let<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b";")
    }

    #[inline]
    fn write_none<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"None")
    }

    #[inline]
    fn write_unit<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"()")
    }

    #[inline]
    fn write_bool<W>(&mut self, writer: &mut W, value: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let buf: &[u8] = if value { b"true" as &[u8] } else { b"false" as &[u8] };
        writer.write_all(buf)
    }

    #[inline]
    fn write_int<W>(&mut self, writer: &mut W, value: i64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }

    #[inline]
    fn write_i128<W>(&mut self, writer: &mut W, value: i128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }

    #[inline]
    fn write_uint<W>(&mut self, writer: &mut W, value: u64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }

    #[inline]
    fn write_u128<W>(&mut self, writer: &mut W, value: u128) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(value);
        writer.write_all(s.as_bytes())
    }

    #[inline]
    fn write_f32<W>(&mut self, writer: &mut W, value: f32) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = ryu::Buffer::new();
        let s = buffer.format_finite(value);
        writer.write_all(s.as_bytes())
    }

    #[inline]
    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut buffer = ryu::Buffer::new();
        let s = buffer.format_finite(value);
        writer.write_all(s.as_bytes())
    }

    fn begin_some<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn end_some<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn begin_tuple<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"(")
    }

    #[inline]
    fn end_tuple<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b")")
    }

    #[inline]
    fn before_begin_string<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn after_end_string<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn write_raw<W>(&mut self, writer: &mut W, fragment: impl AsRef<[u8]>) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(fragment.as_ref())
    }

    #[inline]
    fn write_variant_name<W>(
        &mut self,
        writer: &mut W,
        _enum_name: &'static str,
        variant: &'static str,
    ) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.write_raw(writer, variant)
    }

    fn write_byte_array<W>(&mut self, writer: &mut W, value: &[u8]) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.begin_array(writer)?;
        let mut first = true;
        for byte in value {
            self.begin_array_value(writer, first)?;
            self.write_uint(writer, u64::from(*byte))?;
            self.end_array_value(writer)?;
            first = false;
        }
        self.end_array(writer)
    }

    #[inline]
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"[")
    }

    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"]")
    }

    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b",")
        }
    }

    #[inline]
    fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn before_begin_struct<W>(&mut self, writer: &mut W, name: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(name.as_bytes())
    }

    #[inline]
    fn begin_struct<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"{")
    }

    #[inline]
    fn end_struct<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"}")
    }

    #[inline]
    fn begin_struct_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first {
            Ok(())
        } else {
            writer.write_all(b",")
        }
    }

    #[inline]
    fn end_struct_key<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn begin_struct_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b":")
    }

    #[inline]
    fn end_struct_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        Ok(())
    }

    #[inline]
    fn rest_struct<W>(&mut self, writer: &mut W, is_empty_fields: bool, rest_var: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.begin_struct_key(writer, is_empty_fields)?;
        writer.write_all("..".as_bytes())?;
        writer.write_all(rest_var.as_bytes())
    }

    #[inline]
    fn before_begin_map<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"map!")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompactFormatter;

impl Formatter for CompactFormatter {}

#[derive(Clone, Copy, Debug)]
pub struct PrettyFormatter<'a> {
    current_indent: usize,
    indent: &'a [u8],
    has_value: bool,
    explicit_some: bool,
    full_enum_variant_name: bool,
    use_trailing_comma: bool,
}

impl<'a> Default for PrettyFormatter<'a> {
    fn default() -> Self {
        PrettyFormatter::new()
    }
}

impl<'a> PrettyFormatter<'a> {
    pub fn new() -> Self {
        PrettyFormatter::with_indent(b"    ")
    }

    pub fn with_indent(indent: &'a [u8]) -> Self {
        PrettyFormatter {
            current_indent: 0,
            indent,
            has_value: false,
            explicit_some: false,
            full_enum_variant_name: true,
            use_trailing_comma: true,
        }
    }

    pub fn with_explicit_some(mut self, explicit_some: bool) -> Self {
        self.explicit_some = explicit_some;
        self
    }

    pub fn with_full_enum_variant_name(mut self, full_enum_variant_name: bool) -> Self {
        self.full_enum_variant_name = full_enum_variant_name;
        self
    }

    pub fn with_use_trailing_comma(mut self, use_trailing_comma: bool) -> Self {
        self.use_trailing_comma = use_trailing_comma;
        self
    }
}

impl<'a> Formatter for PrettyFormatter<'a> {
    #[inline]
    fn begin_let<W>(&mut self, writer: &mut W, var_name: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"let ")?;
        writer.write_all(var_name.as_bytes())?;
        writer.write_all(b" = ")
    }

    #[inline]
    fn begin_some<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.explicit_some {
            writer.write_all(b"Some(")
        } else {
            Ok(())
        }
    }

    #[inline]
    fn end_some<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.explicit_some {
            writer.write_all(b")")
        } else {
            Ok(())
        }
    }

    #[inline]
    fn begin_tuple<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"(")
    }

    #[inline]
    fn end_tuple<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b")")
    }

    #[inline]
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"[")
    }

    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b"]")
    }

    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(if first || self.use_trailing_comma {
            b"\n"
        } else {
            b",\n"
        })?;
        indent(writer, self.current_indent, self.indent)
    }

    #[inline]
    fn end_array_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.use_trailing_comma {
            writer.write_all(b",")?;
        }
        self.has_value = true;
        Ok(())
    }

    #[inline]
    fn before_begin_struct<W>(&mut self, writer: &mut W, name: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(name.as_bytes())?;
        writer.write_all(b" ")
    }

    #[inline]
    fn begin_struct<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent += 1;
        self.has_value = false;
        writer.write_all(b"{")
    }

    #[inline]
    fn end_struct<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.current_indent -= 1;

        if self.has_value {
            writer.write_all(b"\n")?;
            indent(writer, self.current_indent, self.indent)?;
        }

        writer.write_all(b"}")
    }

    #[inline]
    fn begin_struct_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(if first || self.use_trailing_comma {
            b"\n"
        } else {
            b",\n"
        })?;
        indent(writer, self.current_indent, self.indent)
    }

    #[inline]
    fn begin_struct_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b": ")
    }

    #[inline]
    fn end_struct_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.use_trailing_comma {
            writer.write_all(b",")?;
        }
        self.has_value = true;
        Ok(())
    }

    #[inline]
    fn rest_struct<W>(&mut self, writer: &mut W, is_empty_fields: bool, rest_var: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.begin_struct_key(writer, is_empty_fields)?;
        writer.write_all("..".as_bytes())?;
        self.has_value = true;
        writer.write_all(rest_var.as_bytes())
    }

    #[inline]
    fn before_begin_map<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"map! ")
    }

    #[inline]
    fn write_variant_name<W>(
        &mut self,
        writer: &mut W,
        enum_name: &'static str,
        variant: &'static str,
    ) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if self.full_enum_variant_name {
            self.write_raw(writer, enum_name)?;
            self.write_raw(writer, b"::")?;
        }
        self.write_raw(writer, variant)
    }
}

fn indent<W>(writer: &mut W, n: usize, indent: &[u8]) -> io::Result<()>
where
    W: ?Sized + io::Write,
{
    for _ in 0..n {
        writer.write_all(indent)?;
    }

    Ok(())
}
