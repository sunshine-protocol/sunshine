pub use ffi_utils;
pub mod error;
pub mod ffi;
/// Generate the FFI for the provided runtime
///
/// ### Example
/// ```
/// use test_client::Client;
/// use sunshine_bounty_ffi::impl_ffi;
///
/// impl_ffi!(client: Client);
/// ```
#[macro_export]
macro_rules! impl_ffi {
    () => {
        use ::std::os::raw;
        #[allow(unused)]
        use $crate::ffi_utils::*;
        #[allow(unused)]
        use $crate::ffi::*;

        gen_ffi! {
            /// Get a bounty Information by using bounty Id
            /// Returns [TODO]
            Bounty::get => fn client_bounty_get(bounty_id: u64 = bounty_id) -> Vec<String>;
            /// Get a submission Information by using submission Id
            /// Returns [TODO]
            Bounty::get_submission => fn client_bounty_get_submission(submission_id: u64 = submission_id) -> Vec<String>;
            /// Create a new Bounty
            /// Returns the `BountyId` as `u64`
            Bounty::post => fn client_bounty_post(
                repo_owner: *const raw::c_char = cstr!(repo_owner),
                repo_name: *const raw::c_char = cstr!(repo_name),
                issue_number: u64 = issue_number,
                amount: u64 = amount
            ) -> u64;
            /// Contribute to a bounty.
            /// Returns the new total bounty amount
            Bounty::contribute => fn client_bounty_contribute(bounty_id: u64 = bounty_id, amount: u64 = amount) -> u128;
            /// Create a submission on a bounty
            /// Returns the `SubmissionId` as `u64`
            Bounty::submit => fn client_bounty_submit(
                bounty_id: u64 = bounty_id,
                repo_owner: *const raw::c_char = cstr!(repo_owner),
                repo_name: *const raw::c_char = cstr!(repo_name),
                issue_number: u64 = issue_number,
                amount: u64 = amount
            ) -> u64;
            /// Approve a Submission using `SubmissionId`
            /// Returns the new total amount on that bounty after this operation
            Bounty::approve => fn client_bounty_approve(submission_id: u64 = submission_id) -> u128;
        }
    };
    (client: $client: ty) => {
        gen_ffi!(client = $client);
        $crate::impl_ffi!();
    };
}
