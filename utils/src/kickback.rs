use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct KickbackEvent<IpfsReference, AccountId, Currency> {
    info: IpfsReference,
    supervisor: AccountId,
    reservation_req: Currency,
    expected_attendance: u32,
    attendance_limit: u32,
}

impl<IpfsReference: Clone, AccountId: Clone, Currency: Copy>
    KickbackEvent<IpfsReference, AccountId, Currency>
{
    pub fn new(
        info: IpfsReference,
        supervisor: AccountId,
        reservation_req: Currency,
        attendance_limit: u32,
    ) -> Self {
        Self {
            info,
            supervisor,
            reservation_req,
            expected_attendance: 0u32,
            attendance_limit,
        }
    }
    pub fn info(&self) -> IpfsReference {
        self.info.clone()
    }
    pub fn supervisor(&self) -> AccountId {
        self.supervisor.clone()
    }
    pub fn reservation_req(&self) -> Currency {
        self.reservation_req
    }
    pub fn expected_attendance(&self) -> u32 {
        self.expected_attendance
    }
    pub fn increment_attendance(&self) -> Option<Self> {
        let new_attendance = self.expected_attendance + 1u32;
        if new_attendance > self.attendance_limit {
            None
        } else {
            Some(Self {
                expected_attendance: new_attendance,
                ..self.clone()
            })
        }
    }
    pub fn attendance_limit(&self) -> u32 {
        self.attendance_limit
    }
}
