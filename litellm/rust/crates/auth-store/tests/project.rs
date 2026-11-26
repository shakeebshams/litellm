//! A project's database read against a Prisma-migrated Postgres. Each row literal is what the
//! repository hands the Python proxy, and each entry literal is what Python caches for that row,
//! less `last_refreshed_at`, which the host that caches the project stamps.
//! `tests/unit/redis/stores/test_auth_cache_project.py` checks both against Python's own model and
//! project reader, and `redis-store/tests/auth_cache_project.rs` round-trips the entries through
//! Redis. Change all three together.

mod support;

use std::sync::Arc;

use litellm_auth_store::{Error, ProjectDatabase};
use litellm_db_testing::MigratedPostgres;
use litellm_repositories::ProjectRepository;
use rstest::{fixture, rstest};
use sqlx::PgPool;

use support::{database, dumped, execute};

const CREATED_ROW: &str = concat!(
    r#"{"project_id":"created","project_alias":null,"description":null,"team_id":null,"#,
    r#""budget_id":null,"metadata":{},"models":[],"spend":0.0,"model_spend":{},"#,
    r#""model_rpm_limit":{},"model_tpm_limit":{},"blocked":false,"object_permission_id":null,"#,
    r#""created_at":"2026-11-19T10:00:00.123+00:00","created_by":"creator","#,
    r#""updated_at":"2026-11-19T10:00:00+00:00","updated_by":"creator","#,
    r#""litellm_budget_table":null}"#,
);
const CREATED_ENTRY: &str = concat!(
    r#"{"project_id": "created", "project_alias": null, "description": null, "team_id": null,"#,
    r#" "budget_id": null, "metadata": {}, "models": [], "spend": 0.0, "model_spend": {},"#,
    r#" "model_rpm_limit": {}, "model_tpm_limit": {}, "blocked": false,"#,
    r#" "object_permission_id": null, "created_by": "creator", "updated_by": "creator","#,
    r#" "created_at": "2026-11-19T10:00:00.123000Z", "updated_at": "2026-11-19T10:00:00Z","#,
    r#" "litellm_budget_table": null, "object_permission": null, "last_refreshed_at": null}"#,
);
const FULL_ROW: &str = concat!(
    r#"{"project_id":"full","project_alias":"Full","description":"Every column set","#,
    r#""team_id":"team","budget_id":"budget","metadata":{"guardrails": ["pii"]},"#,
    r#""models":["claude","all-proxy-models"],"spend":1.5,"model_spend":{"claude": 1.5},"#,
    r#""model_rpm_limit":null,"model_tpm_limit":{"claude": 100},"blocked":true,"#,
    r#""object_permission_id":"permission","created_at":"2026-11-19T10:00:00.123+00:00","#,
    r#""created_by":"creator","updated_at":"2026-11-19T10:00:00+00:00","updated_by":"updater","#,
    r#""litellm_budget_table":{"budget_id":"budget","name":"Project","max_budget":10.5,"#,
    r#""rollover_max_budget":20.0,"soft_budget":8.0,"max_parallel_requests":3,"tpm_limit":1000,"#,
    r#""rpm_limit":10,"tpd_limit":100000,"model_max_budget":{"claude": {"max_budget": 1}},"#,
    r#""model_tpm_limit":{"gpt": "7", "claude": 100.0},"model_rpm_limit":{"claude": 5},"#,
    r#""budget_duration":"30d","budget_limits":[],"cap_policy":"hard","#,
    r#""budget_reset_at":"2026-12-01T00:00:00+00:00","budget_reset_count":2,"#,
    r#""allowed_models":["claude"],"temp_budget_increase":1.5,"#,
    r#""temp_budget_expiry":"2026-11-20T12:30:00.250+00:00","#,
    r#""created_at":"2026-11-01T00:00:00+00:00","created_by":"test","#,
    r#""updated_at":"2026-11-02T00:00:00+00:00","updated_by":"test"}}"#,
);
const FULL_ENTRY: &str = concat!(
    r#"{"project_id": "full", "project_alias": "Full", "description": "Every column set","#,
    r#" "team_id": "team", "budget_id": "budget", "metadata": {"guardrails": ["pii"]},"#,
    r#" "models": ["claude", "all-proxy-models"], "spend": 1.5, "model_spend": {"claude": 1.5},"#,
    r#" "model_rpm_limit": null, "model_tpm_limit": {"claude": 100}, "blocked": true,"#,
    r#" "object_permission_id": "permission", "created_by": "creator", "updated_by": "updater","#,
    r#" "created_at": "2026-11-19T10:00:00.123000Z", "updated_at": "2026-11-19T10:00:00Z","#,
    r#" "litellm_budget_table": {"budget_id": "budget", "soft_budget": 8.0, "max_budget": 10.5,"#,
    r#" "rollover_max_budget": 20.0, "max_parallel_requests": 3, "tpm_limit": 1000,"#,
    r#" "rpm_limit": 10, "tpd_limit": 100000,"#,
    r#" "model_max_budget": {"claude": {"max_budget": 1}}, "model_tpm_limit": {"gpt": 7,"#,
    r#" "claude": 100}, "model_rpm_limit": {"claude": 5}, "budget_duration": "30d","#,
    r#" "allowed_models": ["claude"], "temp_budget_increase": 1.5,"#,
    r#" "temp_budget_expiry": "2026-11-20T12:30:00.250000Z"}, "object_permission": null,"#,
    r#" "last_refreshed_at": null}"#,
);

