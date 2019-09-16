// investment pitches
//
// each pitch proposal specifies an investment type

///
/// T: donation amount
/// U: conditions (which includes vesting schedules)
/// V: org_id
pub enum Donation<T, U, V> {
    DirectDonation(T, U),
    InDirectDonation(T, U, V),
}

pub struct DirectDonation<T, U> {
    donation: T;
    conditions: Vec<U>;
} // might get reputation after investment

// model after grid schema
pub struct IndirectDonation<T, U, V> {
    organization_id: V;
    conditions: Vec<U>;
    donation: T;
} // might get reputation after investment

pub enum Investment<C> {
    Equity,
    Bond,
    Coin
}

// have the runtime vote on this for each investment
pub trait Criteria<> {
    type Asset: Investment;

    type Projections: Vec<dyn Criteria>;

    fn define_criteria() -> 
} // this trait is used like in Joshy's marketplace to enable voting on funds
//

pub trait Pitch<AccountId> {
    
}