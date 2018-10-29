pub enum HCICommand {
    LinkControl(LinkControl),
    ControllerAndBaseband(ControllerAndBaseband),
    InformationParameters(InformationParameters),
    StatusParameters(StatusParameters),
    LEController(LEController),
}

impl HCICommand {
    pub fn as_opcode_pair(&self) -> OpCodePair {
        match *self {
            HCICommand::LinkControl(ref ocf) => ocf.as_opcode_pair(),
            HCICommand::ControllerAndBaseband(ref ocf) => ocf.as_opcode_pair(),
            HCICommand::InformationParameters(ref ocf) => ocf.as_opcode_pair(),
            HCICommand::StatusParameters(ref ocf) => ocf.as_opcode_pair(),
            HCICommand::LEController(ref ocf) => ocf.as_opcode_pair(),
        }
    }
}

/// An type for the pair of OGF (OpCode Group Field) and OCF (OpCode Command Field)
pub struct OpCodePair {
    pub ogf: u16,
    pub ocf: u16,
}

pub enum LinkControl {
    Disconnect,
    ReadRemoteVersionInformation,
}

impl LinkControl {
    const OGF: u16 = 0x1;

    #[inline]
    fn as_opcode_pair(&self) -> OpCodePair {
        use self::LinkControl::*;

        OpCodePair {
            ogf: LinkControl::OGF,
            ocf: match *self {
                Disconnect => 0x6,
                ReadRemoteVersionInformation => 0x1d,
            }
        }
    }
}

pub enum ControllerAndBaseband {
    Reset,
    ReadTransmitPowerLevel,
}

impl ControllerAndBaseband {
    const OGF: u16 = 0x3;

    #[inline]
    fn as_opcode_pair(&self) -> OpCodePair {
        use self::ControllerAndBaseband::*;

        OpCodePair {
            ogf: ControllerAndBaseband::OGF,
            ocf: match *self {
                Reset => 0x3,
                ReadTransmitPowerLevel => 0x2d,
            }
        }
    }
}

pub enum InformationParameters {
    ReadLocalSupportedVersionInformation,
    ReadLocalSupportedCommands,
    ReadLocalSupportedFeatures,
    #[allow(non_camel_case_types)] ReadBD_ADDR,
}

impl InformationParameters {
    const OGF: u16 = 0x4;

    #[inline]
    fn as_opcode_pair(&self) -> OpCodePair {
        use self::InformationParameters::*;

        OpCodePair {
            ogf: InformationParameters::OGF,
            ocf: match *self {
                ReadLocalSupportedVersionInformation => 0x1,
                ReadLocalSupportedCommands => 0x2,
                ReadLocalSupportedFeatures => 0x3,
                ReadBD_ADDR => 0x9,
            }
        }
    }
}

pub enum StatusParameters {
    ReadRSSI,
}

impl StatusParameters {
    const OGF: u16 = 0x5;

    #[inline]
    fn as_opcode_pair(&self) -> OpCodePair {
        use self::StatusParameters::*;

        OpCodePair {
            ogf: StatusParameters::OGF,
            ocf: match *self {
                ReadRSSI => 0x5,
            }
        }
    }
}

pub enum LEController {
    AddDeviceToWhiteList,
    ClearWhiteList,
    ReadBufferSize,
    ReadLocalSupportedFeatures,
    ReadSupportedStates,
    ReadWhiteListSize,
    RemoveDeviceFromWhiteList,
    SetEventMask,
    TestEnd,
    ReceiverTest,
    SetScanEnable,
    SetScanParameters,
    ReadAdvertisingChannelTxPower,
    SetAdvertisingData,
    SetAdvertisingEnable,
    SetAdvertisingParameters,
    SetRandomAddress,
    TransmitterTest,
    ConnectionUpdate,
    CreateConnection,
    CreateConnectionCancel,
    ReadChannelMap,
    ReadRemoteFeatures,
    SetHostChannelClassification,
}

impl LEController {
    const OGF: u16 = 0x8;

    #[inline]
    fn as_opcode_pair( &self ) -> OpCodePair{
        use self::LEController::*;

        OpCodePair {
            ogf: LEController::OGF,
            ocf: match *self {
                AddDeviceToWhiteList => 0x11,
                ClearWhiteList => 0x10,
                ReadBufferSize => 0x2,
                ReadLocalSupportedFeatures => 0x3,
                ReadSupportedStates => 0x1c,
                ReadWhiteListSize => 0xf,
                RemoveDeviceFromWhiteList => 0x12,
                SetEventMask => 0x1,
                TestEnd => 0x1f,
                ReceiverTest => 0x1d,
                SetScanEnable => 0x5,
                SetScanParameters => 0xb,
                ReadAdvertisingChannelTxPower => 0x7,
                SetAdvertisingData => 0x8,
                SetAdvertisingEnable => 0xa,
                SetAdvertisingParameters => 0x6,
                SetRandomAddress => 0x5,
                TransmitterTest => 0x1e,
                ConnectionUpdate => 0x13,
                CreateConnection => 0x5,
                CreateConnectionCancel => 0xe,
                ReadChannelMap => 0x15,
                ReadRemoteFeatures => 0x16,
                SetHostChannelClassification => 0x14,
            }
        }
    }
}
