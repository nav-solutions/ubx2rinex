use log::{debug, error};

use ublox::{
    Parser, UbxPacketMeta, UbxPacketRequest, UbxProtocol,
    cfg_msg::{CfgMsgAllPorts, CfgMsgAllPortsBuilder},
    cfg_prt::{
        CfgPrtUart, CfgPrtUartBuilder, DataBits, InProtoMask, OutProtoMask, Parity, StopBits,
        UartMode, UartPortId,
    },
    cfg_rate::{AlignmentToReferenceTime, CfgRate, CfgRateBuilder},
    mon_ver::MonVer,
    nav_clock::NavClock,
    nav_other::NavEoe,
    nav_sat::NavSat,
    rxm_rawx::RxmRawx,
    rxm_sfrbx::RxmSfrbx,
};

#[cfg(feature = "proto23")]
use ublox::packetref_proto23::PacketRef;

#[cfg(feature = "proto23")]
use ublox::nav_pvt::proto23::NavPvt;

#[cfg(all(feature = "proto27", not(feature = "proto23")))]
use ublox::packetref_proto27::PacketRef;

#[cfg(all(feature = "proto27", not(feature = "proto23")))]
use ublox::nav_pvt::proto27::NavPvt;

#[cfg(all(
    feature = "proto31",
    not(any(feature = "proto23", feature = "proto27"))
))]
use ublox::packetref_proto31::PacketRef;

#[cfg(all(feature = "proto27", not(feature = "proto23")))]
use ublox::nav_pvt::proto31::NavPvt;

mod interface;

use interface::Interface;

use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
    time::Duration,
};

use crate::{UbloxSettings, collecter::Message, utils::from_timescale};

use tokio::sync::mpsc::Sender;

pub struct Device<P: UbxProtocol> {
    pub interface: Interface,
    pub parser: Parser<Vec<u8>, P>,
}

impl<P: UbxProtocol> Device<P> {
    pub fn configure(&mut self, settings: &UbloxSettings, buf: &mut [u8], tx: Sender<Message>) {
        let mut vec = Vec::with_capacity(1024);

        self.read_version(buf, tx).unwrap();

        if settings.rx_clock {
            self.enable_nav_clock(buf);
        }

        self.enable_nav_eoe(buf);
        self.enable_nav_pvt(buf);
        self.enable_nav_sat(buf);

        self.enable_obs_rinex(settings.rawxm, buf);
        self.enable_rxm_sfrbx(settings.ephemeris, buf);

        let time_ref = from_timescale(settings.timescale);

        let measure_rate_ms = (settings.sampling_period.total_nanoseconds() / 1_000_000) as u16;
        self.apply_cfg_rate(buf, measure_rate_ms, settings.solutions_ratio, time_ref);

        settings.to_ram_volatile_cfg(&mut vec);

        self.write_all(&vec)
            .unwrap_or_else(|e| panic!("Failed to apply RAM config: {}", e));
    }

    pub fn open_file(fullpath: &str) -> Self {
        let handle = File::open(fullpath).unwrap_or_else(|e| {
            panic!("Failed to open {}: {}", fullpath, e);
        });

        Self {
            parser: Parser::<_, P>::new(vec![]),
            interface: if fullpath.ends_with(".gz") {
                Interface::from_gzip_file_handle(handle)
            } else {
                Interface::from_file_handle(handle)
            },
        }
    }

    pub fn open_serial_port(port_str: &str, baud: u32, buffer: &mut [u8]) -> Self {
        // open port
        let port = serialport::new(port_str, baud)
            .timeout(Duration::from_millis(250))
            .open()
            .unwrap_or_else(|e| panic!("Failed to open {} port: {}", port_str, e));

        let mut device = Self {
            parser: Parser::<_, P>::new(vec![]),
            interface: Interface::from_serial_port(port),
        };

        for portid in [UartPortId::Uart1, UartPortId::Uart2] {
            // Enable UBX protocol on selected UART port
            device
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

            device
                .wait_for_ack::<CfgPrtUart>(buffer)
                .unwrap_or_else(|e| {
                    panic!("CFG-MSG-UART NACK: {}", e);
                });
        }

        device
    }

