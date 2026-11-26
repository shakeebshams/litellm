//! The auth cache's typed project entry on real Redis. The entry literals are the ones
//! `tests/unit/redis/stores/test_auth_cache_project.py` checks against Python's own model and
//! project reader, and `auth-store/tests/project.rs` finds after a database read.
//! Change all three together.

mod support;

use std::time::Duration;

use litellm_redis_client::commands;
use litellm_redis_store::{
    FromValue, IntoValue,
    auth_cache::{AuthCache, AuthObject, CachedProject, Settings},
};
use litellm_redis_testing::{RedisDeployment, Topology};
use rstest::rstest;
use support::{connect, remote};

const TTL: Duration = Duration::from_secs(60);

const CREATED_ENTRY: &str = concat!(
    r#"{"project_id": "created", "project_alias": null, "description": null, "team_id": null,"#,
    r#" "budget_id": null, "metadata": {}, "models": [], "spend": 0.0, "model_spend": {},"#,
    r#" "model_rpm_limit": {}, "model_tpm_limit": {}, "blocked": false,"#,
    r#" "object_permission_id": null, "created_by": "creator", "updated_by": "creator","#,
    r#" "created_at": "2026-11-19T10:00:00.123000Z", "updated_at": "2026-11-19T10:00:00Z","#,
    r#" "litellm_budget_table": null, "object_permission": null, "last_refreshed_at": null}"#,
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

#[rstest]
#[case::as_created("created", CREATED_ENTRY)]
#[case::every_column_set("full", FULL_ENTRY)]
#[tokio::test]
async fn a_project_entry_python_wrote_is_read_typed_and_written_back_in_the_same_bytes(
    #[values(Topology::Standalone, Topology::Cluster)] topology: Topology,
    #[case] project_id: &str,
    #[case] entry: &str,
) {
    let deployment = RedisDeployment::start(topology).await.unwrap();
    let raw = connect(&deployment);
    let cache = AuthCache::new(Some(remote(connect(&deployment))), Settings::default());
    let read = AuthObject::Project(project_id.to_owned());
    let rewritten = AuthObject::Project("rewritten".to_owned());
    raw.send(commands::set(&read.key(), entry, TTL))
        .await
        .unwrap();

    let held = cache.get(None, &read).await.unwrap().unwrap();
    let project = CachedProject::from_value(&held).unwrap();
    cache
        .set(None, &rewritten, project.clone().into_value(), Some(TTL))
        .await
        .unwrap();

    assert_eq!(
        (
            project.project_id.as_str(),
            project.unread,
            raw.send(commands::get::<String>(&rewritten.key()))
                .await
                .unwrap()
                .as_deref(),
        ),
        (project_id, Vec::new(), Some(entry))
    );
}
