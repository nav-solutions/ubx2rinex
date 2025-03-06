use ublox::{
    AlignmentToReferenceTime, CfgMsgAllPorts, CfgMsgAllPortsBuilder, CfgPrtUart, CfgPrtUartBuilder,
    CfgRate, CfgRateBuilder, DataBits, InProtoMask, MonGnss, MonVer, NavClock, NavEoe, NavPvt,
    NavSat, OutProtoMask, PacketRef, Parity, Parser, RxmRawx, StopBits, UartMode, UartPortId,
    UbxPacketMeta, UbxPacketRequest,
};

use serialport::SerialPort;
use std::time::Duration;

use crate::utils::{constell_mask_to_string, from_timescale};

use log::{debug, error, info};

use crate::{collecter::Message, UbloxSettings};

use tokio::sync::mpsc::Sender;

pub struct Device {
    pub port: Box<dyn SerialPort>,
    pub parser: Parser<Vec<u8>>,
}

impl Device {
    pub fn configure(&mut self, settings: &UbloxSettings, buf: &mut [u8], tx: &Sender<Message>) {
        self.read_version(buf, tx).unwrap();

        if settings.rx_clock {
            self.enable_nav_clock(buf);
        }

        self.enable_nav_eoe(buf);
        debug!("UBX-NAV-EOE enabled");

        self.enable_nav_pvt(buf);
        debug!("UBX-NAV-PVT enabled");

        self.enable_nav_sat(buf);
        debug!("UBX-NAV-SAT enabled");

        self.enable_obs_rinex(buf);

        let measure_rate_ms = (settings.sampling_period.total_nanoseconds() / 1_000_000) as u16;

        let time_ref = from_timescale(settings.timescale);

        self.apply_cfg_rate(buf, measure_rate_ms, settings.solutions_ratio, time_ref);
    }

    pub fn open(port_str: &str, baud: u32, buffer: &mut [u8]) -> Self {
        // open port
        let port = serialport::new(port_str, baud)
            .timeout(Duration::from_millis(250))
            .open()
            .unwrap_or_else(|e| panic!("Failed to open {} port: {}", port_str, e));

        let parser = Parser::default();
        let mut dev = Self { port, parser };

        for portid in [UartPortId::Usb, UartPortId::Uart1, UartPortId::Uart2] {
            // Enable UBX protocol on selected UART port
            dev
            .write_all(
                    &CfgPrtUartBuilder {
                        portid,
                        flags: 0,
                        tx_ready: 0,
                        reserved5: 0,
                        reserved0: 0,
                        baud_rate: baud,
                        in_proto_mask: InProtoMask::all(),
                        out_proto_mask: OutProtoMask::UBLOX,
                        mode: UartMode::new(DataBits::Eight, Parity::None, StopBits::One),
                    }
                    .into_packet_bytes(),
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to enable UBX streaming: {}. Invalid port or incorrect baud rate value.",
                        e
                    )
                });

