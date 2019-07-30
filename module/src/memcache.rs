// entirely just a cache for old membership code
fn join(origin) {
    let new_member = ensure_signed(origin)?;
    ensure!(!Self::is_member(&new_member), "new member is already a member");

    // take a fee from the new member
    let value = T::EntryFee::get(); // TODO: replace with `calculate_entry_fee` based on member size and pot size
    T::Currency::transfer(&new_member, &Self::account_id(), value)
        .map_err(|_| "Not rich enough to join ;(")?;

    // add new member
    <Member<T>>::mutate(|v| v.push(new_member.clone()));
    let c = Self::total_members() + 1;
    <TotalMembers>::put(c);
    // change member set
    T::ChangeMembers::change_members(&[new_member.clone()], &[], &Self::member()[..]);

    Self::deposit_event(RawEvent::NewMember(new_member));
}

fn exit(origin) {
    let old_member = ensure_signed(origin)?;
    ensure!(Self::is_member(&old_member), "exiting member must be a member");
    ensure!(!Self::is_active_voter(&old_member), "exiting member must deregister as a voter before leaving the DAO");

    // exiting member notably gets nothing here
    // remark on dilution complexity normally involved in exits

    // remove existing member
    <Member<T>>::mutate(|m| m.retain(|x| x != &old_member));
    let c = Self::total_members() - 1;
    <TotalMembers>::put(c);
    // change member set
    T::ChangeMembers::change_members(&[], &[old_member.clone()], &Self::member()[..]);

    Self::deposit_event(RawEvent::MemberExit(old_member));
}