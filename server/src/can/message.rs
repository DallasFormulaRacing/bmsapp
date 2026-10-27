#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfrCanMessage<'a> {
    pub id: DfrCanId,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DfrCanMessageBuf {
    pub id: DfrCanId,
    pub data: Vec<u8>,
}

impl DfrCanMessageBuf {
    pub fn as_message(&self) -> DfrCanMessage<'_> {
        DfrCanMessage {
            id: self.id,
            data: self.data.as_slice(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DfrCanId {
    pub priority: u8,
    pub target: CanNode,
    pub source: CanNode,
    pub command: CanCommand,
}

impl DfrCanId {
    pub fn new(
        priority: u8,
        target: CanNode,
        source: CanNode,
        command: CanCommand,
    ) -> Result<Self, CanMessageError> {
        if priority > 0x07 {
            return Err(CanMessageError::InvalidPriority(priority));
        }

        Ok(Self {
            priority,
            target,
            source,
            command,
        })
    }

    pub fn to_raw_id(self) -> u32 {
        ((self.priority as u32) << 26)
            | ((u8::from(self.target) as u32) << 21)
            | ((u16::from(self.command) as u32) << 5)
            | u8::from(self.source) as u32
    }
}

impl TryFrom<u32> for DfrCanId {
    type Error = CanMessageError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > 0x1FFF_FFFF {
            return Err(CanMessageError::InvalidRawId(value));
        }

        let priority = ((value >> 26) & 0x07) as u8;
        let target = ((value >> 21) & 0x1F) as u8;
        let command = ((value >> 5) & 0xFFFF) as u16;
        let source = (value & 0x1F) as u8;

        Ok(Self {
            priority,
            target: CanNode::try_from(target)?,
            source: CanNode::try_from(source)?,
            command: CanCommand::try_from(command)?,
        })
    }
}

impl From<DfrCanId> for u32 {
    fn from(value: DfrCanId) -> Self {
        value.to_raw_id()
    }
}

impl<'a> DfrCanMessage<'a> {
    pub fn new(
        priority: u8,
        target: CanNode,
        source: CanNode,
        command: CanCommand,
        data: &'a [u8],
    ) -> Result<Self, CanMessageError> {
        Ok(Self {
            id: DfrCanId::new(priority, target, source, command)?,
            data,
        })
    }

    pub fn to_buf(self) -> DfrCanMessageBuf {
        DfrCanMessageBuf {
            id: self.id,
            data: self.data.to_vec(),
        }
    }
}

impl<'a> TryFrom<(u32, &'a [u8])> for DfrCanMessage<'a> {
    type Error = CanMessageError;

