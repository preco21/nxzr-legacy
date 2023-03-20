use libc::sa_family_t;

pub const BTPROTO_HCI: i32 = 1;
pub const SOL_HCI: i32 = 0;
pub const HCI_FILTER: i32 = 2;

#[repr(C)]
#[derive(Clone)]
pub struct sockaddr_hci {
    pub hci_family: sa_family_t,
    pub hci_dev: u16,
    pub hci_channel: u16,
}

// HCI filter
#[repr(C)]
#[derive(Clone)]
pub struct hci_filter {
    pub type_mask: u32,
    pub event_mask: [u32; 2],
    pub opcode: u16,
}
