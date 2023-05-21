// Excerpt from `bluer` project: https://github.com/bluez/bluer/blob/8ffd4aeef3f8ab0d65dca66eb5a03f223351f586/bluer/src/sys.rs
//! System native types and constants.
#![allow(dead_code)]
use libc::{c_ushort, sa_family_t};

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

pub const SOL_L2CAP: i32 = 6;

/// Bluetooth security.
#[repr(C)]
#[derive(Clone)]
pub struct bt_security {
    /// Level.
    pub level: u8,
    /// Key size.
    pub key_size: u8,
}

pub const BT_SECURITY: i32 = 4;
pub const BT_SECURITY_SDP: i32 = 0;
pub const BT_SECURITY_LOW: i32 = 1;
pub const BT_SECURITY_MEDIUM: i32 = 2;
pub const BT_SECURITY_HIGH: i32 = 3;
pub const BT_SECURITY_FIPS: i32 = 4;

#[repr(C)]
#[derive(Clone)]
pub struct bt_power {
    pub force_active: u8,
}

pub const BT_POWER: i32 = 9;
pub const BT_POWER_FORCE_ACTIVE_OFF: i32 = 0;
pub const BT_POWER_FORCE_ACTIVE_ON: i32 = 1;

pub const BT_SNDMTU: i32 = 12;
pub const BT_RCVMTU: i32 = 13;
pub const BT_PHY: i32 = 14;
pub const BT_MODE: i32 = 15;

/// BR1M1SLOT PHY.
pub const BR1M1SLOT: i32 = 1 << 0;
/// BR1M3SLOT PHY.
pub const BR1M3SLOT: i32 = 1 << 1;
/// BR1M5SLOT PHY.
pub const BR1M5SLOT: i32 = 1 << 2;
/// EDR2M1SLOT PHY.
pub const EDR2M1SLOT: i32 = 1 << 3;
/// EDR2M3SLOT PHY.
pub const EDR2M3SLOT: i32 = 1 << 4;
/// EDR2M5SLOT PHY.
pub const EDR2M5SLOT: i32 = 1 << 5;
/// EDR3M1SLOT PHY.
pub const EDR3M1SLOT: i32 = 1 << 6;
/// EDR3M3SLOT PHY.
pub const EDR3M3SLOT: i32 = 1 << 7;
/// EDR3M5SLOT PHY.
pub const EDR3M5SLOT: i32 = 1 << 8;
/// LE1MTX PHY.
pub const LE1MTX: i32 = 1 << 9;
/// LE1MRX PHY.
pub const LE1MRX: i32 = 1 << 10;
/// LE2MTX PHY.
pub const LE2MTX: i32 = 1 << 11;
/// LE2MRX PHY.
pub const LE2MRX: i32 = 1 << 12;
/// LECODEDTX PHY.
pub const LECODEDTX: i32 = 1 << 13;
/// LECODEDRX PHY.
pub const LECODEDRX: i32 = 1 << 14;

pub const BTPROTO_L2CAP: i32 = 0;

/// Bluetooth address.
#[repr(packed)]
#[repr(C)]
#[derive(Clone, Default)]
pub struct bdaddr_t {
    pub b: [u8; 6],
}

pub const BDADDR_BREDR: u8 = 0x00;
pub const BDADDR_LE_PUBLIC: u8 = 0x01;
pub const BDADDR_LE_RANDOM: u8 = 0x02;

/// L2CAP socket address.
#[repr(C)]
#[derive(Clone)]
pub struct sockaddr_l2 {
    pub l2_family: sa_family_t,
    pub l2_psm: c_ushort,
    pub l2_bdaddr: bdaddr_t,
    pub l2_cid: c_ushort,
    pub l2_bdaddr_type: u8,
}

pub const L2CAP_OPTIONS: i32 = 0x01;
pub const L2CAP_CONNINFO: i32 = 0x02;
pub const L2CAP_LM: i32 = 0x03;

/// Master.
pub const L2CAP_LM_MASTER: i32 = 0x0001;
/// Auth.
pub const L2CAP_LM_AUTH: i32 = 0x0002;
/// Encrypt.
pub const L2CAP_LM_ENCRYPT: i32 = 0x0004;
/// Trusted.
pub const L2CAP_LM_TRUSTED: i32 = 0x0008;
/// Reliable.
pub const L2CAP_LM_RELIABLE: i32 = 0x0010;
/// Secure.
pub const L2CAP_LM_SECURE: i32 = 0x0020;
/// FIPS.
pub const L2CAP_LM_FIPS: i32 = 0x0040;

/// Raw socket options for classic Bluetooth (BR/EDR) L2CAP sockets.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct l2cap_options {
    /// Outgoing MTU.
    pub omtu: u16,
    /// Incoming MTU.
    pub imtu: u16,
    /// Flush to?
    pub flush_to: u16,
    /// Mode.
    pub mode: u8,
    /// FCS option.
    pub fcs: u8,
    /// Max transmission.
    pub max_tx: u8,
    /// Transmission window.
    pub txwin_size: u16,
}

impl Default for l2cap_options {
    fn default() -> Self {
        Self {
            omtu: 0,
            imtu: 672,
            flush_to: 65535,
            mode: 0,
            fcs: 0x01,
            max_tx: 3,
            txwin_size: 63,
        }
    }
}

/// L2CAP socket connection information.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct l2cap_conninfo {
    /// Host controller interface (HCI) handle for the connection.
    pub hci_handle: u16,
    /// Device class.
    pub dev_class: [u8; 3],
}
