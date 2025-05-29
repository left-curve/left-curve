#[cfg(feature = "async-graphql")]
use {
    async_graphql::{
        InputType, InputValueResult, OutputType, Positioned, ServerResult,
        context::ContextSelectionSet, parser::types::Field, registry::Registry,
    },
    std::borrow::Cow,
};
use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::{Inner, InnerMut},
    serde::{Deserialize, Serialize},
    serde_json::{Map, Number, Value as JsonValue},
    std::{
        io,
        ops::{Deref, DerefMut},
    },
};

const TAG_NULL: u8 = 0;
const TAG_BOOL: u8 = 1;
const TAG_NUMBER: u8 = 2;
const TAG_STRING: u8 = 3;
const TAG_ARRAY: u8 = 4;
const TAG_OBJECT: u8 = 5;

const TAG_NON_NEG_INT: u8 = 0;
const TAG_NEG_INT: u8 = 1;
const TAG_NON_INT: u8 = 2;

/// Construct a [`Json`](crate::Json) from a JSON literal.
///
/// This is simply a wrapper over the [`serde_json::json`](serde_json::json) macro.
#[macro_export]
macro_rules! json {
    ($($json:tt)+) => {
        $crate::Json::from_inner($crate::__private::serde_json::json!($($json)+))
    };
}

/// A wrapper over [`serde_json::Value`](serde_json::Value) that implements
/// [Borsh](https://github.com/near/borsh-rs) traits.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Json(JsonValue);

impl Json {
    pub const fn null() -> Self {
        Self(JsonValue::Null)
    }

    pub fn from_inner(inner: JsonValue) -> Self {
        Self(inner)
    }
}

impl Inner for Json {
    type U = JsonValue;

    fn inner(&self) -> &Self::U {
        &self.0
    }

    fn into_inner(self) -> Self::U {
        self.0
    }
}

impl InnerMut for Json {
    fn inner_mut(&mut self) -> &mut Self::U {
        &mut self.0
    }
}

impl Deref for Json {
    type Target = JsonValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Json {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl BorshSerialize for Json {
    fn serialize<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        serialize_value(&self.0, writer)
    }
}

impl BorshDeserialize for Json {
    fn deserialize_reader<R>(reader: &mut R) -> io::Result<Self>
    where
        R: io::Read,
    {
        deserialize_value(reader).map(Self)
    }
}

fn serialize_value<W>(value: &JsonValue, writer: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    match value {
        JsonValue::Null => BorshSerialize::serialize(&TAG_NULL, writer),
        JsonValue::Bool(b) => {
            BorshSerialize::serialize(&TAG_BOOL, writer)?;
            BorshSerialize::serialize(b, writer)
        },
        JsonValue::Number(n) => {
            BorshSerialize::serialize(&TAG_NUMBER, writer)?;
            serialize_number(n, writer)
        },
        JsonValue::String(s) => {
            BorshSerialize::serialize(&TAG_STRING, writer)?;
            BorshSerialize::serialize(s, writer)
        },
        JsonValue::Array(a) => {
            BorshSerialize::serialize(&TAG_ARRAY, writer)?;
            serialize_array(a, writer)
        },
        JsonValue::Object(o) => {
            BorshSerialize::serialize(&TAG_OBJECT, writer)?;
            serialize_object(o, writer)
        },
    }
}

// A JSON number can either be a non-negative integer (represented in
// serde_json by a u64), a negative integer (by an i64), or a non-integer
// (by an f64).
//
// We identify these cases with the following single-byte discriminants:
// 0 - u64
// 1 - i64
// 2 - f64
#[inline]
fn serialize_number<W>(number: &Number, writer: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    if let Some(u) = number.as_u64() {
        BorshSerialize::serialize(&TAG_NON_NEG_INT, writer)?;
        return BorshSerialize::serialize(&u, writer);
    }

    if let Some(i) = number.as_i64() {
        BorshSerialize::serialize(&TAG_NEG_INT, writer)?;
        return BorshSerialize::serialize(&i, writer);
    }

    if let Some(f) = number.as_f64() {
        BorshSerialize::serialize(&TAG_NON_INT, writer)?;
        return BorshSerialize::serialize(&f, writer);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "json number is neither u64, i64, nor f64",
    ))
}

#[inline]
fn serialize_array<W>(array: &Vec<JsonValue>, writer: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    // Assume the array isn't longer than `u32::MAX` elements.
    BorshSerialize::serialize(&(array.len() as u32), writer)?;

    for value in array {
        serialize_value(value, writer)?;
    }

    Ok(())
}

#[inline]
fn serialize_object<W>(object: &Map<String, JsonValue>, writer: &mut W) -> io::Result<()>
where
    W: io::Write,
{
    // Assume the object doesn't have more than `u32::MAX` keys.
    BorshSerialize::serialize(&(object.len() as u32), writer)?;

    for (key, value) in object {
        BorshSerialize::serialize(key, writer)?;
        serialize_value(value, writer)?;
    }

    Ok(())
}

fn deserialize_value<R>(reader: &mut R) -> io::Result<JsonValue>
where
    R: io::Read,
{
    let tag = u8::deserialize_reader(reader)?;

    match tag {
        TAG_NULL => Ok(JsonValue::Null),
        TAG_BOOL => {
            let b = bool::deserialize_reader(reader)?;
            Ok(JsonValue::Bool(b))
        },
        TAG_NUMBER => {
            let n = deserialize_number(reader)?;
            Ok(JsonValue::Number(n))
        },
        TAG_STRING => {
            let s = String::deserialize_reader(reader)?;
            Ok(JsonValue::String(s))
        },
        TAG_ARRAY => {
            let a = deserialize_array(reader)?;
            Ok(JsonValue::Array(a))
        },
        TAG_OBJECT => {
            let o = deserialize_object(reader)?;
            Ok(JsonValue::Object(o))
        },
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid json tag: {tag}, must be 0-5"),
        )),
    }
}

