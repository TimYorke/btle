//! Generic BLE Advertiser (WIP)
use crate::hci::adapter;
use crate::BTAddress;
use crate::ConversionError;
use core::convert::TryFrom;
use futures_util::future::LocalBoxFuture;
use core::convert::TryInto;
use core::time::Duration;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct AdvertisingInterval(u16);
impl AdvertisingInterval {
    pub const BYTE_LEN: usize = 2;
    pub const MIN_U16: u16 = 0x0020u16;
    pub const MIN: AdvertisingInterval = AdvertisingInterval(Self::MIN_U16);
    pub const MIN_NON_CONN_U16: u16 = 0x00A0;
    pub const MIN_NON_CONN: AdvertisingInterval = AdvertisingInterval(Self::MIN_NON_CONN_U16);
    pub const MAX_U16: u16 = 0x4000u16;
    pub const MAX: AdvertisingInterval = AdvertisingInterval(Self::MAX_U16);
    pub const DEFAULT_U16: u16 = 0x0800u16;
    pub const DEFAULT: AdvertisingInterval = AdvertisingInterval(Self::DEFAULT_U16);
    /// Creates a new `AdvertisingInterval`.
    /// # Panics
    /// Panics if
    /// `interval < AdvertisingInterval::MIN_U16 || interval > AdvertisingInterval::MAX_U16`.
    pub fn new(interval: u16) -> AdvertisingInterval {
        assert!(
            interval <= Self::MAX_U16 && interval >= Self::MIN_U16,
            "invalid advertising interval '{}'",
            interval
        );
        AdvertisingInterval(interval)
    }
    pub const fn as_duration(self) -> core::time::Duration {
        core::time::Duration::from_micros(self.as_microseconds() as u64)
    }
    pub const fn as_microseconds(self) -> u32 {
        self.0 as u32 * 625
    }
    pub fn from_milliseconds(milli: u16) -> Option<AdvertisingInterval> {
        (milli * 16 / 10).try_into().ok()
    }
}
impl Default for AdvertisingInterval {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl TryFrom<core::time::Duration> for AdvertisingInterval {
    type Error = ConversionError;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        Self::from_milliseconds(
            value
                .as_millis()
                .try_into()
                .map_err(|_| ConversionError(()))?,
        )
        .ok_or(ConversionError(()))
    }
}
impl TryFrom<u16> for AdvertisingInterval {
    type Error = ConversionError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value <= Self::MAX_U16 && value >= Self::MIN_U16 {
            Ok(Self(value))
        } else {
            Err(ConversionError(()))
        }
    }
}
impl From<AdvertisingInterval> for u16 {
    fn from(a: AdvertisingInterval) -> Self {
        a.0
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum AdvertisingType {
    AdvInd = 0x00,
    AdvDirectIndHighDutyCycle = 0x01,
    AdvScanInd = 0x02,
    /// WARNING: AdvNonnConnInd only allows a advertise interval of 100ms. BT 5.0 lifts this
    /// restriction but BT 4.0 adapters (most adapters) will report an error if you try to set it
    /// less
    AdvNonnConnInd = 0x03,
    AdvDirectIndLowDutyCycle = 0x04,
}
impl AdvertisingType {
    pub const BYTE_LEN: usize = 1;
    pub const DEFAULT: AdvertisingType = AdvertisingType::AdvInd;
}
impl Default for AdvertisingType {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl From<AdvertisingType> for u8 {
    fn from(a: AdvertisingType) -> Self {
        a as u8
    }
}
impl TryFrom<u8> for AdvertisingType {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(AdvertisingType::AdvInd),
            0x01 => Ok(AdvertisingType::AdvDirectIndHighDutyCycle),
            0x02 => Ok(AdvertisingType::AdvScanInd),
            0x03 => Ok(AdvertisingType::AdvNonnConnInd),
            0x04 => Ok(AdvertisingType::AdvDirectIndLowDutyCycle),
            _ => Err(ConversionError(())),
        }
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum PeerAddressType {
    Public = 0x00,
    Random = 0x01,
}
impl PeerAddressType {
    pub const BYTE_LEN: usize = 1;
    pub const DEFAULT: PeerAddressType = PeerAddressType::Public;
}
impl Default for PeerAddressType {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl From<PeerAddressType> for u8 {
    fn from(a: PeerAddressType) -> Self {
        a as u8
    }
}
impl TryFrom<u8> for PeerAddressType {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(PeerAddressType::Public),
            0x01 => Ok(PeerAddressType::Random),
            _ => Err(ConversionError(())),
        }
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum OwnAddressType {
    PublicDevice = 0x00,
    RandomDevice = 0x01,
    PrivateOrPublic = 0x02,
    PrivateOrRandom = 0x03,
}
impl OwnAddressType {
    pub const DEFAULT: OwnAddressType = OwnAddressType::PublicDevice;
}
impl Default for OwnAddressType {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl From<OwnAddressType> for u8 {
    fn from(t: OwnAddressType) -> Self {
        t as u8
    }
}
impl TryFrom<u8> for OwnAddressType {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(OwnAddressType::PublicDevice),
            0x01 => Ok(OwnAddressType::RandomDevice),
            0x02 => Ok(OwnAddressType::PrivateOrPublic),
            0x03 => Ok(OwnAddressType::PrivateOrRandom),
            _ => Err(ConversionError(())),
        }
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Channels {
    Channel37 = 0x00,
    Channel38 = 0x01,
    Channel39 = 0x02,
}
impl From<Channels> for u8 {
    fn from(c: Channels) -> Self {
        c as u8
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ChannelMap(u8);
impl ChannelMap {
    pub const ZEROED: ChannelMap = ChannelMap(0);
    pub const ALL_U8: u8 = 0x07;
    pub const ALL: ChannelMap = ChannelMap(ChannelMap::ALL_U8);
    pub const DEFAULT: ChannelMap = ChannelMap::ALL;
    /// Creates a new `ChannelMap`.
    /// # Panics
    /// Panics if `map > u16::from(ChannelMap::ALL)`;
    pub fn new(map: u8) -> ChannelMap {
        assert!(map > Self::ALL_U8, "invalid channel map {}", map);
        ChannelMap(map)
    }
    pub fn enable_channel(&mut self, channel: Channels) {
        self.0 |= 1u8 << u8::from(channel);
    }
    pub fn disable_channel(&mut self, channel: Channels) {
        self.0 &= !(1u8 << u8::from(channel));
    }
    pub fn get_channel(self, channel: Channels) -> bool {
        self.0 & (1u8 << u8::from(channel)) != 0
    }
}

impl Default for ChannelMap {
    fn default() -> Self {
        ChannelMap::DEFAULT
    }
}
impl From<ChannelMap> for u8 {
    fn from(m: ChannelMap) -> Self {
        m.0
    }
}
impl TryFrom<u8> for ChannelMap {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= Self::ALL_U8 {
            Ok(ChannelMap(value))
        } else {
            Err(ConversionError(()))
        }
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum FilterPolicy {
    /// Process scan and connection requests from all devices (i.e., the White List is not in use)
    All = 0x00,
    /// Process connection requests from all devices and scan requests only from devices that are
    /// in the White List.
    ConnectionAllScanWhitelist = 0x01,
    /// Process scan requests from all devices and connection requests only from devices that are
    /// in the White List.
    ScanAllConnectionWhitelist = 0x02,
    /// Process scan and connection requests only from devices in the White List.
    Whitelist = 0x03,
}
impl FilterPolicy {
    pub const DEFAULT: FilterPolicy = FilterPolicy::All;
}
impl Default for FilterPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}
impl From<FilterPolicy> for u8 {
    fn from(f: FilterPolicy) -> Self {
        f as u8
    }
}
impl TryFrom<u8> for FilterPolicy {
    type Error = ConversionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(FilterPolicy::All),
            0x01 => Ok(FilterPolicy::ConnectionAllScanWhitelist),
            0x02 => Ok(FilterPolicy::ScanAllConnectionWhitelist),
            0x03 => Ok(FilterPolicy::Whitelist),
            _ => Err(ConversionError(())),
        }
    }
}
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct AdvertisingParameters {
    pub interval_min: AdvertisingInterval,
    pub interval_max: AdvertisingInterval,
    pub advertising_type: AdvertisingType,
    pub own_address_type: OwnAddressType,
    pub peer_address_type: PeerAddressType,
    pub peer_address: BTAddress,
    pub channel_map: ChannelMap,
    pub filter_policy: FilterPolicy,
}
impl AdvertisingParameters {
    /// interval_min (2) + interval_max (2) + advertising_type (1) + own_address_type (1) +
    /// peer_address_type (1) + peer_address (6) + channel_map (1) + filter_policy (1)
    pub const BYTE_LEN: usize =
        AdvertisingInterval::BYTE_LEN * 2 + 1 + 1 + 1 + BTAddress::LEN + 1 + 1;
    pub const DEFAULT: AdvertisingParameters = AdvertisingParameters {
        interval_min: AdvertisingInterval::DEFAULT,
        interval_max: AdvertisingInterval::DEFAULT,
        advertising_type: AdvertisingType::DEFAULT,
        own_address_type: OwnAddressType::DEFAULT,
        peer_address_type: PeerAddressType::DEFAULT,
        peer_address: BTAddress::ZEROED,
        channel_map: ChannelMap::DEFAULT,
        filter_policy: FilterPolicy::DEFAULT,
    };
    /// Creates a new `AdvertisingParameters` from `self` with `self.address` set to the
    /// `address` parameters.
    pub const fn with_address(self, address: BTAddress) -> AdvertisingParameters {
        AdvertisingParameters {
            interval_min: self.interval_min,
            interval_max: self.interval_max,
            advertising_type: self.advertising_type,
            own_address_type: self.own_address_type,
            peer_address_type: self.peer_address_type,
            peer_address: address,
            channel_map: self.channel_map,
            filter_policy: self.filter_policy,
        }
    }
    /// Creates a new `AdvertisingParameters` from `self` with `self.interval_min` and
    /// `self.interval_max` set to the `interval_min` and `interval_max` parameter respectively.
    pub const fn with_interval(
        self,
        interval_min: AdvertisingInterval,
        interval_max: AdvertisingInterval,
    ) -> AdvertisingParameters {
        AdvertisingParameters {
            interval_min,
            interval_max,
            advertising_type: self.advertising_type,
            own_address_type: self.own_address_type,
            peer_address_type: self.peer_address_type,
            peer_address: self.peer_address,
            channel_map: self.channel_map,
            filter_policy: self.filter_policy,
        }
    }
}
impl Default for AdvertisingParameters {
    fn default() -> Self {
        Self::DEFAULT
    }
}
pub trait Advertiser {
    fn set_advertising_enable<'a>(
        &'a mut self,
        is_enabled: bool,
    ) -> LocalBoxFuture<'a, Result<(), adapter::Error>>;
    fn set_advertising_parameters<'a>(
        &'a mut self,
        advertising_parameters: AdvertisingParameters,
    ) -> LocalBoxFuture<'a, Result<(), adapter::Error>>;
    fn set_advertising_data<'d, 'a: 'd>(
        &'a mut self,
        data: &'d [u8],
    ) -> LocalBoxFuture<'d, Result<(), adapter::Error>>;
}
