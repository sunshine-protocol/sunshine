use crate::InvalidGithubIssueUrl;
use regex::Regex;
use std::convert::TryFrom;
const GITHUB_ISSUE_URL_REGEX: &str = r"(?m)^https://github.com/([A-Za-z0-9]+(?:[ _-][A-Za-z0-9]+)*)/([A-Za-z0-9]+(?:[ _-][A-Za-z0-9]+)*)/issues/(\d+)$";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GithubIssueMetadata {
    pub owner: String,
    pub repo: String,
    pub issue: u64,
}

impl<'a> TryFrom<&'a str> for GithubIssueMetadata {
    type Error = InvalidGithubIssueUrl;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        parse_url(value)
    }
}

fn parse_url(url: &str) -> Result<GithubIssueMetadata, InvalidGithubIssueUrl> {
    let re = Regex::new(GITHUB_ISSUE_URL_REGEX).expect("invalid regex used!");
    match re.captures(url) {
        Some(groups) => {
            Ok(GithubIssueMetadata {
                owner: groups
                    .get(1)
                    .ok_or(InvalidGithubIssueUrl)?
                    .as_str()
                    .to_owned(),
                repo: groups
                    .get(2)
                    .ok_or(InvalidGithubIssueUrl)?
                    .as_str()
                    .to_owned(),
                issue: groups
                    .get(3)
                    .ok_or(InvalidGithubIssueUrl)?
                    .as_str()
                    .parse()
                    .expect("should be a valid issue number!"),
            })
        }
        None => Err(InvalidGithubIssueUrl),
    }
}

#[cfg(test)]
mod test_utils {
    use super::*;

    #[test]
    fn test_parse_github_issue() {
        let url = "https://github.com/sunshine-protocol/sunshine/issues/16";
        let issue = parse_url(url).unwrap();
        assert_eq!(
            issue,
            GithubIssueMetadata {
                owner: String::from("sunshine-protocol"),
                repo: String::from("sunshine"),
                issue: 16
            }
        );
    }

    #[test]
    #[should_panic(expected = "InvalidGithubIssueUrl")]
    fn test_parse_memehub_issue() {
        let url = "https://memehub.com/sunshine-protocol/sunshine/issues/16";
        let _ = parse_url(url).unwrap();
    }
}
