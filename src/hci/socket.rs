/// BlueZ socket layer. Interacts with the BlueZ driver over socket AF_BLUETOOTH.
use crate::bytes::ToFromBytesEndian;
use crate::hci::stream::PacketType;
use crate::hci::EventCode;
use std::os::unix::{
    io::{AsRawFd, FromRawFd, RawFd},
    net::UnixStream,
};
use std::sync::Mutex;

mod ioctl {
    nix::ioctl_write_int!(hci_device_up, b'H', 201);
    nix::ioctl_write_int!(hci_device_down, b'H', 202);
    nix::ioctl_write_int!(hci_device_reset, b'H', 203);
    nix::ioctl_write_int!(hci_device_stats, b'H', 204);
    // HCIGETDEVLIST =	_IOR('H', 210, int)
    nix::ioctl_read!(hci_get_dev_list, b'H', 210, super::HCIDevListReq);

    // HCIGETDEVINFO =	_IOR('H', 211, int)
    nix::ioctl_read!(hci_get_dev_info, b'H', 211, super::HCIDevInfo);
}
#[repr(i32)]
enum BTProtocol {
    L2CAP = 0,
    HCI = 1,
    SCO = 2,
    RFCOMM = 3,
    BNEP = 4,
    CMTP = 5,
    HIDP = 6,
    AVDTP = 7,
}
impl From<BTProtocol> for i32 {
    fn from(protocol: BTProtocol) -> Self {
        protocol as i32
    }
}
/// BlueZ HCI Channels. Each Channel gives different levels of control over the Bluetooth
/// Controller.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
#[repr(u16)]
enum HCIChannel {
    /// Requires sudo. Exclusive access to the Controller.
    Raw = 0,
    /// Shouldn't require sudo. Exclusive access to the Controller.
    User = 1,
    Monitor = 2,
    Control = 3,
    Logging = 4,
}
impl From<HCIChannel> for u16 {
    fn from(channel: HCIChannel) -> Self {
        channel as u16
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash, Default)]
struct HCIDevStats {
    pub err_rx: u32,
    pub err_tx: u32,
    pub cmd_tx: u32,
    pub evt_rx: u32,
    pub acl_tx: u32,
    pub acl_rx: u32,
    pub sco_tx: u32,
    pub sco_rx: u32,
    pub byte_rx: u32,
    pub byte_tx: u32,
}
struct HCIDevReq {
    pub dev_id: u16,
    pub dev_opt: u32,
}
struct HCIDevInfo {
    pub dev_id: u16,
    pub name: [u8; 8],
    pub address: BTAddress,
    pub flags: u32,
    pub dev_type: u8,
    pub features: [u8; 8],
    pub pkt_type: u32,
    pub link_policy: u32,
    pub link_mode: u32,
    pub acl_mtu: u16,
    pub acl_pkts: u16,
    pub sco_mtu: u16,
    pub sco_pkts: u16,
    pub stats: HCIDevStats,
}
pub struct HCIDevListReq {}
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub struct AdapterID(pub u16);

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct SockaddrHCI {
    hci_family: libc::sa_family_t,
    hci_dev: u16,
    hci_channel: u16,
}
/// Wrapper for a BlueZ HCI Stream. Uses Unix Sockets. `HCISocket`'s have a special filter on them
/// for HCI Events so that is why they are wrapped. Besides the filter, they are just byte streams
/// that need to have the Events and Commands abstracted over them.
pub struct HCISocket(UnixStream);
/// Turns an libc `ERRNO` error number into a `HCISocketError`.
pub fn handle_libc_error(i: RawFd) -> Result<i32, HCISocketError> {
    if i < 0 {
        Err(match nix::errno::errno() {
            1 => HCISocketError::PermissionDenied,
            16 => HCISocketError::Busy,
            e => HCISocketError::Other(e),
        })
    } else {
        Ok(i)
    }
}
#[derive(Debug)]
pub enum HCISocketError {
    PermissionDenied,
    DeviceNotFound,
    NotConnected,
    Busy,
    IO(std::io::Error),
    Other(i32),
}
impl HCISocket {
    /// Creates an `HCISocket` based on a `libc` file_descriptor (`i32`). Returns an error if could
    /// not bind to the `adapter_id`.
    pub fn new_channel(
        adapter_id: AdapterID,
        channel: HCIChannel,
    ) -> Result<HCISocket, HCISocketError> {
        let adapter_fd = handle_libc_error(unsafe {
            libc::socket(
                libc::AF_BLUETOOTH,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                BTProtocol::HCI.into(),
            )
        })?;
        let address = SockaddrHCI {
            hci_family: libc::AF_BLUETOOTH as u16,
            hci_dev: adapter_id.0,
            hci_channel: HCIChannel::User.into(),
        };
        handle_libc_error(unsafe {
            libc::bind(
                adapter_fd,
                &address as *const SockaddrHCI as *const libc::sockaddr,
                std::mem::size_of::<SockaddrHCI>() as u32,
            )
        })?;
        let stream = unsafe { UnixStream::from_raw_fd(adapter_fd) };
        let out = HCISocket(stream);
        out.set_filter()?;
        Ok(out)
    }
    pub fn raw_fd(&self) -> i32 {
        self.0.as_raw_fd()
    }
}
impl From<HCISocket> for UnixStream {
    fn from(socket: HCISocket) -> Self {
        socket.0
    }
}
impl HCISocket {
    /// Sets the HCI Event filter on the socket. Should only need to be called once. Is also called
    /// automatically by the `new` constructor.
    fn set_filter(&self) -> Result<(), HCISocketError> {
        const HCI_FILTER: i32 = 2;
        const SOL_HCI: i32 = 0;
        let type_mask =
            (1u32 << u32::from(PacketType::Command)) | (1u32 << u32::from(PacketType::Event));
        let event_mask1 = (1u32 << u32::from(EventCode::CommandComplete))
            | (1u32 << u32::from(EventCode::CommandStatus));

        let mut filter = [0_u8; 14];
        filter[0..4].copy_from_slice(&type_mask.to_bytes_le());
        filter[4..8].copy_from_slice(&event_mask1.to_bytes_le());

        handle_libc_error(unsafe {
            libc::setsockopt(
                self.raw_fd(),
                SOL_HCI,
                HCI_FILTER,
                filter.as_mut_ptr() as *mut _ as *mut libc::c_void,
                filter.len() as u32,
            )
        })?;
        Ok(())
    }
}

