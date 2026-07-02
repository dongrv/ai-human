use crate::core::action::{ActionKind, ActionRequest, PermissionLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    RequireConfirmation(String),
    Deny(String),
}

#[derive(Debug, Clone)]
pub struct PolicyGate {
    allow_local_write: bool,
}

impl Default for PolicyGate {
    fn default() -> Self {
        Self {
            allow_local_write: true,
        }
    }
}

impl PolicyGate {
    pub fn evaluate(&self, request: &ActionRequest) -> PolicyDecision {
        match request.kind.permission_level() {
            PermissionLevel::L0Read | PermissionLevel::L1Analyze => PolicyDecision::Allow,
            PermissionLevel::L2LocalWrite if self.allow_local_write => {
                PolicyDecision::RequireConfirmation(
                    "local file writes require a task plan confirmation".into(),
                )
            }
            PermissionLevel::L3Verify => PolicyDecision::Allow,
            PermissionLevel::L4VcsMutate => match request.kind {
                ActionKind::GitCommit => PolicyDecision::RequireConfirmation(
                    "git commit requires explicit confirmation".into(),
                ),
                _ => PolicyDecision::Deny("VCS mutation is blocked in V1".into()),
            },
            PermissionLevel::L5ExternalSideEffect => {
                PolicyDecision::Deny("external side effects are outside V1 scope".into())
            }
            PermissionLevel::L2LocalWrite => {
                PolicyDecision::Deny("local writes are disabled".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::{ActionKind, ActionRequest};

    fn req(kind: ActionKind) -> ActionRequest {
        ActionRequest {
            kind,
            description: "test action".into(),
            target: None,
        }
    }

    #[test]
    fn read_actions_are_allowed() {
        let gate = PolicyGate::default();
        assert_eq!(
            gate.evaluate(&req(ActionKind::ReadFile)),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn local_writes_require_confirmation() {
        let gate = PolicyGate::default();
        assert!(matches!(
            gate.evaluate(&req(ActionKind::WriteLocalFile)),
            PolicyDecision::RequireConfirmation(_)
        ));
    }

    #[test]
    fn external_deployment_is_denied() {
        let gate = PolicyGate::default();
        assert!(matches!(
            gate.evaluate(&req(ActionKind::ExternalDeployment)),
            PolicyDecision::Deny(_)
        ));
    }
}