fn projects(postgres: &MigratedPostgres) -> ProjectRepository {
    ProjectRepository::new(database(postgres))
}

/// A migrated Postgres holding a project as `/project/new` stores it when asked for nothing, and
/// one with every column set, its budget's too.
#[fixture]
async fn postgres() -> MigratedPostgres {
    let postgres = MigratedPostgres::start().await.unwrap();
    insert_projects(postgres.pool()).await;
    postgres
}

async fn insert_projects(pool: &PgPool) {
    execute(
        pool,
        r#"INSERT INTO "LiteLLM_ProjectTable" (project_id, created_by, updated_by, created_at, updated_at)
           VALUES ('created', 'creator', 'creator', '2026-11-19 10:00:00.123', '2026-11-19 10:00:00')"#,
    )
    .await;
    execute(
        pool,
        r#"INSERT INTO "LiteLLM_TeamTable" (team_id) VALUES ('team')"#,
    )
    .await;
    execute(
        pool,
        r#"INSERT INTO "LiteLLM_ObjectPermissionTable" (object_permission_id) VALUES ('permission')"#,
    )
    .await;
    execute(
        pool,
        r#"INSERT INTO "LiteLLM_BudgetTable" (
               budget_id, name, max_budget, rollover_max_budget, soft_budget, max_parallel_requests,
               tpm_limit, rpm_limit, tpd_limit, model_max_budget, model_tpm_limit, model_rpm_limit,
               budget_duration, budget_limits, cap_policy, budget_reset_at, budget_reset_count,
               allowed_models, temp_budget_increase, temp_budget_expiry, created_at, created_by,
               updated_at, updated_by
           ) VALUES (
               'budget', 'Project', 10.5, 20.0, 8, 3,
               1000, 10, 100000, '{"claude": {"max_budget": 1}}', '{"claude": 100.0, "gpt": "7"}', '{"claude": 5}',
               '30d', '[]', 'hard', '2026-12-01 00:00:00', 2,
               ARRAY['claude'], 1.5, '2026-11-20 12:30:00.25', '2026-11-01 00:00:00', 'test',
               '2026-11-02 00:00:00', 'test'
           )"#,
    )
    .await;
    execute(
        pool,
        r#"INSERT INTO "LiteLLM_ProjectTable" (
               project_id, project_alias, description, team_id, budget_id, metadata, models, spend,
               model_spend, model_rpm_limit, model_tpm_limit, blocked, object_permission_id,
               created_at, created_by, updated_at, updated_by
           ) VALUES (
               'full', 'Full', 'Every column set', 'team', 'budget', '{"guardrails": ["pii"]}',
               ARRAY['claude', 'all-proxy-models'], 1.5, '{"claude": 1.5}', 'null', '{"claude": 100}',
               true, 'permission', '2026-11-19 10:00:00.123', 'creator', '2026-11-19 10:00:00', 'updater'
           )"#,
    )
    .await;
}

#[rstest]
#[case::as_created("created", CREATED_ROW, CREATED_ENTRY)]
#[case::every_column_set("full", FULL_ROW, FULL_ENTRY)]
#[tokio::test(flavor = "multi_thread")]
async fn a_project_read_from_the_database_is_the_entry_python_caches(
    #[future(awt)] postgres: MigratedPostgres,
    #[case] project_id: &str,
    #[case] row: &str,
    #[case] cached: &str,
) {
    let found = projects(&postgres)
        .find_with_budget(project_id)
        .await
        .unwrap()
        .unwrap();
    let read = ProjectDatabase::new(Arc::new(projects(&postgres)))
        .project(project_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        (
            serde_json::to_string(&found).unwrap().as_str(),
            dumped(read).as_str()
        ),
        (row, cached)
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn a_project_no_row_has_is_not_found(#[future(awt)] postgres: MigratedPostgres) {
    let read = ProjectDatabase::new(Arc::new(projects(&postgres)))
        .project("gone")
        .await
        .unwrap();

    assert_eq!(read, None);
}

#[rstest]
#[case::metadata_that_is_a_list(
    r#"UPDATE "LiteLLM_ProjectTable" SET metadata = '[1]'"#,
    "metadata"
)]
#[case::a_budget_with_concurrent_windows(
    r#"UPDATE "LiteLLM_BudgetTable" SET budget_limits = '[{"budget_duration": "1d", "max_budget": 1}]'"#,
    "budget_limits"
)]
#[case::a_budget_model_limit_that_is_no_whole_number(
    r#"UPDATE "LiteLLM_BudgetTable" SET model_tpm_limit = '{"claude": 1.5}'"#,
    "model_tpm_limit"
)]
#[tokio::test(flavor = "multi_thread")]
async fn a_project_row_the_model_refuses_fails_the_read(
    #[future(awt)] postgres: MigratedPostgres,
    #[case] corrupt: &'static str,
    #[case] column: &str,
) {
    execute(postgres.pool(), corrupt).await;

    let refused = ProjectDatabase::new(Arc::new(projects(&postgres)))
        .project("full")
        .await;

    assert!(
        matches!(refused, Err(Error::Row { column: named }) if named == column),
        "{refused:?}"
    );
}
