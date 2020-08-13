use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BountyInformation {
    pub id: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
    pub depositer: String,
    pub total: u128,
}

#[derive(Debug, Serialize)]
pub struct BountySubmissionInformation {
    pub id: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
    pub bounty_id: String,
    pub submitter: String,
    pub amount: u128,
    pub awaiting_review: bool,
    pub approved: bool,
}
