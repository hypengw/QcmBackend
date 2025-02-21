use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(SqlTable, attributes(table, column))]
pub fn sql_table(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    let table_name = input.attrs.iter()
        .find(|attr| attr.path().is_ident("table"))
        .map(|attr| attr.parse_args::<String>().unwrap())
        .unwrap_or_else(|| name.to_string().to_lowercase());

    // Get field names for select query
    let fields = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => {
                    fields.named.iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect::<Vec<_>>()
                }
                _ => vec![],
            }
        }
        _ => vec![],
    };

    let field_list = fields.iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let expanded = quote! {
        impl #name {
            pub async fn find_by_id(pool: &sqlx::SqlitePool, id: i64) -> anyhow::Result<Option<Self>> {
                sqlx::query_as!(
                    Self,
                    r#"SELECT * FROM {} WHERE id = ?"#,
                    #table_name,
                    id
                )
                .fetch_optional(pool)
                .await
                .map_err(|e| e.into())
            }

            pub async fn find_all(pool: &sqlx::SqlitePool) -> anyhow::Result<Vec<Self>> {
                sqlx::query_as!(
                    Self,
                    r#"SELECT {} FROM {}"#,
                    #field_list,
                    #table_name
                )
                .fetch_all(pool)
                .await
                .map_err(|e| e.into())
            }

            pub async fn find_by_library_id(pool: &sqlx::SqlitePool, library_id: i64) -> anyhow::Result<Vec<Self>> {
                sqlx::query_as!(
                    Self,
                    r#"SELECT {} FROM {} WHERE library_id = ?"#,
                    #field_list,
                    #table_name,
                    library_id
                )
                .fetch_all(pool)
                .await
                .map_err(|e| e.into())
            }

            pub async fn insert(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
                let fields = #field_list;
                let placeholders = std::iter::repeat("?")
                    .take(fields.split(",").count())
                    .collect::<Vec<_>>()
                    .join(",");
                
                sqlx::query(&format!(
                    r#"INSERT INTO {} ({}) VALUES ({})"#,
                    #table_name,
                    fields,
                    placeholders
                ))
                #(.bind(&self.#fields))*
                .execute(pool)
                .await?;
                
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}