            dev.wait_for_ack::<CfgPrtUart>(buffer).unwrap_or_else(|e| {
                panic!("CFG-MSG-UART NACK: {}", e);
            });
        }
        dev
    }

    pub fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.port.write_all(data)
    }

    // pub fn read_until_timeout(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    //     let size = self.read_port(buf)?;
    //     Ok(size)
    // }

    pub fn consume_all_cb<T: FnMut(PacketRef)>(
        &mut self,
        buffer: &mut [u8],
        mut cb: T,
    ) -> std::io::Result<()> {
        loop {
            let nbytes = self.read_port(buffer)?;
            if nbytes == 0 {
                break;
            }

            // parser.consume adds the buffer to its internal buffer, and
            // returns an iterator-like object we can use to process the packets
            let mut it = self.parser.consume(&buffer[..nbytes]);
            loop {
                match it.next() {
                    Some(Ok(packet)) => {
                        cb(packet);
                    },
                    Some(Err(e)) => {
                        error!("parsing error: {}", e);
                    },
                    None => {
                        // We've eaten all the packets we have
                        break;
                    },
                }
            }
        }
        Ok(())
    }

    pub fn wait_for_ack<T: UbxPacketMeta>(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        let mut found_packet = false;
        while !found_packet {
            self.consume_all_cb(buffer, |packet| {
                if let PacketRef::AckAck(ack) = packet {
                    if ack.class() == T::CLASS && ack.msg_id() == T::ID {
                        found_packet = true;
                    }
                }
            })?;
        }
        Ok(())
    }

    pub fn read_version(&mut self, buffer: &mut [u8], tx: &Sender<Message>) -> std::io::Result<()> {
        self.write_all(&UbxPacketRequest::request_for::<MonVer>().into_packet_bytes())
            .unwrap_or_else(|e| panic!("Failed to request firmware version: {}", e));

        let mut packet_found = false;

        while !packet_found {
            self.consume_all_cb(buffer, |packet| {
                if let PacketRef::MonVer(pkt) = packet {
                    let firmware = pkt.hardware_version();
                    debug!("U-Blox Software version: {}", pkt.software_version());
                    debug!("U-Blox Firmware version: {}", firmware);

                    tx.try_send(Message::FirmwareVersion(pkt.hardware_version().to_string()))
                        .unwrap_or_else(|e| {
                            panic!("internal error reading firmware version: {}", e)
                        });

                    packet_found = true;
                }
            })?;
        }

        Ok(())
    }

    pub fn apply_cfg_rate(
        &mut self,
        buffer: &mut [u8],
        measure_rate_ms: u16,
        nav_solutions_ratio: u16,
        time_ref: AlignmentToReferenceTime,
    ) {
        self.write_all(
            &CfgRateBuilder {
                measure_rate_ms,
                nav_rate: nav_solutions_ratio,
                time_ref,
            }
            .into_packet_bytes(),
        )
        .unwrap_or_else(|e| panic!("UBX-CFG-RATE: {}", e));

        self.wait_for_ack::<CfgRate>(buffer).unwrap_or_else(|e| {
            panic!("UBX-CFG-RATE NACK: {}", e);
        });
    }

    fn enable_obs_rinex(&mut self, buffer: &mut [u8]) {
        // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
        // The other positions are for I2C, SPI, etc. Consult your device manual.

        self.write_all(
            &CfgMsgAllPortsBuilder::set_rate_for::<RxmRawx>([1, 1, 1, 1, 1, 1]).into_packet_bytes(),
        )
        .unwrap_or_else(|e| panic!("UBX-RXM-RAWX error: {}", e));

        self.wait_for_ack::<CfgMsgAllPorts>(buffer)
            .unwrap_or_else(|e| panic!("UBX-RXM-RAWX error: {}", e));
    }

    fn enable_nav_eoe(&mut self, buffer: &mut [u8]) {
        // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
        // The other positions are for I2C, SPI, etc. Consult your device manual.

        self.write_all(
            &CfgMsgAllPortsBuilder::set_rate_for::<NavEoe>([1, 1, 1, 1, 1, 1]).into_packet_bytes(),
        )
        .unwrap_or_else(|e| panic!("UBX-NAV-EOE error: {}", e));

        self.wait_for_ack::<CfgMsgAllPorts>(buffer)
            .unwrap_or_else(|e| panic!("UBX-RXM-EOE error: {}", e));
    }

    fn enable_nav_clock(&mut self, buffer: &mut [u8]) {
        self.write_all(
            &CfgMsgAllPortsBuilder::set_rate_for::<NavClock>([1, 1, 1, 1, 1, 1])
                .into_packet_bytes(),
        )
        .unwrap_or_else(|e| panic!("UBX-NAV-CLK error: {}", e));

        self.wait_for_ack::<CfgMsgAllPorts>(buffer)
            .unwrap_or_else(|e| panic!("UBX-RXM-CLK error: {}", e));
    }

    pub fn enable_nav_sat(&mut self, buffer: &mut [u8]) {
        // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
        // The other positions are for I2C, SPI, etc. Consult your device manual.

        self.write_all(
            &CfgMsgAllPortsBuilder::set_rate_for::<NavSat>([1, 1, 1, 1, 1, 1]).into_packet_bytes(),
        )
        .unwrap_or_else(|e| panic!("UBX-NAV-SAT error: {}", e));

        self.wait_for_ack::<CfgMsgAllPorts>(buffer)
            .unwrap_or_else(|e| panic!("UBX-RXM-SAT error: {}", e));
    }

    pub fn enable_nav_pvt(&mut self, buffer: &mut [u8]) {
        // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
        // The other positions are for I2C, SPI, etc. Consult your device manual.

        self.write_all(
            &CfgMsgAllPortsBuilder::set_rate_for::<NavPvt>([1, 1, 1, 1, 1, 1]).into_packet_bytes(),
        )
        .unwrap_or_else(|e| panic!("UBX-NAV-PVT error: {}", e));

        self.wait_for_ack::<CfgMsgAllPorts>(buffer)
            .unwrap_or_else(|e| panic!("UBX-RXM-PVT error: {}", e));
    }

    pub fn read_gnss(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        self.write_all(&UbxPacketRequest::request_for::<MonGnss>().into_packet_bytes())
            .unwrap_or_else(|e| panic!("Failed to request firmware version: {}", e));

        let mut packet_found = false;
        while !packet_found {
            self.consume_all_cb(buffer, |packet| {
                if let PacketRef::MonGnss(pkt) = packet {
                    info!(
                        "Enabled constellations: {}",
                        constell_mask_to_string(pkt.enabled())
                    );
                    info!(
                        "Supported constellations: {}",
                        constell_mask_to_string(pkt.supported())
                    );
                    packet_found = true;
                }
            })?;
        }
        Ok(())
    }

    /// Reads the serial port, converting timeouts into "no data received"
    fn read_port(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self.port.read(output) {
            Ok(b) => Ok(b),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    Ok(0)
                } else {
                    Err(e)
                }
            },
        }
    }
}
