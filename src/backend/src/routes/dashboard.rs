//! Role-based dashboard route.
//!
//! Implements `GET /api/v1/dashboard` (Story 6.1, extended in 6.3/6.4). The
//! handler does only authentication, query validation, and role gating; the
//! per-role data assembly lives in `services::dashboard_service::build_dashboard`.

use axum::{
    extract::{RawQuery, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;

use crate::error::{AppError, Result};
use crate::services::audit_log::user_claims_from_headers;
use crate::services::dashboard_service::{
    build_dashboard, DepartmentHeadTeamRange, RoleDashboardResponse, DH_TEAM_RANGE_DEFAULT_DAYS,
    DH_TEAM_RANGE_MAX_DAYS,
};

/// Optional query parameters for the role dashboard.
///
/// Only the Department Head role consumes the team range fields (Story 6.4);
/// other roles ignore them. Both fields default to `None`, in which case the
/// service applies the documented operational window starting today.
#[derive(Debug, Default)]
pub struct DashboardQuery {
    pub team_start_date: Option<String>,
    pub team_end_date: Option<String>,
}

pub fn dashboard_routes() -> Router<PgPool> {
    Router::new().route("/dashboard", get(get_role_dashboard))
}

async fn get_role_dashboard(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    RawQuery(raw_query): RawQuery,
) -> Result<Json<RoleDashboardResponse>> {
    let claims = user_claims_from_headers(&headers)?
        .ok_or_else(|| AppError::Authentication("Missing token".to_string()))?;

    let dashboard_date = Utc::now().date_naive();
    let team_range = if claims.role == "department_head" {
        let query = parse_dashboard_query(raw_query.as_deref())?;
        resolve_team_range(
            query.team_start_date.as_deref(),
            query.team_end_date.as_deref(),
            dashboard_date,
        )?
    } else {
        default_team_range(dashboard_date)
    };
    let response = build_dashboard(&pool, &headers, &claims, team_range, dashboard_date).await?;
    Ok(Json(response))
}

fn parse_dashboard_query(raw_query: Option<&str>) -> Result<DashboardQuery> {
    let mut query = DashboardQuery::default();
    let Some(raw_query) = raw_query else {
        return Ok(query);
    };

    for pair in raw_query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "team_start_date" => {
                set_unique_query_value("team_start_date", &mut query.team_start_date, value)?
            }
            "team_end_date" => {
                set_unique_query_value("team_end_date", &mut query.team_end_date, value)?
            }
            _ => {}
        }
    }

    Ok(query)
}

fn set_unique_query_value(field: &str, slot: &mut Option<String>, value: &str) -> Result<()> {
    if slot.is_some() {
        return Err(AppError::Validation(format!(
            "{} may only be provided once",
            field
        )));
    }
    *slot = Some(value.to_string());
    Ok(())
}

fn default_team_range(today: NaiveDate) -> DepartmentHeadTeamRange {
    let default_end = today
        .checked_add_signed(chrono::Duration::days(DH_TEAM_RANGE_DEFAULT_DAYS))
        .unwrap_or(today);

    DepartmentHeadTeamRange {
        start_date: today,
        end_date: default_end,
    }
}

/// Resolve and validate the Department Head team trend range from optional
/// query params. Validation is enforced at the route boundary so the service
/// layer can assume a coherent, bounded range.
fn resolve_team_range(
    start: Option<&str>,
    end: Option<&str>,
    today: NaiveDate,
) -> Result<DepartmentHeadTeamRange> {
    let start_date = parse_team_range_date("team_start_date", start)?.unwrap_or(today);
    let default_end = start_date
        .checked_add_signed(chrono::Duration::days(DH_TEAM_RANGE_DEFAULT_DAYS))
        .unwrap_or(start_date);
    let end_date = parse_team_range_date("team_end_date", end)?.unwrap_or(default_end);

    if start_date > end_date {
        return Err(AppError::Validation(
            "team_start_date cannot be after team_end_date".to_string(),
        ));
    }

    let span_days = (end_date - start_date).num_days() + 1;
    if span_days > DH_TEAM_RANGE_MAX_DAYS {
        return Err(AppError::Validation(format!(
            "team trend range cannot exceed {} days",
            DH_TEAM_RANGE_MAX_DAYS
        )));
    }

    Ok(DepartmentHeadTeamRange {
        start_date,
        end_date,
    })
}

fn parse_team_range_date(field: &str, value: Option<&str>) -> Result<Option<NaiveDate>> {
    match value {
        Some(raw) if !raw.trim().is_empty() => {
            let parsed = NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").map_err(|_| {
                AppError::Validation(format!("{} must be a valid YYYY-MM-DD date", field))
            })?;
            Ok(Some(parsed))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_team_range_defaults_when_both_none() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let range = resolve_team_range(None, None, today).expect("default range");
        assert_eq!(range.start_date, today);
        assert_eq!(
            range.end_date,
            range.start_date + chrono::Duration::days(DH_TEAM_RANGE_DEFAULT_DAYS)
        );
    }

    #[test]
    fn resolve_team_range_rejects_start_after_end() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        assert!(matches!(
            resolve_team_range(Some("2026-06-30"), Some("2026-06-01"), today),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn resolve_team_range_rejects_range_above_cap() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = start + chrono::Duration::days(DH_TEAM_RANGE_MAX_DAYS);
        assert!(matches!(
            resolve_team_range(Some("2026-01-01"), Some(&end.to_string()), today),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn resolve_team_range_accepts_exactly_max_span() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = start + chrono::Duration::days(DH_TEAM_RANGE_MAX_DAYS - 1);
        let range = resolve_team_range(Some("2026-01-01"), Some(&end.to_string()), today)
            .expect("max span allowed");
        assert_eq!(
            (range.end_date - range.start_date).num_days() + 1,
            DH_TEAM_RANGE_MAX_DAYS
        );
    }

    #[test]
    fn resolve_team_range_defaults_end_from_supplied_start() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        let range = resolve_team_range(Some("2026-09-01"), None, today)
            .expect("partial start range allowed");
        assert_eq!(
            range.start_date,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        );
        assert_eq!(
            range.end_date,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
                + chrono::Duration::days(DH_TEAM_RANGE_DEFAULT_DAYS)
        );
    }

    #[test]
    fn resolve_team_range_rejects_malformed_date_with_app_validation() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 22).unwrap();
        assert!(matches!(
            resolve_team_range(Some("not-a-date"), None, today),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn parse_dashboard_query_rejects_duplicate_team_range_keys() {
        assert!(matches!(
            parse_dashboard_query(Some(
                "team_start_date=2026-05-01&team_start_date=2026-06-01"
            )),
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            parse_dashboard_query(Some("team_end_date=2026-05-31&team_end_date=2026-06-30")),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn parse_dashboard_query_ignores_unrelated_params() {
        let query = parse_dashboard_query(Some(
            "unused=1&team_start_date=2026-05-01&team_end_date=2026-05-31",
        ))
        .expect("query parsed");
        assert_eq!(query.team_start_date.as_deref(), Some("2026-05-01"));
        assert_eq!(query.team_end_date.as_deref(), Some("2026-05-31"));
    }
}
