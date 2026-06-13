#[cfg(feature = "async-graphql")]
use {
    async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value},
    bigdecimal::BigDecimal,
    std::str::FromStr,
};

/// GraphQL BigDecimal scalar that always serializes in plain notation.
///
/// async-graphql's built-in BigDecimal scalar uses `Display`, which can emit
/// scientific notation for very small/large values. The frontend expects plain
/// decimal strings, so this wrapper uses `to_plain_string()` on output.
#[cfg(feature = "async-graphql")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GraphqlBigDecimal(pub BigDecimal);

#[cfg(feature = "async-graphql")]
impl From<BigDecimal> for GraphqlBigDecimal {
    fn from(value: BigDecimal) -> Self {
        Self(value)
    }
}

#[cfg(feature = "async-graphql")]
#[Scalar(name = "BigDecimal")]
impl ScalarType for GraphqlBigDecimal {
    fn parse(value: Value) -> InputValueResult<Self> {
        let decimal = match &value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    BigDecimal::try_from(f).map_err(InputValueError::custom)?
                } else if let Some(i) = n.as_i64() {
                    BigDecimal::from(i)
                } else if let Some(u) = n.as_u64() {
                    BigDecimal::from(u)
                } else {
                    return Err(InputValueError::custom("Unsupported numeric value"));
                }
            },
            Value::String(s) => BigDecimal::from_str(s).map_err(InputValueError::custom)?,
            _ => return Err(InputValueError::expected_type(value)),
        };

        Ok(Self(decimal))
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_plain_string())
    }
}

#[cfg(all(test, feature = "async-graphql"))]
mod tests {
    use {
        super::GraphqlBigDecimal,
        async_graphql::{ScalarType, Value},
        bigdecimal::BigDecimal,
        std::str::FromStr,
    };

    #[test]
    fn serializes_small_values_without_scientific_notation() {
        let value =
            BigDecimal::from_str("1.971860244407904E-9").expect("scientific notation should parse");
        let graphql_value = GraphqlBigDecimal(value);

        let serialized = <GraphqlBigDecimal as ScalarType>::to_value(&graphql_value);
        assert_eq!(
            serialized,
            Value::String("0.000000001971860244407904".to_string())
        );
    }

    #[test]
    fn serializes_large_values_in_plain_decimal_notation() {
        let value =
            BigDecimal::from_str("4286068785875013.661991").expect("large decimal should parse");
        let graphql_value = GraphqlBigDecimal(value);

        let serialized = <GraphqlBigDecimal as ScalarType>::to_value(&graphql_value);
        assert_eq!(
            serialized,
            Value::String("4286068785875013.661991".to_string())
        );
    }
}
