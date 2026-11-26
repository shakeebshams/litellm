//! A project as the database holds it, in the shape the auth cache holds it.

use std::sync::Arc;

use litellm_redis_store::auth_cache::CachedProject;
use litellm_repositories::{Project, ProjectRepository};

use crate::{
    Error,
    budget::cached_budget,
    column::{nullable, time},
};

/// The project as the database holds it, in the shape the auth cache holds it: the row with its
/// budget, and nothing else nested.
pub struct ProjectDatabase {
    projects: Arc<ProjectRepository>,
}

impl ProjectDatabase {
    pub fn new(projects: Arc<ProjectRepository>) -> Self {
        Self { projects }
    }

    /// The project `project_id`, or `None` when no row has that id. The object permission the
    /// project points at is not read, and `last_refreshed_at` is unset, since the host that caches
    /// the project stamps it.
    pub async fn project(&self, project_id: &str) -> Result<Option<CachedProject>, Error> {
        let Some(found) = self.projects.find_with_budget(project_id).await? else {
            return Ok(None);
        };
        cached_project(found).map(Some)
    }
}

/// What `LiteLLM_ProjectTableCachedObj.model_dump(mode="json")` holds for `project`, or the
/// column the model would refuse the row for. The JSON columns are never SQL NULL, but each may
/// hold a JSON `null`, which the model reads as unset.
fn cached_project(project: Project) -> Result<CachedProject, Error> {
    let Project {
        project_id,
        project_alias,
        description,
        team_id,
        budget_id,
        metadata,
        models,
        spend,
        model_spend,
        model_rpm_limit,
        model_tpm_limit,
        blocked,
        object_permission_id,
        created_at,
        created_by,
        updated_at,
        updated_by,
        litellm_budget_table,
    } = project;
    Ok(CachedProject {
        project_id,
        project_alias,
        description,
        team_id,
        budget_id,
        metadata: nullable("metadata", Some(&metadata))?,
        models,
        spend,
        model_spend: nullable("model_spend", Some(&model_spend))?,
        model_rpm_limit: nullable("model_rpm_limit", Some(&model_rpm_limit))?,
        model_tpm_limit: nullable("model_tpm_limit", Some(&model_tpm_limit))?,
        blocked,
        object_permission_id,
        created_by: Some(created_by),
        updated_by: Some(updated_by),
        created_at: time("created_at", Some(created_at))?,
        updated_at: time("updated_at", Some(updated_at))?,
        litellm_budget_table: litellm_budget_table
            .map(|budget| cached_budget(budget.0))
            .transpose()?,
        object_permission: None,
        last_refreshed_at: None,
        unread: Vec::new(),
    })
}