    pub fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.interface.write_all(data)
    }

    // pub fn read_until_timeout(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
    //     let size = self.read_port(buf)?;
    //     Ok(size)
    // }

    /// Consume all potential UBX packets.
    ///
    /// ## Returns
    /// - Ok(0) once all packets were consumed (no packet present)
    /// - Ok(n) with n=number of packets that were consumed (not bytes)
    /// - Err(e) on I/O error
    pub fn consume_all_cb<T: FnMut(PacketRef)>(
        &mut self,
        buffer: &mut [u8],
        mut cb: T,
    ) -> std::io::Result<usize> {
        let mut total = 0;

        loop {
            let nbytes = self.read_interface(buffer)?;
            if nbytes == 0 {
                return Ok(0);
            }

            // parser.consume adds the buffer to its internal buffer, and
            // returns an iterator-like object we can use to process the packets
            let mut it = self.parser.consume_ubx(&buffer[..nbytes]);

            loop {
                match it.next() {
                    Some(Ok(packet)) => {
                        cb(packet);
                        total += 1;
                    },
                    Some(Err(e)) => {
                        error!("UBX parsing error: {}", e);
                    },
                    None => {
                        // consumed all packets
                        return Ok(total);
                    },
                }
            }
        }
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

    // pub fn request_mga_gps_eph(&mut self) {
    //     match self.write_all(&UbxPacketRequest::request_for::<MgaGpsEph>().into_packet_bytes()) {
    //         Ok(_) => {
    //             debug!("MGA-GPS-EPH");
    //         },
    //         Err(e) => {
    //             error!("Failed to request MGA-GPS-EPH: {}", e);
    //         },
    //     }
    // }

    // pub fn request_mga_glonass_eph(&mut self) {
    //     match self.write_all(&UbxPacketRequest::request_for::<MgaGloEph>().into_packet_bytes()) {
    //         Ok(_) => {
    //             debug!("MGA-GLO-EPH");
    //         },
    //         Err(e) => {
    //             error!("Failed to request MGA-GLO-EPH: {}", e);
    //         },
    //     }
    // }

    pub fn read_version(&mut self, buffer: &mut [u8], tx: Sender<Message>) -> std::io::Result<()> {
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

    fn enable_rxm_sfrbx(&mut self, enable: bool, buffer: &mut [u8]) {
        let msg = if enable {
            // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
            // The other positions are for I2C, SPI, etc. Consult your device manual.
            CfgMsgAllPortsBuilder::set_rate_for::<RxmSfrbx>([1, 1, 1, 1, 1, 1])
        } else {
            CfgMsgAllPortsBuilder::set_rate_for::<RxmSfrbx>([0, 0, 0, 0, 0, 0])
        };

        self.write_all(&msg.into_packet_bytes())
            .unwrap_or_else(|e| panic!("UBX-RXM-SFRBX error: {}", e));

        self.wait_for_ack::<CfgMsgAllPorts>(buffer)
            .unwrap_or_else(|e| panic!("UBX-RXM-SFRBX error: {}", e));
    }

    fn enable_obs_rinex(&mut self, enable: bool, buffer: &mut [u8]) {
        let msg = if enable {
            // By setting 1 in the array below, we enable the NavPvt message for Uart1, Uart2 and USB
            // The other positions are for I2C, SPI, etc. Consult your device manual.
            CfgMsgAllPortsBuilder::set_rate_for::<RxmRawx>([1, 1, 1, 1, 1, 1])
        } else {
            CfgMsgAllPortsBuilder::set_rate_for::<RxmRawx>([0, 0, 0, 0, 0, 0])
        };

        self.write_all(&msg.into_packet_bytes())
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

        debug!("UBX-NAV-EOE enabled");
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

        debug!("UBX-NAV-SAT enabled");
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

        debug!("UBX-NAV-PVT enabled");
    }

    // pub fn read_gnss(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
    //     self.write_all(&UbxPacketRequest::request_for::<MonGnss>().into_packet_bytes())
    //         .unwrap_or_else(|e| panic!("Failed to request firmware version: {}", e));

    //     let mut packet_found = false;
    //     while !packet_found {
    //         self.consume_all_cb(buffer, |packet| {
    //             if let PacketRef::MonGnss(pkt) = packet {
    //                 info!(
    //                     "Enabled constellations: {}",
    //                     constell_mask_to_string(pkt.enabled())
    //                 );
    //                 info!(
    //                     "Supported constellations: {}",
    //                     constell_mask_to_string(pkt.supported())
    //                 );
    //                 packet_found = true;
    //             }
    //         })?;
    //     }
    //     Ok(())
    // }

    /// Reads internal [Interface], converting timeouts into "No Data Received",
    /// which is most convenient for real-time perpertual hardware application like this one.
    fn read_interface(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self.interface.read(output) {
            Ok(b) => Ok(b),
            Err(e) => {
                if e.kind() == ErrorKind::TimedOut {
                    Ok(0)
                } else {
                    Err(e)
                }
            },
        }
    }
}
