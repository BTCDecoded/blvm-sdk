//! Nested Multisig Support
//!
//! Provides cryptographic primitives for nested multisig operations.
//! Supports team-based signature aggregation for nested 7×7 multisig structure.

use crate::governance::error::{GovernanceError, GovernanceResult};
use crate::governance::{PublicKey, Signature};

/// Team structure for nested multisig
#[derive(Debug, Clone)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub maintainers: Vec<TeamMaintainer>,
}

#[derive(Debug, Clone)]
pub struct TeamMaintainer {
    pub github: String,
    pub public_key: PublicKey,
}

/// Nested multisig configuration
#[derive(Debug, Clone)]
pub struct NestedMultisig {
    teams: Vec<Team>,
    teams_required: usize,
    maintainers_per_team_required: usize,
}

impl NestedMultisig {
    /// Create a new nested multisig configuration
    pub fn new(
        teams: Vec<Team>,
        teams_required: usize,
        maintainers_per_team_required: usize,
    ) -> GovernanceResult<Self> {
        if teams_required == 0 || teams_required > teams.len() {
            return Err(GovernanceError::InvalidThreshold {
                threshold: teams_required,
                total: teams.len(),
            });
        }

        if maintainers_per_team_required == 0 {
            return Err(GovernanceError::InvalidThreshold {
                threshold: maintainers_per_team_required,
                total: 7, // Assuming 7 maintainers per team
            });
        }

        // Validate each team has enough maintainers
        for team in &teams {
            if team.maintainers.len() < maintainers_per_team_required {
                return Err(GovernanceError::InvalidMultisig(format!(
                    "Team {} has {} maintainers, but {} required",
                    team.id,
                    team.maintainers.len(),
                    maintainers_per_team_required
                )));
            }
        }

        Ok(Self {
            teams,
            teams_required,
            maintainers_per_team_required,
        })
    }

    /// Verify nested multisig signatures
    ///
    /// Process:
    /// 1. Group signatures by team
    /// 2. Count team approvals (maintainers_per_team_required per team)
    /// 3. Count inter-team approvals (teams_required)
    pub fn verify(
        &self,
        message: &[u8],
        signatures: &[(String, Signature)], // (github_username, signature)
    ) -> GovernanceResult<NestedMultisigResult> {
        // Group signatures by team
        let mut team_signatures: std::collections::HashMap<String, Vec<(String, Signature)>> =
            std::collections::HashMap::new();

        for (github, signature) in signatures {
            if let Some(team_id) = self.find_maintainer_team(github) {
                team_signatures
                    .entry(team_id)
                    .or_default()
                    .push((github.clone(), signature.clone()));
            }
        }

        // Count team approvals
        let mut teams_approved = 0;
        let mut total_maintainers_approved = 0;
        let mut team_details = Vec::new();

        for team in &self.teams {
            // Verify signatures for this team
            let mut valid_sigs = 0;
            if let Some(sigs) = team_signatures.get(&team.id) {
                for (github, sig) in sigs {
                    // Find maintainer's public key
                    if let Some(maintainer) = team.maintainers.iter().find(|m| m.github == *github)
                    {
                        if crate::governance::verify_signature(
                            sig,
                            message,
                            &maintainer.public_key,
                        )? {
                            valid_sigs += 1;
                        }
                    }
                }
            }

            let team_approved = valid_sigs >= self.maintainers_per_team_required;

            if team_approved {
                teams_approved += 1;
                total_maintainers_approved += valid_sigs;
            }

            team_details.push(TeamApprovalStatus {
                team_id: team.id.clone(),
                team_name: team.name.clone(),
                maintainers_signed: valid_sigs,
                maintainers_required: self.maintainers_per_team_required,
                approved: team_approved,
            });
        }

        let inter_team_approved = teams_approved >= self.teams_required;
        let total_maintainers_required = self.teams_required * self.maintainers_per_team_required;

        Ok(NestedMultisigResult {
            teams_approved,
            teams_required: self.teams_required,
            maintainers_approved: total_maintainers_approved,
            maintainers_required: total_maintainers_required,
            inter_team_approved,
            team_details,
        })
    }

    /// Find which team a maintainer belongs to
    fn find_maintainer_team(&self, github: &str) -> Option<String> {
        for team in &self.teams {
            if team.maintainers.iter().any(|m| m.github == github) {
                return Some(team.id.clone());
            }
        }
        None
    }
}

/// Result of nested multisig verification
#[derive(Debug, Clone)]
pub struct NestedMultisigResult {
    pub teams_approved: usize,
    pub teams_required: usize,
    pub maintainers_approved: usize,
    pub maintainers_required: usize,
    pub inter_team_approved: bool,
    pub team_details: Vec<TeamApprovalStatus>,
}

#[derive(Debug, Clone)]
pub struct TeamApprovalStatus {
    pub team_id: String,
    pub team_name: String,
    pub maintainers_signed: usize,
    pub maintainers_required: usize,
    pub approved: bool,
}