#[inline]
fn deserialize_number<R>(reader: &mut R) -> io::Result<Number>
where
    R: io::Read,
{
    let tag = u8::deserialize_reader(reader)?;

    match tag {
        TAG_NON_NEG_INT => {
            let u = u64::deserialize_reader(reader)?;
            Ok(Number::from(u))
        },
        TAG_NEG_INT => {
            let i = i64::deserialize_reader(reader)?;
            Ok(Number::from(i))
        },
        TAG_NON_INT => {
            let f = f64::deserialize_reader(reader)?;
            // This returns None if the number is a NaN or +/-Infinity, which
            // are not valid JSON numbers.
            Number::from_f64(f).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid json number: {f}")
            })
        },
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid number tag: {tag}, must be 0-2"),
        )),
    }
}

#[inline]
fn deserialize_array<R>(reader: &mut R) -> io::Result<Vec<JsonValue>>
where
    R: io::Read,
{
    let len = u32::deserialize_reader(reader)?;
    let mut array = Vec::with_capacity(len as usize);

    for _ in 0..len {
        let value = deserialize_value(reader)?;
        array.push(value);
    }

    Ok(array)
}

#[inline]
fn deserialize_object<R>(reader: &mut R) -> io::Result<Map<String, JsonValue>>
where
    R: io::Read,
{
    let len = u32::deserialize_reader(reader)?;
    let mut object = Map::with_capacity(len as usize);

    for _ in 0..len {
        let key = String::deserialize_reader(reader)?;
        let value = deserialize_value(reader)?;
        object.insert(key, value);
    }

    Ok(object)
}

#[cfg(feature = "async-graphql")]
impl InputType for Json {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        "JSON".into()
    }

    fn create_type_info(_registry: &mut Registry) -> String {
        "JSON".to_string()
    }

    fn parse(value: Option<async_graphql::Value>) -> InputValueResult<Self> {
        async_graphql::types::Json::<JsonValue>::parse(value)
            .map(|json| Json(json.0))
            .map_err(|e| e.propagate())
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::types::Json(&self.0).to_value()
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
}

#[cfg(feature = "async-graphql")]
impl OutputType for Json {
    fn type_name() -> Cow<'static, str> {
        "JSON".into()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        <async_graphql::types::Json<JsonValue> as OutputType>::create_type_info(registry)
    }

    async fn resolve(
        &self,
        ctx: &ContextSelectionSet<'_>,
        field: &Positioned<Field>,
    ) -> ServerResult<async_graphql::Value> {
        async_graphql::types::Json(self.0.clone())
            .resolve(ctx, field)
            .await
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{BorshDeExt, BorshSerExt, Json, JsonDeExt, JsonSerExt, json},
        borsh::{BorshDeserialize, BorshSerialize},
        serde::{Deserialize, Serialize},
    };

    /// A struct that contains a `Json`. Used to ensure serializing `Json` works
    /// even if it's part of another struct.
    #[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
    struct ComplexStruct {
        pub json: Json,
    }

    #[test]
    fn serialization_works() {
        let original = json!({
            "null": null,
            "true": true,
            "false": false,
            "zero": 0,
            "positive_integer": 12345,
            "negative_integer": -88888,
            "positive_float": 123.45,
            "negative_float": -888.88,
            "positive_max": 1.7976931348623157e308,
            "negative_max": -1.7976931348623157e308,
            "string": "Larry",
            "array_of_nulls": [null, null, null],
            "array_of_numbers": [0, -1, 1, 1.1, -1.1, 34798324],
            "array_of_strings": ["Larry", "Jake", "Pumpkin"],
            "array_of_arrays": [
                [1, 2, 3],
                [4, 5, 6],
                [7, 8, 9]
            ],
            "array_of_objects": [
                {
                    "name": "Larry",
                    "age": 30
                },
                {
                    "name": "Jake",
                    "age": 7
                },
                {
                    "name": "Pumpkin",
                    "age": 8
                }
            ],
            "object": {
                "name": "Larry",
                "age": 30,
                "pets": [
                    {
                        "name": "Jake",
                        "age": 7
                    },
                    {
                        "name": "Pumpkin",
                        "age": 8
                    }
                ]
            }
        });

        // JSON serialization
        {
            let serialized = original.to_json_vec().unwrap();
            let deserialized = serialized.deserialize_json::<Json>().unwrap();

            assert_eq!(original, deserialized);
        }

        // Borsh serialization
        {
            let serialized = original.to_borsh_vec().unwrap();
            let deserialized = serialized.deserialize_borsh::<Json>().unwrap();

            assert_eq!(original, deserialized);
        }

        let complex = ComplexStruct { json: original };

        // JSON serialization in a complex struct
        {
            let serialized = complex.to_json_vec().unwrap();
            let deserialized = serialized.deserialize_json::<ComplexStruct>().unwrap();

            assert_eq!(complex, deserialized);
        }

        // Borsh serialization in a complex struct
        {
            let serialized = complex.to_borsh_vec().unwrap();
            let deserialized = serialized.deserialize_borsh::<ComplexStruct>().unwrap();

            assert_eq!(complex, deserialized);
        }
    }
}
