//! Enums stored in a `TEXT` column, as the value their `as_str` gives.
//!
//! The column keeps its `CHECK` constraint; in Rust the value is the enum, so
//! a status or an app can only be one of its variants.

/// Implements sqlx's `Type`, `Encode` and `Decode` for `$type`, an enum with
/// `const fn as_str(self) -> &'static str`, listing every variant so a stored
/// value reads back as the one whose `as_str` it is.
macro_rules! text_column {
    ($type:ident: $($variant:ident),+ $(,)?) => {
        impl sqlx::Type<sqlx::Postgres> for $type {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                <&str as sqlx::Type<sqlx::Postgres>>::type_info()
            }

            fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
                <&str as sqlx::Type<sqlx::Postgres>>::compatible(ty)
            }
        }

        impl sqlx::Encode<'_, sqlx::Postgres> for $type {
            fn encode_by_ref(
                &self,
                buf: &mut sqlx::postgres::PgArgumentBuffer,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <&str as sqlx::Encode<sqlx::Postgres>>::encode(self.as_str(), buf)
            }
        }

        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $type {
            fn decode(
                value: sqlx::postgres::PgValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let text = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
                [$($type::$variant),+]
                    .into_iter()
                    .find(|variant| variant.as_str() == text)
                    .ok_or_else(|| format!("unknown {} value: {text}", stringify!($type)).into())
            }
        }
    };
}

pub(crate) use text_column;
