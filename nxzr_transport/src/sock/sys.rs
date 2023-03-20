use libc::sa_family_t;

pub const BTPROTO_HCI: i32 = 1;

#[repr(C)]
#[derive(Clone)]
pub struct sockaddr_hci {
    pub hci_family: sa_family_t,
    pub hci_dev: u16,
    pub hci_channel: u16,
}
