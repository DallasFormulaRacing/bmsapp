use std::io;
use std::sync::Arc;

use embedded_can::{ExtendedId, Frame as _, Id};
use socketcan::frame::FdFlags;
use socketcan::tokio::CanFdSocket;
use socketcan::{CanAnyFrame, CanFdFrame};

use super::*;

#[derive(Debug, thiserror::Error)]
pub enum CanSocketError {
    #[error("CAN socket IO error: {0}")]
    Io(#[from] io::Error),

    #[error("failed to construct socketcan extended ID from raw ID 0x{0:08X}")]
    InvalidExtendedId(u32),

    #[error("failed to construct CAN FD frame")]
    InvalidFdFrame,
}

#[derive(Clone)]
pub struct CanSocket {
    socket: Arc<CanFdSocket>,
}

impl CanSocket {
    pub fn open(interface: &str) -> Result<Self, CanSocketError> {
        let socket = CanFdSocket::open(interface)?;
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub async fn read_message(&self) -> Result<Option<DfrCanMessageBuf>, CanSocketError> {
        let frame = self.socket.read_frame().await?;
        Ok(DfrCanMessageBuf::try_from(frame).ok())
    }

    pub async fn write_message(&self, message: &DfrCanMessageBuf) -> Result<(), CanSocketError> {
        self.write_raw(message.id, message.data.as_slice()).await
    }

    pub async fn write_raw(&self, id: DfrCanId, data: &[u8]) -> Result<(), CanSocketError> {
        let raw_id = id.to_raw_id();
        let extended_id =
            ExtendedId::new(raw_id).ok_or(CanSocketError::InvalidExtendedId(raw_id))?;
        let frame = CanFdFrame::with_flags(extended_id, data, FdFlags::empty())
            .ok_or(CanSocketError::InvalidFdFrame)?;

        self.socket.write_frame(&frame).await?;
        Ok(())
    }
}

impl<'a> TryFrom<&'a CanFdFrame> for DfrCanMessage<'a> {
    type Error = CanMessageError;

    fn try_from(frame: &'a CanFdFrame) -> Result<Self, Self::Error> {
        let id = match frame.id() {
            Id::Extended(id) => DfrCanId::try_from(id.as_raw())?,
            _ => return Err(CanMessageError::NotExtendedId),
        };

        Ok(DfrCanMessage {
            id,
            data: frame.data(),
        })
    }
}

impl TryFrom<CanAnyFrame> for DfrCanMessageBuf {
    type Error = CanMessageError;

