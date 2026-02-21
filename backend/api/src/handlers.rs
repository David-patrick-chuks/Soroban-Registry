use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};

use crate::state::AppState;

pub type ApiResult<T> = Result<T, (StatusCode, String)>;

pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();

    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    if db_ok {
        tracing::info!(uptime_secs = uptime, "health check passed");

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "version": "0.1.0",
                "timestamp": now,
                "uptime_secs": uptime
            })),
        )
    } else {
        tracing::warn!(
            uptime_secs = uptime,
            "health check degraded — db unreachable"
        );

        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "degraded",
                "version": "0.1.0",
                "timestamp": now,
                "uptime_secs": uptime
            })),
        )
    }
}

/// Get registry statistics
pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count contracts", err))?;

    let verified_contracts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE is_verified = true")
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("count verified contracts", err))?;

    let total_publishers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publishers")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count publishers", err))?;

    Ok(Json(serde_json::json!({
        "total_contracts": total_contracts,
        "verified_contracts": verified_contracts,
        "total_publishers": total_publishers
    })))
}

        "total_publishers": total_publishers,
    })))
}

/// List and search contracts
pub async fn list_contracts(
    State(state): State<AppState>,
    params: Result<Query<ContractSearchParams>, QueryRejection>,
) -> axum::response::Response {
    let Query(params) = match params {
        Ok(q) => q,
        Err(err) => return map_query_rejection(err).into_response(),
    };
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1).max(0) * limit;

    let contracts: Vec<Contract> = match sqlx::query_as(
        "SELECT * FROM contracts ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(err) => return db_internal_error("list contracts", err).into_response(),
    };
    let contracts: Vec<Contract> =
        match sqlx::query_as("SELECT * FROM contracts ORDER BY created_at DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
        {
            Ok(rows) => rows,
            Err(err) => return db_internal_error("list contracts", err).into_response(),
        };
    let total: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
    {
        Ok(v) => v,
        Err(err) => return db_internal_error("count contracts", err).into_response(),
    };
    (StatusCode::OK, Json(PaginatedResponse::new(contracts, total, page, limit))).into_response()
    (
        StatusCode::OK,
        Json(PaginatedResponse::new(contracts, total, page, limit)),
    )
        .into_response()

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);

    // bad input, bail early
    if page < 1 || limit < 1 || limit > 100 {
        return ApiError::bad_request(
            "InvalidPagination",
            "page must be >= 1 and limit must be between 1 and 100",
        )
        .into_response();
    }

    let offset = (page - 1) * limit;

    // Build dynamic query based on filters
    let mut query = String::from("SELECT * FROM contracts WHERE 1=1");
    let mut count_query = String::from("SELECT COUNT(*) FROM contracts WHERE 1=1");

    if let Some(ref q) = params.query {
        let search_clause = format!(" AND (name ILIKE '%{}%' OR description ILIKE '%{}%')", q, q);
        query.push_str(&search_clause);
        count_query.push_str(&search_clause);
    }

    if let Some(verified) = params.verified_only {
        if verified {
            query.push_str(" AND is_verified = true");
            count_query.push_str(" AND is_verified = true");
        }
    }

    if let Some(ref category) = params.category {
        let category_clause = format!(" AND category = '{}'", category);
        query.push_str(&category_clause);
        count_query.push_str(&category_clause);
    }

    query.push_str(&format!(
        " ORDER BY created_at DESC LIMIT {} OFFSET {}",
        limit, offset
    ));

    let contracts: Vec<Contract> = match sqlx::query_as(&query).fetch_all(&state.db).await {
        Ok(rows) => rows,
        Err(err) => return db_internal_error("list contracts", err).into_response(),
    };

    let total: i64 = match sqlx::query_scalar(&count_query).fetch_one(&state.db).await {
        Ok(n) => n,
        Err(err) => return db_internal_error("count filtered contracts", err).into_response(),
    };

    let paginated = PaginatedResponse::new(contracts, total, page, limit);

    // link headers for pagination
    let total_pages = paginated.total_pages;
    let mut links: Vec<String> = Vec::new();

    if page > 1 {
        links.push(format!(
            "</api/contracts?page={}&limit={}>; rel=\"prev\"",
            page - 1,
            limit
        ));
    }
    if page < total_pages {
        links.push(format!(
            "</api/contracts?page={}&limit={}>; rel=\"next\"",
            page + 1,
            limit
        ));
    }

    let mut response = (StatusCode::OK, Json(paginated)).into_response();

    if !links.is_empty() {
        if let Ok(value) = axum::http::HeaderValue::from_str(&links.join(", ")) {
            response.headers_mut().insert("link", value);
        }
    }

    response
    Ok(Json(PaginatedResponse::new(
        contracts, total, page, page_size,
    )))
}

pub async fn get_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Contract>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;
    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Contract>> {
    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("get contract", err),
        })?;
    Ok(Json(contract))
}

pub async fn get_contract_abi(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let abi: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT abi FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    abi.map(Json).ok_or(StatusCode::NOT_FOUND)
}

            _ => db_internal_error("get contract by id", err),
        })?;

    let active_deployment: Option<ContractDeployment> = sqlx::query_as(
        "SELECT * FROM contract_deployments
         WHERE contract_id = $1 AND status = 'active'",
    )
    .bind(contract.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("get active deployment", err))?;

    if let Some(deployment) = active_deployment {
        let mut contract_with_deployment = contract.clone();
        contract_with_deployment.wasm_hash = deployment.wasm_hash;
        Ok(Json(contract_with_deployment))
        (StatusCode::OK, Json(json!({"status": "ok", "version": "0.1.0", "timestamp": now, "uptime_secs": uptime})))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"status": "degraded", "version": "0.1.0", "timestamp": now, "uptime_secs": uptime})))
    }
}

pub async fn list_contracts() -> impl IntoResponse {
    Json(json!({"contracts": []}))
}

pub async fn get_contract() -> impl IntoResponse {
    Json(json!({"contract": null}))
}

pub async fn get_contract_abi() -> impl IntoResponse {
    Json(json!({"abi": null}))
}

pub async fn get_contract_versions() -> impl IntoResponse {
    Json(json!({"versions": []}))
}

pub async fn get_contract_state() -> impl IntoResponse {
    Json(json!({"state": {}}))
}

pub async fn update_contract_state() -> impl IntoResponse {
    Json(json!({"success": true}))
}

pub async fn get_contract_analytics() -> impl IntoResponse {
    Json(json!({"analytics": {}}))
}

pub async fn get_trust_score() -> impl IntoResponse {
    Json(json!({"score": 0}))
}

pub async fn get_contract_dependencies() -> impl IntoResponse {
    Json(json!({"dependencies": []}))
}

pub async fn get_contract_dependents() -> impl IntoResponse {
    Json(json!({"dependents": []}))
}

pub async fn get_contract_graph() -> impl IntoResponse {
    Json(json!({"graph": {}}))
}

pub async fn get_trending_contracts() -> impl IntoResponse {
    Json(json!({"trending": []}))
}

pub async fn publish_contract() -> impl IntoResponse {
    Json(json!({"success": true}))
}

pub async fn verify_contract() -> impl IntoResponse {
    Json(json!({"verified": true}))
}

pub async fn get_deployment_status() -> impl IntoResponse {
    Json(json!({"status": "pending"}))
}

pub async fn deploy_green() -> impl IntoResponse {
    Json(json!({"deployment_id": ""}))
}

pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    Ok(Json(json!({"total_contracts": total_contracts, "verified_contracts": 0, "total_downloads": 0})))
}

pub async fn route_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, Json(json!({"error": "Route not found"})))
}
