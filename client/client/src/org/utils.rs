use crate::error::Error;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct AccountShare(pub String, pub u64);

impl FromStr for AccountShare {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();
        let acc_str = coords[0];
        let share_fromstr =
            coords[1].parse::<u64>().map_err(|_| Error::ParseIntError)?;
        Ok(AccountShare(acc_str.to_string(), share_fromstr))
    }
}
