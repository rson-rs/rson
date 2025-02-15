//! Provides pretty serialization with `to_string`.

use serde::ser::Serialize;

use super::{Pretty, Result, Serializer};

/// Serializes `value` in the recommended RSON layout.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut s = Serializer {
        output: String::new(),
        pretty: Some(Pretty { indent: 0 }),
        struct_names: false,
    };
    value.serialize(&mut s)?;
    Ok(s.output)
}
