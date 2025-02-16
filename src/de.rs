use paste::paste;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use syn::{Expr, ExprCall, ExprLit, ExprPath, ExprStruct, ExprTuple, ExprUnary, Ident, Lit, Member, Path, UnOp};

use self::accessor::{RsonEnumAccessor, RsonFieldMapAccessor, RsonMapAccessor, RsonSeqAccessor};
use self::error::{RsonDeError, RsonDeResult};
use self::parser::{expected_func_call_args, is_expr_path_none, make_tuples_from_map_macro, RsonParser};

pub mod accessor;
pub mod error;
pub mod parser;

pub fn from_str<'de, T: Deserialize<'de>>(input: &str) -> RsonDeResult<T> {
    builder().from_str(input)?.deserialize()
}

pub fn builder() -> RsonDeBuilder {
    RsonDeBuilder::default()
}

#[derive(Debug, Default, Clone)]
pub struct RsonDeBuilder {
    parser: RsonParser,
}

impl RsonDeBuilder {
    pub fn new(parser: RsonParser) -> Self {
        Self { parser }
    }

    pub fn with_parser(mut self, parser: RsonParser) -> Self {
        self.parser = parser;
        self
    }

    pub fn from_str(mut self, input: impl AsRef<str>) -> RsonDeResult<Self> {
        let parser = RsonParser::from_str(input.as_ref())?;
        self.parser = parser;
        Ok(self)
    }

    pub fn deserialize<'de, T: Deserialize<'de>>(&self) -> RsonDeResult<T> {
        let deserializer = RsonDeserializer::new(&self.parser)?;
        let value = Deserialize::deserialize(deserializer)?;
        Ok(value)
    }

    pub fn deserialize_var<'de, T: Deserialize<'de>>(&self, var_name: impl AsRef<str>) -> RsonDeResult<T> {
        let var_name = var_name.as_ref();
        let expr = self
            .parser
            .get_var_expr(var_name)
            .ok_or_else(|| RsonDeError::VarNotFound(var_name.to_string()))?;

        let deserializer = RsonDeserializer::new_with_expr(&self.parser, expr);
        let value = Deserialize::deserialize(deserializer)?;
        Ok(value)
    }
}

#[derive(Clone, Copy)]
pub struct RsonDeserializer<'a> {
    parser: &'a RsonParser,
    current_expr: &'a Expr,
}

impl<'a> RsonDeserializer<'a> {
    pub fn new(parser: &'a RsonParser) -> RsonDeResult<Self> {
        if parser.is_empty() {
            return Err(RsonDeError::NoStatements);
        }

        let Some(current_expr) = parser.return_expr.as_ref() else {
            return Err(RsonDeError::ReturnExprNotFound);
        };

        Ok(Self { parser, current_expr })
    }

    pub fn new_with_expr(parser: &'a RsonParser, expr: &'a Expr) -> Self {
        Self {
            parser,
            current_expr: expr,
        }
    }

    pub fn duplicate_for_expr(&self, expr: &'a Expr) -> Self {
        Self::new_with_expr(self.parser, expr)
    }

    pub fn resolve_expr(&self, expr: &'a Expr) -> RsonDeResult<&Expr> {
        match expr {
            Expr::Field(field) => self.parser.find_field_init_expr(&field.base, &field.member),
            Expr::Path(path) => self.parser.find_var_init_expr(expr, &path.path),
            _ => Ok(expr),
        }
    }

    pub fn resolve_current_expr(&self) -> RsonDeResult<&Expr> {
        self.resolve_expr(self.current_expr)
    }

    pub fn resolve_struct_rest_fields(
        &self,
        mut fields: Vec<&str>,
        rest: &'a Expr,
    ) -> RsonDeResult<Vec<(&Ident, &Expr)>> {
        let mut found_fields = Vec::new();

        if fields.is_empty() {
            return Ok(found_fields);
        }

        let expr = self.resolve_expr(rest)?;
        match expr {
            Expr::Struct(struct_expr) => {
                for field in &struct_expr.fields {
                    if let Member::Named(ident) = &field.member {
                        let name = ident.to_string();

                        if let Some(idx) = fields.iter().position(|field| field == &name) {
                            found_fields.push((ident, &field.expr));
                            fields.remove(idx);
                        }
                    }
                }

                if let Some(rest) = &struct_expr.rest {
                    found_fields.extend(self.resolve_struct_rest_fields(fields, rest)?);
                }

                Ok(found_fields)
            },
            _ => Err(RsonDeError::Parse(format!(
                "Unsupported rest expr for {rest:?}, resolved as {expr:?}"
            ))),
        }
    }
}

macro_rules! deserialize_int_method {
    ($ty:ident) => {
        paste! {
            fn [<deserialize_ $ty>]<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                let expr = self.resolve_current_expr()?;
                match expr {
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(lit), ..
                    }) => return visitor.[<visit_ $ty>](lit.base10_parse()?),
                    Expr::Unary(ExprUnary {
                        op: UnOp::Neg(_), expr, ..
                    }) => {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Int(lit), ..
                        }) = expr.as_ref()
                        {
                            return visitor.[<visit_ $ty>](-lit.base10_parse()?);
                        }
                    },
                    _ => (),
                }
                Err(RsonDeError::Serde(format!(
                    "Unsupported {} type for {expr:?}", stringify!($ty)
                )))
            }
        }
    };
}

