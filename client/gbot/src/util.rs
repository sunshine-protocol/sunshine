use crate::error::Error;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Bounty {
    pub id: u64,
    pub total: u128,
}

impl FromStr for Bounty {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id_sub_index =
            s.find("BountyID: ").ok_or(Error::ParseBountyError)?;
        let bounty_id: u64 = s
            .get((id_sub_index + 9)..(id_sub_index + 29))
            .ok_or(Error::ParseBountyError)?
            .parse::<u64>()?;
        let amt_sub_index =
            s.find("Total Balance: ").ok_or(Error::ParseBountyError)?;
        let total_amt: u128 = s
            .get((amt_sub_index + 15)..(amt_sub_index + 55))
            .ok_or(Error::ParseBountyError)?
            .parse::<u128>()?;

        Ok(Bounty {
            id: bounty_id,
            total: total_amt,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Submission {
    pub bounty_id: u64,
    pub submission_id: u64,
    pub requested_amt: u128,
    pub approved: bool,
}

impl FromStr for Submission {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bid_sub_index =
            s.find("BountyID: ").ok_or(Error::ParseSubmissionError)?;
        let bounty_id: u64 = s
            .get((bid_sub_index + 9)..(bid_sub_index + 29))
            .ok_or(Error::ParseSubmissionError)?
            .parse::<u64>()?;
        let sid_sub_index = s
            .find("SubmissionID: ")
            .ok_or(Error::ParseSubmissionError)?;
        let submission_id: u64 = s
            .get((sid_sub_index + 14)..(sid_sub_index + 34))
            .ok_or(Error::ParseSubmissionError)?
            .parse::<u64>()?;
        let amt_sub_index = s
            .find("Requested Amount: ")
            .ok_or(Error::ParseSubmissionError)?;
        let requested_amt: u128 = s
            .get((amt_sub_index + 18)..(amt_sub_index + 58))
            .ok_or(Error::ParseSubmissionError)?
            .parse::<u128>()?;
        let approved_sub_index =
            s.find("Approved: ").ok_or(Error::ParseSubmissionError)?;
        let approved: bool = s
            .get((approved_sub_index + 10)..(approved_sub_index + 15))
            .ok_or(Error::ParseSubmissionError)?
            .parse::<bool>()?;

        Ok(Submission {
            bounty_id,
            submission_id,
            requested_amt,
            approved,
        })
    }
}
