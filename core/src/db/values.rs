use sea_orm::{
    sea_query::ArrayType,
    sea_query::{Value, ValueType, ValueTypeErr},
    ColumnType, QueryResult,
};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StringVec(pub Vec<String>);

impl std::convert::From<StringVec> for Value {
    fn from(source: StringVec) -> Self {
        let jarr: serde_json::Value = source.0.into();
        jarr.into()
    }
}

impl sea_orm::TryGetable for StringVec {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        <serde_json::Value as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            match serde_json::from_value(v) {
                Ok(jarr) => {
                    let arr: Vec<String> = jarr;
                    return Ok(StringVec(arr));
                }
                Err(err) => {
                    return Err(sea_orm::TryGetError::Null(err.to_string()));
                }
            }
        })
    }
}

impl ValueType for StringVec {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <serde_json::Value as ValueType>::try_from(v).and_then(|jarr| match serde_json::from_value(
            jarr,
        ) {
            Ok(jarr) => {
                let arr: Vec<String> = jarr;
                return Ok(StringVec(arr));
            }
            Err(_) => {
                return Err(ValueTypeErr);
            }
        })
    }

    fn type_name() -> String {
        stringify!(StringVec).to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> ColumnType {
        ColumnType::Json
    }
}
