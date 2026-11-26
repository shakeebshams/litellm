use jiff::Timestamp;
use litellm_python_compat::Value;

use super::{
    bag::{Bag, Ints},
    record::{ExcludedIfNone, Field, fields, record},
};
use crate::{DecodeError, FromValue, IntoValue};

record! {
    /// `LiteLLM_BudgetTable.model_dump(mode="json")`, the budget row a cached project nests under
    /// `litellm_budget_table`. The row's `name`, `budget_limits`, `cap_policy`, reset and audit
    /// columns are no fields of the model, so its dump leaves them out.
    pub struct CachedBudget {
        budget_id: Option<String>,
        soft_budget: Option<f64>,
        max_budget: Option<f64>,
        rollover_max_budget: ExcludedIfNone<f64>,
        max_parallel_requests: Option<i64>,
        tpm_limit: Option<i64>,
        rpm_limit: Option<i64>,
        tpd_limit: Option<i64>,
        model_max_budget: Option<Bag>,
        model_tpm_limit: Option<Ints>,
        model_rpm_limit: Option<Ints>,
        budget_duration: Option<String>,
        allowed_models: Option<Vec<String>>,
        temp_budget_increase: Option<f64>,
        temp_budget_expiry: Option<Timestamp>,
    }
}

#[cfg(test)]
mod tests {
    use litellm_python_compat::json;
    use rstest::rstest;

    use super::CachedBudget;
    use crate::{FromValue, IntoValue};

    /// `LiteLLM_BudgetTable(budget_id="budget").model_dump(mode="json")`.
    const NO_ROLLOVER_CAP: &str = concat!(
        r#"{"budget_id": "budget", "soft_budget": null, "max_budget": null,"#,
        r#" "max_parallel_requests": null, "tpm_limit": null, "rpm_limit": null, "tpd_limit": null,"#,
        r#" "model_max_budget": null, "model_tpm_limit": null, "model_rpm_limit": null,"#,
        r#" "budget_duration": null, "allowed_models": null, "temp_budget_increase": null,"#,
        r#" "temp_budget_expiry": null}"#,
    );

    #[rstest]
    fn a_budget_with_no_rollover_cap_leaves_it_out_of_its_entry() {
        let budget = CachedBudget::from_value(&json::loads(NO_ROLLOVER_CAP).unwrap()).unwrap();

        assert_eq!(json::dumps(&budget.into_value()).unwrap(), NO_ROLLOVER_CAP);
    }
}