    fn try_from(frame: CanAnyFrame) -> Result<Self, Self::Error> {
        let CanAnyFrame::Fd(frame) = frame else {
            return Err(CanMessageError::NonFdFrame);
        };

        Ok(DfrCanMessage::try_from(&frame)?.to_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_can::{ExtendedId, StandardId};
    use socketcan::CanDataFrame;

    fn raw_id(priority: u8, target: CanNode, command: CanCommand, source: CanNode) -> u32 {
        DfrCanId {
            priority,
            target,
            source,
            command,
        }
        .to_raw_id()
    }

    fn fd_frame(
        priority: u8,
        target: CanNode,
        command: CanCommand,
        source: CanNode,
        data: &[u8],
    ) -> CanFdFrame {
        let raw_id = raw_id(priority, target, command, source);
        let id = ExtendedId::new(raw_id).expect("raw DFR ID should be a valid extended ID");
        CanFdFrame::new(id, data).expect("payload should fit in a CAN FD frame")
    }

    #[test]
    fn parses_fd_frame_into_dfr_message() {
        let frame = fd_frame(
            1,
            CanNode::Raspi,
            CanCommand::Common(CommonCanCommand::Pong),
            CanNode::Bms,
            &[0xAA, 0xBB],
        );

        let message = DfrCanMessage::try_from(&frame).expect("frame should parse");

        assert_eq!(message.id.priority, 1);
        assert_eq!(message.id.target, CanNode::Raspi);
        assert_eq!(message.id.source, CanNode::Bms);
        assert_eq!(
            message.id.command,
            CanCommand::Common(CommonCanCommand::Pong)
        );
        assert_eq!(message.data, &[0xAA, 0xBB]);
    }

    #[test]
    fn parses_any_fd_frame_into_owned_dfr_message() {
        let frame = fd_frame(
            2,
            CanNode::FrontLeft,
            CanCommand::Daq(DaqCanCommand::SpeedData),
            CanNode::Raspi,
            &[1, 2, 3, 4],
        );

        let message = DfrCanMessageBuf::try_from(CanAnyFrame::Fd(frame))
            .expect("FD frame should parse into owned DFR message");

        assert_eq!(message.id.priority, 2);
        assert_eq!(message.id.target, CanNode::FrontLeft);
        assert_eq!(message.id.source, CanNode::Raspi);
        assert_eq!(
            message.id.command,
            CanCommand::Daq(DaqCanCommand::SpeedData)
        );
        assert_eq!(message.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn rejects_non_fd_frame() {
        let raw_id = raw_id(
            1,
            CanNode::Raspi,
            CanCommand::Common(CommonCanCommand::Ping),
            CanNode::Bms,
        );
        let id = ExtendedId::new(raw_id).expect("raw DFR ID should be a valid extended ID");
        let classic_frame = CanDataFrame::new(id, &[1, 2, 3]).expect("classic frame");

        let error = DfrCanMessageBuf::try_from(CanAnyFrame::Normal(classic_frame))
            .expect_err("classic CAN frames should be discarded");

        assert_eq!(error, CanMessageError::NonFdFrame);
    }

    #[test]
    fn rejects_standard_id_fd_frame() {
        let id = StandardId::new(0x123).expect("valid standard ID");
        let frame = CanFdFrame::new(id, &[1, 2, 3]).expect("FD frame");

        let error = DfrCanMessage::try_from(&frame).expect_err("standard IDs are not DFR IDs");

        assert_eq!(error, CanMessageError::NotExtendedId);
    }

    #[test]
    fn rejects_unknown_node_id() {
        let raw_id = ((1_u32) << 26)
            | ((0x12_u32) << 21)
            | ((u16::from(CanCommand::Common(CommonCanCommand::Ping)) as u32) << 5)
            | u8::from(CanNode::Bms) as u32;
        let id = ExtendedId::new(raw_id).expect("raw ID should still be a valid extended ID");
        let frame = CanFdFrame::new(id, &[]).expect("FD frame");

        let error = DfrCanMessage::try_from(&frame).expect_err("unknown node should be rejected");

        assert_eq!(error, CanMessageError::UnknownNode(0x12));
    }

    #[test]
    fn rejects_unknown_command_id() {
        let raw_id = ((1_u32) << 26)
            | ((u8::from(CanNode::Raspi) as u32) << 21)
            | ((0x1234_u32) << 5)
            | u8::from(CanNode::Bms) as u32;
        let id = ExtendedId::new(raw_id).expect("raw ID should still be a valid extended ID");
        let frame = CanFdFrame::new(id, &[]).expect("FD frame");

        let error =
            DfrCanMessage::try_from(&frame).expect_err("unknown command should be rejected");

        assert_eq!(error, CanMessageError::UnknownCommand(0x1234));
    }

    #[test]
    #[ignore = "fill in once BMS voltage payload layout is known"]
    fn parses_bms_voltage_payload_skeleton() {
        let frame = fd_frame(
            1,
            CanNode::Raspi,
            CanCommand::Bms(BmsCanCommand::BatteryPackData),
            CanNode::Bms,
            &[
                // TODO: replace with real firmware bytes for BMS pack/min/max/average voltage.
            ],
        );

        let message = DfrCanMessage::try_from(&frame).expect("frame ID should parse");

        assert_eq!(
            message.id.command,
            CanCommand::Bms(BmsCanCommand::BatteryPackData)
        );
        // TODO: pass `message` into the future BMS payload parser and assert decoded values.
    }

    #[test]
    #[ignore = "fill in once DAQ speed payload layout is known"]
    fn parses_daq_speed_payload_skeleton() {
        let frame = fd_frame(
            1,
            CanNode::Raspi,
            CanCommand::Daq(DaqCanCommand::SpeedData),
            CanNode::FrontLeft,
            &[
                // TODO: make sample data
            ],
        );

        let message = DfrCanMessage::try_from(&frame).expect("frame ID should parse");

        assert_eq!(
            message.id.command,
            CanCommand::Daq(DaqCanCommand::SpeedData)
        );
        // TODO: pass `message` into the future DAQ payload parser and assert decoded values.
    }
}