macro_rules! deserialize_uint_method {
    ($ty:ident) => {
        paste! {
            fn [<deserialize_ $ty>]<V>(self, visitor: V) -> Result<V::Value, Self::Error>
            where
                V: Visitor<'de>,
            {
                let expr = self.resolve_current_expr()?;
                match expr {
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(lit), ..
                    }) => visitor.[<visit_ $ty>](lit.base10_parse()?),
                    _ => Err(RsonDeError::Serde(format!(
                        "Unsupported {} type for {expr:?}", stringify!($ty)
                    ))),
                }
            }
        }
    };
}

impl<'a, 'de> Deserializer<'de> for RsonDeserializer<'a> {
    type Error = RsonDeError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Bool(lit), ..
            }) => visitor.visit_bool(lit.value),
            _ => Err(RsonDeError::Serde(format!("Unsupported bool type for {expr:?}"))),
        }
    }

    deserialize_int_method!(i8);
    deserialize_int_method!(i16);
    deserialize_int_method!(i32);
    deserialize_int_method!(i64);
    deserialize_int_method!(i128);

    deserialize_uint_method!(u8);
    deserialize_uint_method!(u16);
    deserialize_uint_method!(u32);
    deserialize_uint_method!(u64);
    deserialize_uint_method!(u128);

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;

        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Float(lit), ..
            }) => return visitor.visit_f32(lit.base10_parse()?),
            Expr::Unary(ExprUnary {
                op: UnOp::Neg(_), expr, ..
            }) => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Float(lit), ..
                }) = expr.as_ref()
                {
                    return visitor.visit_f32(-lit.base10_parse()?);
                }
            },
            _ => (),
        }
        Err(RsonDeError::Serde(format!("Unsupported f32 type for {expr:?}")))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Float(lit), ..
            }) => return visitor.visit_f64(lit.base10_parse()?),
            Expr::Unary(ExprUnary {
                op: UnOp::Neg(_), expr, ..
            }) => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Float(lit), ..
                }) = expr.as_ref()
                {
                    return visitor.visit_f64(-lit.base10_parse()?);
                }
            },
            _ => (),
        }
        Err(RsonDeError::Serde(format!("Unsupported f64 type for {expr:?}")))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Char(lit), ..
            }) => visitor.visit_char(lit.value()),
            _ => Err(RsonDeError::Serde(format!("Unsupported char type for {expr:?}"))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) => visitor.visit_str(&lit.value()),
            _ => Err(RsonDeError::Serde(format!("Unsupported str type for {expr:?}"))),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Lit(ExprLit { lit: Lit::Str(lit), .. }) => visitor.visit_string(lit.value()),
            _ => Err(RsonDeError::Serde(format!("Unsupported String type for {expr:?}"))),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::ByteStr(lit), ..
            }) => visitor.visit_byte_buf(lit.value()),
            Expr::Array(_) => self.deserialize_seq(visitor),
            _ => Err(RsonDeError::Serde(format!("Unsupported byte buf type for {expr:?}"))),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut expr = self.resolve_current_expr()?;
        if is_expr_path_none(expr) {
            visitor.visit_none()
        } else {
            if let Some(args) = expected_func_call_args(expr, "Some", 1) {
                expr = args[0];
            }
            let deserializer = self.duplicate_for_expr(expr);
            visitor.visit_some(deserializer)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Tuple(ExprTuple { elems, .. }) if elems.is_empty() => visitor.visit_unit(),
            Expr::Path(ExprPath { .. }) => visitor.visit_unit(),
            _ => Err(RsonDeError::Serde(format!("Unsupported unit type for {expr:?}"))),
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Path(ExprPath { path, .. })
                if path.get_ident().map(|ident| ident.to_string() == name).unwrap_or(false) =>
            {
                visitor.visit_unit()
            },
            _ => Err(RsonDeError::Serde(format!("Unsupported unit struct type for {expr:?}"))),
        }
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        if let Some(args) = expected_func_call_args(expr, name, 1) {
            let deserializer = self.duplicate_for_expr(args[0]);

            visitor.visit_newtype_struct(deserializer)
        } else {
            Err(RsonDeError::Serde(format!(
                "Unsupported newtype struct type for {expr:?}"
            )))
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Array(array) => visitor.visit_seq(RsonSeqAccessor::new(self.parser, array.elems.iter())),
            _ => Err(RsonDeError::Serde(format!("Unsupported sequence type for {expr:?}"))),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Tuple(tuple) => visitor.visit_seq(RsonSeqAccessor::new(self.parser, tuple.elems.iter())),
            _ => Err(RsonDeError::Serde(format!("Unsupported tuple type for {expr:?}"))),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        // name will be "" for enum tuple struct variant
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        if let Some(args) = expected_func_call_args(expr, name, len) {
            visitor.visit_seq(RsonSeqAccessor::new(self.parser, args))
        } else {
            Err(RsonDeError::Serde(format!(
                "Unsupported tuple struct type for {expr:?}"
            )))
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Macro(r#macro)
                if r#macro
                    .mac
                    .path
                    .get_ident()
                    .map(|ident| ident.to_string() == "map")
                    .unwrap_or(false) =>
            {
                let map_tuples = make_tuples_from_map_macro(r#macro.mac.tokens.clone());
                let parser = RsonParser::from_str(&map_tuples)?;
                let deserializer = RsonDeserializer::new(&parser)?;

                deserializer.deserialize_map(visitor)
            },
            Expr::Tuple(tuple) => visitor.visit_map(RsonMapAccessor::new(self.parser, tuple.elems.iter())),
            Expr::Array(array) => visitor.visit_map(RsonMapAccessor::new(self.parser, array.elems.iter())),
            _ => Err(RsonDeError::Serde(format!("Unsupported map type for {expr:?}"))),
        }
    }

    fn deserialize_struct<V>(
        self,
        // name will be "" for enum struct variant
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;
        match expr {
            Expr::Struct(struct_expr) => {
                if name != "" // No checking name for enum struct variant
                        && !struct_expr
                            .path
                            .get_ident()
                            .map(|ident| ident.to_string() == name)
                            .unwrap_or(false)
                {
                    return Err(RsonDeError::Serde(format!("Wrong struct type name for {expr:?}")));
                }

                let mut fields = fields.to_vec();
                let mut found_fields: Vec<_> = struct_expr
                    .fields
                    .iter()
                    .filter_map(|field| {
                        if let Member::Named(ident) = &field.member {
                            let name = ident.to_string();

                            if let Some(idx) = fields.iter().position(|field| *field == &name) {
                                fields.remove(idx);
                            }

                            Some((ident, &field.expr))
                        } else {
                            None
                        }
                    })
                    .collect();

                if !fields.is_empty() {
                    if let Some(rest_expr) = struct_expr.rest.as_deref() {
                        let rest_fields = self.resolve_struct_rest_fields(fields, rest_expr)?;

                        found_fields.extend(rest_fields);
                    }
                }

                visitor.visit_map(RsonFieldMapAccessor::new(self.parser, found_fields))
            },
            _ => Err(RsonDeError::Serde(format!("Unsupported struct type for {expr:?}"))),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let expr = self.resolve_current_expr()?;

        let visit_enum = |visitor: V, path: Option<&Path>| {
            if let Some(path) = path {
                if path.segments.len() > 1 {
                    let enum_name = path.segments[path.segments.len() - 2].ident.to_string();
                    if enum_name != name {
                        return Err(RsonDeError::Serde(format!(
                            "Wrong enum type name for {expr:?}, expexted `{name}`"
                        )));
                    }
                }
            }

            let deserializer = self.duplicate_for_expr(&expr);
            visitor.visit_enum(RsonEnumAccessor::new(deserializer))
        };

        match expr {
            Expr::Path(ExprPath { path, .. }) => visit_enum(visitor, Some(path)),
            Expr::Call(ExprCall { func, .. }) => {
                let path = if let Expr::Path(ExprPath { path, .. }) = func.as_ref() {
                    Some(path)
                } else {
                    None
                };
                visit_enum(visitor, path)
            },
            Expr::Struct(ExprStruct { path, .. }) => visit_enum(visitor, Some(path)),
            Expr::Lit(_) => visit_enum(visitor, None),
            _ => Err(RsonDeError::Serde(format!("Unsupported enum type for {expr:?}"))),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let visit_path = |visitor: V, path: &Path| {
            if path.segments.len() > 0 {
                let name = path.segments[path.segments.len() - 1].ident.to_string();
                Some(visitor.visit_str(name.as_str()))
            } else {
                None
            }
        };

        let expr = self.current_expr;
        let maybe_result = match expr {
            Expr::Path(ExprPath { path, .. }) => visit_path(visitor, path),
            Expr::Call(ExprCall { func, .. }) => {
                if let Expr::Path(ExprPath { path, .. }) = func.as_ref() {
                    visit_path(visitor, path)
                } else {
                    None
                }
            },
            Expr::Struct(ExprStruct { path, .. }) => visit_path(visitor, path),
            Expr::Lit(ExprLit { lit, .. }) => match lit {
                Lit::Str(lit_str) => Some(visitor.visit_str(&lit_str.value())),
                Lit::ByteStr(lit_byte_str) => Some(visitor.visit_bytes(&lit_byte_str.value())),
                Lit::Int(lit_int) => Some(visitor.visit_u64(lit_int.base10_parse()?)),
                _ => None,
            },
            _ => None,
        };

        maybe_result.unwrap_or_else(|| Err(RsonDeError::Serde(format!("Unsupported identifier type for {expr:?}"))))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}
