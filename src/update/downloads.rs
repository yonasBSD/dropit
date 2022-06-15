use axum::extract::Path;
use axum::Extension;
use sqlx::SqlitePool;

use crate::alias::Alias;
use crate::error::downloads as DownloadsError;
use crate::error::Error;
use crate::include_query;
use crate::response::{ApiResponse, ResponseType};
use crate::update::AdminToken;

pub async fn handler(
    Extension(pool): Extension<SqlitePool>,
    response_type: ResponseType,
    AdminToken(admin_token): AdminToken,
    alias: Alias,
    Path((_, count)): Path<(String, u16)>,
) -> Result<ApiResponse<()>, ApiResponse<Error>> {
    process_downloads(pool, alias, admin_token, count)
        .await
        .map_err(|err| response_type.to_api_response(err))?;
    Ok(response_type.to_api_response(()))
}

async fn process_downloads(
    pool: SqlitePool,
    alias: Alias,
    admin_token: String,
    count: u16,
) -> Result<(), Error> {
    let (id, _size, mut conn) = super::authorize(pool, &alias, &admin_token).await?;
    let count = if count >= 1 { Some(count) } else { None };

    sqlx::query(include_query!("update_file_downloads"))
        .bind(count)
        .bind(&id)
        .execute(&mut conn)
        .await
        .map_err(|_| DownloadsError::UnexpectedFileModification)?;

    Ok(())
}
