use jiff::Timestamp;
use litellm_python_compat::Value;

use super::{
    bag::Bag,
    budget::CachedBudget,
    object_permission::CachedObjectPermission,
    record::{Field, fields, record, required},
};
use crate::{DecodeError, FromValue, IntoValue};

record! {
    /// `LiteLLM_ProjectTableCachedObj.model_dump(mode="json")`, the value under
    /// [`AuthObject::Project`](crate::auth_cache::AuthObject::Project).
    pub struct CachedProject {
        project_id: String = required!("project_id"),
        project_alias: Option<String>,
        description: Option<String>,
        team_id: Option<String>,
        budget_id: Option<String>,
        metadata: Option<Bag>,
        models: Vec<String>,
        spend: f64,
        model_spend: Option<Bag>,
        model_rpm_limit: Option<Bag>,
        model_tpm_limit: Option<Bag>,
        blocked: bool,
        object_permission_id: Option<String>,
        created_by: Option<String>,
        updated_by: Option<String>,
        created_at: Option<Timestamp>,
        updated_at: Option<Timestamp>,
        litellm_budget_table: Option<CachedBudget>,
        object_permission: Option<CachedObjectPermission>,
        last_refreshed_at: Option<f64>,
    }
}
