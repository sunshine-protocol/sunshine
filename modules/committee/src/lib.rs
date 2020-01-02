// committee
// must be instanceable like a `council`

/// Trait for type that can handle incremental changes to a set of account IDs.
pub trait OnMembersChanged<AccountId> {
	/// A number of members `new` just joined the set and replaced some `old` ones.
	fn on_members_changed(new: &[AccountId], old: &[AccountId]);
}

impl<T> OnMembersChanged<T> for () {
	fn on_members_changed(_new: &[T], _old: &[T]) {}
}
