// trait abstractions (wip)

pub trait Moloch {
    type Shares: _;
    type Currency: _;

    fn propose(&self, Currency, Shares) -> Result<(Currency, Shares), &str>;

    fn vote(&self, Hash, Shares);

    fn burn(&self, Shares);

    fn kick(&self, AccountId, Shares);
}