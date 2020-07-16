use crate::error::Error;
use libipld::DagCbor;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq, DagCbor)]
pub struct Text {
    text: String,
    codec: CustomCodec,
}

impl Text {
    pub fn new(prefix: &str, full: &str) -> Result<Self, Error> {
        let codec = CustomCodec::from_str(prefix)?;
        if codec.matches_schema(full) {
            Ok(Text {
                text: full.to_string(),
                codec,
            })
        } else {
            Err(Error::ParseCodecError)
        }
    }
}

impl FromStr for Text {
    type Err = Error;

    fn from_str(full: &str) -> Result<Self, Self::Err> {
        let mut lines = full.lines();
        match lines.next() {
            Some(prefix) => Self::new(prefix, full),
            None => Err(Error::ParseCodecError),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, DagCbor)]
/// Must define custom syntax for each of these items
pub enum CustomCodec {
    OrgConstitution,
    VoteTopic,
    BountyPost,
    MilestonePost,
}

impl FromStr for CustomCodec {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "org_constitution" => Ok(CustomCodec::OrgConstitution),
            "vote_topic" => Ok(CustomCodec::VoteTopic),
            "bounty_post" => Ok(CustomCodec::BountyPost),
            "milestone_post" => Ok(CustomCodec::MilestonePost),
            _ => Err(Error::ParseCodecError),
        }
    }
}

/// Bounty Post Body Schema:
/// ```
/// struct BountyBody {
///     repo_owner: String,
///     repo_name: String,
///     issue_number: u64,
/// }
/// ```
/// -> everything else is (for now)
/// ```
/// struct EverythingElse {
///     description: String,
/// }
/// ```
impl CustomCodec {
    pub fn matches_schema(&self, s: &str) -> bool {
        let mut lines = s.lines();
        match self {
            CustomCodec::OrgConstitution => {
                if let Some(s1) = lines.next() {
                    if s1 == "org_constitution" && lines.next().is_some() {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CustomCodec::VoteTopic => {
                if let Some(s1) = lines.next() {
                    if s1 == "vote_topic" && lines.next().is_some() {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CustomCodec::BountyPost => {
                if let Some(s1) = lines.next() {
                    if s1 == "bounty_post" {
                        if let Some(_) = lines.next() {
                            // _ is the repo owner
                            if let Some(_) = lines.next() {
                                // _ is the repo
                                if let Some(s4) = lines.next() {
                                    // s4 is the issue number so it must be parse-able into u64
                                    if let Ok(_) = s4.parse::<u64>() {
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CustomCodec::MilestonePost => {
                if let Some(s1) = lines.next() {
                    if s1 == "milestone_post" && lines.next().is_some() {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}
