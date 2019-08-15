use parity_scale_codec::{Codec, Encode, Decode};

// GENERIC VOTING

// https://youtu.be/9x7W3_KKKeA?t=1409
// using rayon to compute multiple magnitudes at the same time?
// -- is this a good idea for calculating conviction of ayes vs nays votes at the same time?
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum VoteThreshold {
	/// A supermajority of approvals is needed to pass this vote.
	SuperMajorityApprove,
	/// A supermajority of rejects is needed to fail this vote.
	SuperMajorityAgainst,
	/// A simple majority of approvals is needed to pass this vote.
	SimpleMajority,
}


/// GOAL
///
/// make this more modular than the existing implementation
///
/// first exercise, implement Quadratic Votinhg
pub trait Approved<T> {
    type Currency: T;
	/// Given `approve` votes for and `against` votes against from a total electorate size of
	/// `electorate` (`electorate - (approve + against)` are abstainers), then returns true if the
	/// overall outcome is in favor of approval.
	fn approved(&self, approve: Self::Currency, against: Self::Currency, voters: Self::Currency, electorate: Self::Currency) -> bool;
}

/// Return `true` iff `n1 / d1 < n2 / d2`. `d1` and `d2` may not be zero.
fn compare_rationals<T: Zero + Mul<T, Output = T> + Div<T, Output = T> + Rem<T, Output = T> + Ord + Copy>(mut n1: T, mut d1: T, mut n2: T, mut d2: T) -> bool {
	// Uses a continued fractional representation for a non-overflowing compare.
	// Detailed at https://janmr.com/blog/2014/05/comparing-rational-numbers-without-overflow/.
	loop {
		let q1 = n1 / d1;
		let q2 = n2 / d2;
		if q1 < q2 {
			return true;
		}
		if q2 < q1 {
			return false;
		}
		let r1 = n1 % d1;
		let r2 = n2 % d2;
		if r2.is_zero() {
			return false;
		}
		if r1.is_zero() {
			return true;
		}
		n1 = d2;
		n2 = d1;
		d1 = r2;
		d2 = r1;
	}
}

impl<Balance: IntegerSquareRoot + Zero + Ord + Add<Balance, Output = Balance> + Mul<Balance, Output = Balance> + Div<Balance, Output = Balance> + Rem<Balance, Output = Balance> + Copy> Approved<Balance> for VoteThreshold {
	/// Given `approve` votes for and `against` votes against from a total electorate size of
	/// `electorate` of whom `voters` voted (`electorate - voters` are abstainers) then returns true if the
	/// overall outcome is in favor of approval.
	///
	/// We assume each *voter* may cast more than one *vote*, hence `voters` is not necessarily equal to
	/// `approve + against`.
	fn approved(&self, approve: Balance, against: Balance, voters: Balance, electorate: Balance) -> bool {
		let sqrt_voters = voters.integer_sqrt();
		let sqrt_electorate = electorate.integer_sqrt();
		if sqrt_voters.is_zero() { return false; }
		match *self {
			VoteThreshold::SuperMajorityApprove =>
				compare_rationals(against, sqrt_voters, approve, sqrt_electorate),
			VoteThreshold::SuperMajorityAgainst =>
				compare_rationals(against, sqrt_electorate, approve, sqrt_voters),
			VoteThreshold::SimpleMajority => approve > against,
		}
	}
}