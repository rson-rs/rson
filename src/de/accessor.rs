use serde::de::{DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess};
use serde::{Deserialize, Deserializer};
use syn::{Expr, ExprCall, ExprPath, Ident};

use super::error::{RsonDeError, RsonDeResult};
use super::parser::RsonParser;
use super::RsonDeserializer;

pub struct RsonSeqAccessor<'a> {
    parser: &'a RsonParser,
    elems: Vec<&'a Expr>,
    index: usize,
}

impl<'a> RsonSeqAccessor<'a> {
    pub fn new(parser: &'a RsonParser, elems: impl IntoIterator<Item = &'a Expr>) -> Self {
        Self {
            parser,
            elems: elems.into_iter().collect(),
            index: 0,
        }
    }
}

impl<'a, 'de> SeqAccess<'de> for RsonSeqAccessor<'a> {
    type Error = RsonDeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if let Some(elem) = self.elems.get(self.index) {
            let expr = check_and_resolve_expr(&self.parser, elem)?
                .ok_or_else(|| RsonDeError::Serde(format!("Unsupported sequence type for {elem:?}")))?;

            let deserializer = RsonDeserializer::new_with_expr(self.parser, expr);
            self.index += 1;
            Ok(Some(seed.deserialize(deserializer)?))
        } else {
            Ok(None)
        }
    }
}

pub struct RsonMapAccessor<'a> {
    parser: &'a RsonParser,
    elems: Vec<&'a Expr>,
    index: usize,
}

impl<'a> RsonMapAccessor<'a> {
    pub fn new(parser: &'a RsonParser, elems: impl IntoIterator<Item = &'a Expr>) -> Self {
        Self {
            parser,
            elems: elems.into_iter().collect(),
            index: 0,
        }
    }
}

impl<'a, 'de> MapAccess<'de> for RsonMapAccessor<'a> {
    type Error = RsonDeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some(elem) = self.elems.get(self.index) {
            if let Expr::Tuple(tuple) = elem {
                if tuple.elems.len() == 2 {
                    let key_expr = &tuple.elems[0];
                    let key_expr = check_and_resolve_expr(&self.parser, key_expr)?
                        .ok_or_else(|| RsonDeError::Serde(format!("Unsupported map key type for {key_expr:?}")))?;

                    let deserializer = RsonDeserializer::new_with_expr(self.parser, key_expr);
                    return Ok(Some(seed.deserialize(deserializer)?));
                }
            }

            Err(RsonDeError::Serde(format!("Unsupported map type for {elem:?}")))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let elem = &self.elems[self.index];
        if let Expr::Tuple(tuple) = elem {
            let val_expr = &tuple.elems[1];
            let val_expr = check_and_resolve_expr(&self.parser, val_expr)?
                .ok_or_else(|| RsonDeError::Serde(format!("Unsupported map val type for {val_expr:?}")))?;

            let deserializer = RsonDeserializer::new_with_expr(self.parser, val_expr);
            self.index += 1;

            Ok(seed.deserialize(deserializer)?)
        } else {
            Err(RsonDeError::Serde(format!("Unsupported map type for {elem:?}")))
        }
    }
}

pub struct RsonFieldMapAccessor<'a> {
    parser: &'a RsonParser,
    elems: Vec<(&'a Ident, &'a Expr)>,
    index: usize,
}

impl<'a> RsonFieldMapAccessor<'a> {
    pub fn new(parser: &'a RsonParser, elems: impl IntoIterator<Item = (&'a Ident, &'a Expr)>) -> Self {
        Self {
            parser,
            elems: elems.into_iter().collect(),
            index: 0,
        }
    }
}

impl<'a, 'de> MapAccess<'de> for RsonFieldMapAccessor<'a> {
    type Error = RsonDeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some((field, _)) = self.elems.get(self.index) {
            let expr = Expr::Path(ExprPath {
                attrs: vec![],
                qself: None,
                path: (*field).clone().into(),
            });
            let deserializer = RsonDeserializer::new_with_expr(self.parser, &expr);
            Ok(Some(seed.deserialize(deserializer)?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (_, val_expr) = &self.elems[self.index];
        let val_expr = check_and_resolve_expr(&self.parser, val_expr)?
            .ok_or_else(|| RsonDeError::Serde(format!("Unsupported field map val type for {val_expr:?}")))?;

        let deserializer = RsonDeserializer::new_with_expr(self.parser, val_expr);
        self.index += 1;

        Ok(seed.deserialize(deserializer)?)
    }
}

pub struct RsonEnumAccessor<'a> {
    deserializer: RsonDeserializer<'a>,
}

impl<'a> RsonEnumAccessor<'a> {
    pub fn new(deserializer: RsonDeserializer<'a>) -> Self {
        Self { deserializer }
    }
}

impl<'a, 'de> EnumAccess<'de> for RsonEnumAccessor<'a> {
    type Error = RsonDeError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = seed.deserialize(self.deserializer)?;
        Ok((value, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for RsonEnumAccessor<'a> {
    type Error = RsonDeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Deserialize::deserialize(self.deserializer)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let expr = self.deserializer.current_expr;
        match expr {
            Expr::Call(ExprCall { args, .. }) if args.len() == 1 => {
                return seed.deserialize(self.deserializer.duplicate_for_expr(&args[0]));
            },
            _ => (),
        }
        Err(RsonDeError::Serde(format!(
            "Unsupported newtupe enum variant for {expr:?}"
        )))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserializer.deserialize_tuple_struct("", len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserializer.deserialize_struct("", fields, visitor)
    }
}

pub fn check_and_resolve_expr<'a>(parser: &'a RsonParser, expr: &'a Expr) -> RsonDeResult<Option<&'a Expr>> {
    Ok(Some(match expr {
        Expr::Array(_)
        | Expr::Call(_)
        | Expr::Lit(_)
        | Expr::Macro(_)
        | Expr::Range(_)
        | Expr::Repeat(_)
        | Expr::Struct(_)
        | Expr::Tuple(_)
        | Expr::Unary(_) => expr,
        Expr::Field(field) => parser.find_field_init_expr(&field.base, &field.member)?,
        Expr::Path(path) => parser.find_var_init_expr(expr, &path.path)?,
        _ => return Ok(None),
    }))
}