pub struct Manager {
    control_fd: Mutex<i32>,
}
impl Manager {
    pub fn new() -> Result<Manager, HCISocketError> {
        Ok(Manager {
            control_fd: Mutex::new(handle_libc_error(unsafe {
                libc::socket(
                    libc::AF_BLUETOOTH,
                    libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                    BTProtocol::HCI.into(),
                )
            })?),
        })
    }
    pub fn device_up(&self, adapter_id: AdapterID) -> Result<(), HCISocketError> {
        let control_lock = self
            .control_fd
            .lock()
            .expect("mutexs only fail when poisoned");
        let control_fd = *control_lock.deref();
        unsafe {
            ioctl::hci_device_up(
                control_fd,
                adapter_id.0 as nix::sys::ioctl::ioctl_param_type,
            )?
        }
        Ok(())
    }
    pub fn device_down(&self, adapter_id: AdapterID) -> Result<(), HCISocketError> {
        let control_lock = self
            .control_fd
            .lock()
            .expect("mutexs only fail when poisoned");
        let control_fd = *control_lock.deref();
        unsafe {
            ioctl::hci_device_down(
                control_fd,
                adapter_id.0 as nix::sys::ioctl::ioctl_param_type,
            )?
        }
        Ok(())
    }
}

#[cfg(feature = "bluez_async")]
pub mod async_socket {
    use super::HCISocket;
    use core::convert::TryFrom;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use tokio::io::AsyncRead;
    impl TryFrom<HCISocket> for AsyncHCISocket {
        type Error = std::io::Error;

        /// Returns `std::io::Error` if it can't bind the `UnixStream` to the tokio Event
        /// loop. Usually safe to `.unwrap()/.expect()` unless bad file descriptor.
        fn try_from(socket: HCISocket) -> Result<Self, Self::Error> {
            Ok(AsyncHCISocket(tokio::net::UnixStream::from_std(
                socket.into(),
            )?))
        }
    }

    pub struct AsyncHCISocket(pub tokio::net::UnixStream);

    impl futures::AsyncRead for AsyncHCISocket {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }
}
use crate::BTAddress;
#[cfg(feature = "bluez_async")]
pub use async_socket::AsyncHCISocket;
use core::ops::Deref;