    fn try_from((id, data): (u32, &'a [u8])) -> Result<Self, Self::Error> {
        Ok(Self {
            id: DfrCanId::try_from(id)?,
            data,
        })
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum CanMessageError {
    #[error("raw CAN ID 0x{0:08X} is not a 29-bit extended ID")]
    InvalidRawId(u32),

    #[error("CAN priority {0} is out of range")]
    InvalidPriority(u8),

    #[error("unknown CAN node 0x{0:02X}")]
    UnknownNode(u8),

    #[error("unknown CAN command 0x{0:04X}")]
    UnknownCommand(u16),

    #[error("CAN frame is not a CAN FD frame")]
    NonFdFrame,

    #[error("CAN frame does not use a 29-bit extended ID")]
    NotExtendedId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanNode {
    AllNodes,
    FrontLeft,
    FrontRight,
    RearLeft,
    RearRight,
    Nucleo1,
    Nucleo2,
    Vcu,
    Bms,
    Dash,
    Raspi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanSystem {
    Bms,
    Daq,
    Vcu,
}

impl CanNode {
    pub fn system(self) -> Option<CanSystem> {
        match self {
            Self::Bms => Some(CanSystem::Bms),
            Self::Vcu => Some(CanSystem::Vcu),
            Self::FrontLeft
            | Self::FrontRight
            | Self::RearLeft
            | Self::RearRight
            | Self::Nucleo1
            | Self::Nucleo2
            | Self::Dash => Some(CanSystem::Daq),
            Self::AllNodes | Self::Raspi => None,
        }
    }
}

impl TryFrom<u8> for CanNode {
    type Error = CanMessageError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::AllNodes),
            0x02 => Ok(Self::FrontLeft),
            0x03 => Ok(Self::FrontRight),
            0x04 => Ok(Self::RearLeft),
            0x05 => Ok(Self::RearRight),
            0x06 => Ok(Self::Nucleo1),
            0x07 => Ok(Self::Nucleo2),
            0x1B => Ok(Self::Vcu),
            0x1F => Ok(Self::Bms),
            0x1D => Ok(Self::Dash),
            0x1E => Ok(Self::Raspi),
            other => Err(CanMessageError::UnknownNode(other)),
        }
    }
}

impl From<CanNode> for u8 {
    fn from(value: CanNode) -> Self {
        match value {
            CanNode::AllNodes => 0x01,
            CanNode::FrontLeft => 0x02,
            CanNode::FrontRight => 0x03,
            CanNode::RearLeft => 0x04,
            CanNode::RearRight => 0x05,
            CanNode::Nucleo1 => 0x06,
            CanNode::Nucleo2 => 0x07,
            CanNode::Vcu => 0x1B,
            CanNode::Bms => 0x1F,
            CanNode::Dash => 0x1D,
            CanNode::Raspi => 0x1E,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanCommand {
    Common(CommonCanCommand),
    Daq(DaqCanCommand),
    Bms(BmsCanCommand),
    Bootloader(BootloaderCanCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonCanCommand {
    Ping,
    Pong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaqCanCommand {
    ReqImuData,
    ReqTempData,
    ReqSpeedData,
    ReqRideHeightData,
    ImuData,
    TempData,
    SpeedData,
    RideHeightData,
    SetLed,
    ResetNode,
    ReqUuid,
    ReqFwVer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsCanCommand {
    CellVoltagesPack1,
    CellVoltagesPack2,
    CellVoltagesPack3,
    CellVoltagesPack4,
    CellVoltagesPack5,
    CellVoltagesPack6,
    SegmentTempsHalf1,
    SegmentTempsHalf2,
    BatteryPackData,
    ImdData,
    /// Raspi -> BMS: enable charging session (DLC 0).
    GuiConnect,
    /// Raspi -> BMS: setpoints - u16 BE volts, u16 BE amps, u16 BE delta mV (DLC 6).
    GuiChargingRequest,
    /// Raspi -> BMS: disable charging session (DLC 0).
    GuiDisconnect,
    /// Raspi -> BMS: soft-stop charge (DLC 0).
    GuiChargingSoftstop,
    CurrentSensor,
    ImdRequest,
    ImdResponse,
    ImdGeneral,
    ImdIsoDetail,
    ImdVoltage,
    ImdItSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootloaderCanCommand {
    Erase,
    EraseOk,
    Write,
    WriteOk,
    AddrSize,
    FwQuery,
    FwResp,
    Reboot,
    Jump,
}

impl TryFrom<u16> for CanCommand {
    type Error = CanMessageError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0xA001 => Ok(Self::Common(CommonCanCommand::Ping)),
            0xA002 => Ok(Self::Common(CommonCanCommand::Pong)),

            0xD101 => Ok(Self::Daq(DaqCanCommand::ReqImuData)),
            0xD102 => Ok(Self::Daq(DaqCanCommand::ReqTempData)),
            0xD103 => Ok(Self::Daq(DaqCanCommand::ReqSpeedData)),
            0xD104 => Ok(Self::Daq(DaqCanCommand::ReqRideHeightData)),
            0xD201 => Ok(Self::Daq(DaqCanCommand::ImuData)),
            0xD202 => Ok(Self::Daq(DaqCanCommand::TempData)),
            0xD203 => Ok(Self::Daq(DaqCanCommand::SpeedData)),
            0xD204 => Ok(Self::Daq(DaqCanCommand::RideHeightData)),
            0xD301 => Ok(Self::Daq(DaqCanCommand::SetLed)),
            0xDF01 => Ok(Self::Daq(DaqCanCommand::ResetNode)),
            0xDF02 => Ok(Self::Daq(DaqCanCommand::ReqUuid)),
            0xDF03 => Ok(Self::Daq(DaqCanCommand::ReqFwVer)),

            0x00A0 => Ok(Self::Bms(BmsCanCommand::CellVoltagesPack1)),
            0x00A1 => Ok(Self::Bms(BmsCanCommand::CellVoltagesPack2)),
            0x00A2 => Ok(Self::Bms(BmsCanCommand::CellVoltagesPack3)),
            0x00A3 => Ok(Self::Bms(BmsCanCommand::CellVoltagesPack4)),
            0x00A4 => Ok(Self::Bms(BmsCanCommand::CellVoltagesPack5)),
            0x00A5 => Ok(Self::Bms(BmsCanCommand::CellVoltagesPack6)),
            0x00B0 => Ok(Self::Bms(BmsCanCommand::SegmentTempsHalf1)),
            0x00B1 => Ok(Self::Bms(BmsCanCommand::SegmentTempsHalf2)),
            0x00C0 => Ok(Self::Bms(BmsCanCommand::BatteryPackData)),
            0x00D0 => Ok(Self::Bms(BmsCanCommand::ImdData)),
            0x00E1 => Ok(Self::Bms(BmsCanCommand::GuiConnect)),
            0x00E0 => Ok(Self::Bms(BmsCanCommand::GuiChargingRequest)),
            0x00E2 => Ok(Self::Bms(BmsCanCommand::GuiDisconnect)),
            0x00E2 => Ok(Self::Bms(BmsCanCommand::GuiChargingSoftstop)),
            // GuiDisconnect and GuiChargingSoftstop do the same thing (disable charging session)
            0xBEEF => Ok(Self::Bms(BmsCanCommand::CurrentSensor)),
            0xBF22 => Ok(Self::Bms(BmsCanCommand::ImdRequest)),
            0xBF23 => Ok(Self::Bms(BmsCanCommand::ImdResponse)),
            0xBF37 => Ok(Self::Bms(BmsCanCommand::ImdGeneral)),
            0xBF38 => Ok(Self::Bms(BmsCanCommand::ImdIsoDetail)),
            0xBF39 => Ok(Self::Bms(BmsCanCommand::ImdVoltage)),
            0xBF3A => Ok(Self::Bms(BmsCanCommand::ImdItSystem)),

            0xF001 => Ok(Self::Bootloader(BootloaderCanCommand::Erase)),
            0xF002 => Ok(Self::Bootloader(BootloaderCanCommand::EraseOk)),
            0xF003 => Ok(Self::Bootloader(BootloaderCanCommand::Write)),
            0xF004 => Ok(Self::Bootloader(BootloaderCanCommand::WriteOk)),
            0xF005 => Ok(Self::Bootloader(BootloaderCanCommand::AddrSize)),
            0xF006 => Ok(Self::Bootloader(BootloaderCanCommand::FwQuery)),
            0xF007 => Ok(Self::Bootloader(BootloaderCanCommand::FwResp)),
            0xF008 => Ok(Self::Bootloader(BootloaderCanCommand::Reboot)),
            0xFAAA => Ok(Self::Bootloader(BootloaderCanCommand::Jump)),

            other => Err(CanMessageError::UnknownCommand(other)),
        }
    }
}

impl From<CanCommand> for u16 {
    fn from(value: CanCommand) -> Self {
        match value {
            CanCommand::Common(command) => command.into(),
            CanCommand::Daq(command) => command.into(),
            CanCommand::Bms(command) => command.into(),
            CanCommand::Bootloader(command) => command.into(),
        }
    }
}

impl From<CommonCanCommand> for u16 {
    fn from(value: CommonCanCommand) -> Self {
        match value {
            CommonCanCommand::Ping => 0xA001,
            CommonCanCommand::Pong => 0xA002,
        }
    }
}

impl From<DaqCanCommand> for u16 {
    fn from(value: DaqCanCommand) -> Self {
        match value {
            DaqCanCommand::ReqImuData => 0xD101,
            DaqCanCommand::ReqTempData => 0xD102,
            DaqCanCommand::ReqSpeedData => 0xD103,
            DaqCanCommand::ReqRideHeightData => 0xD104,
            DaqCanCommand::ImuData => 0xD201,
            DaqCanCommand::TempData => 0xD202,
            DaqCanCommand::SpeedData => 0xD203,
            DaqCanCommand::RideHeightData => 0xD204,
            DaqCanCommand::SetLed => 0xD301,
            DaqCanCommand::ResetNode => 0xDF01,
            DaqCanCommand::ReqUuid => 0xDF02,
            DaqCanCommand::ReqFwVer => 0xDF03,
        }
    }
}

impl From<BmsCanCommand> for u16 {
    fn from(value: BmsCanCommand) -> Self {
        match value {
            BmsCanCommand::CellVoltagesPack1 => 0x00A0,
            BmsCanCommand::CellVoltagesPack2 => 0x00A1,
            BmsCanCommand::CellVoltagesPack3 => 0x00A2,
            BmsCanCommand::CellVoltagesPack4 => 0x00A3,
            BmsCanCommand::CellVoltagesPack5 => 0x00A4,
            BmsCanCommand::CellVoltagesPack6 => 0x00A5,
            BmsCanCommand::SegmentTempsHalf1 => 0x00B0,
            BmsCanCommand::SegmentTempsHalf2 => 0x00B1,
            BmsCanCommand::BatteryPackData => 0x00C0,
            BmsCanCommand::ImdData => 0x00D0,
            BmsCanCommand::GuiConnect => 0x00E1,
            BmsCanCommand::GuiChargingRequest => 0x00E0,
            BmsCanCommand::GuiDisconnect => 0x00E2,
            BmsCanCommand::GuiChargingSoftstop => 0x00E2,
            BmsCanCommand::CurrentSensor => 0xBEEF,
            BmsCanCommand::ImdRequest => 0xBF22,
            BmsCanCommand::ImdResponse => 0xBF23,
            BmsCanCommand::ImdGeneral => 0xBF37,
            BmsCanCommand::ImdIsoDetail => 0xBF38,
            BmsCanCommand::ImdVoltage => 0xBF39,
            BmsCanCommand::ImdItSystem => 0xBF3A,
        }
    }
}

impl From<BootloaderCanCommand> for u16 {
    fn from(value: BootloaderCanCommand) -> Self {
        match value {
            BootloaderCanCommand::Erase => 0xF001,
            BootloaderCanCommand::EraseOk => 0xF002,
            BootloaderCanCommand::Write => 0xF003,
            BootloaderCanCommand::WriteOk => 0xF004,
            BootloaderCanCommand::AddrSize => 0xF005,
            BootloaderCanCommand::FwQuery => 0xF006,
            BootloaderCanCommand::FwResp => 0xF007,
            BootloaderCanCommand::Reboot => 0xF008,
            BootloaderCanCommand::Jump => 0xFAAA,
        }
    }
}
