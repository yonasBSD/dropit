use axum::Extension;
use hyper::{Body, Request, Response, StatusCode};
// use routerify::ext::RequestExt;
use crate::alias::Alias;
use serde_json::{Map, Value};
use sqlx::SqlitePool;

use crate::error::revoke as RevokeError;
use crate::error::Error;
use crate::include_query;
use crate::response::{ApiResponse, ResponseType};
// use crate::response::json_response;
use crate::storage::dir::Dir;
use crate::update::AdminToken;

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: ResponseType,
    AdminToken(admin_token): AdminToken,
    alias: Alias,
    Extension(dir): Extension<Dir>,
) -> Result<ApiResponse<()>, ApiResponse<Error>> {
    process_revoke(pool, alias, admin_token, dir)
        .await
        .map_err(|err| response_type.to_api_response(err))?;
    Ok(response_type.to_api_response(()))
    // Ok(json_response(
    //     StatusCode::OK,
    //     process_revoke(pool, alias, admin_token, dir)
    //         .await
    //         .map(|_| Value::Object(Map::new()))?,
    // )?)
}

async fn process_revoke(
    pool: SqlitePool,
    alias: Alias,
    admin_token: String,
    dir: Dir,
) -> Result<(), Error> {
    let (id, _size, mut conn) = super::authorize(pool, &alias, &admin_token).await?;

    tokio::fs::remove_file(dir.file_path(&id))
        .await
        .map_err(|_| RevokeError::RemoveFile)?;

    sqlx::query(include_query!("delete_file"))
        .bind(&id)
        .execute(&mut conn)
        .await
        .map_err(|_| RevokeError::PartialRemove)?;
    Ok(())
}
