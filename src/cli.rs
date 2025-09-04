use clap::{Arg, ArgAction, ArgMatches, ColorChoice, Command};
use rinex::prelude::{Constellation, Duration, Observable, TimeScale};

use crate::{collecter::settings::Settings as RinexSettings, UbloxSettings};

use std::{collections::HashMap, str::FromStr};

pub struct Cli {
    /// Arguments passed by user
    matches: ArgMatches,
}

impl Cli {
    /// Build new command line interface
    pub fn new() -> Self {
        Self {
            matches: {
                Command::new("ubx2rinex")
                    .author("Guillaume W. Bres, <guillaume.bressaix@gmail.com>")
                    .version(env!("CARGO_PKG_VERSION"))
                    .about("U-Blox stream to RINEX collecter")
                    .color(ColorChoice::Always)
                    .arg_required_else_help(true)
                    .next_help_heading("Serial port (Active device, GNSS module)")
                    .arg(
                        Arg::new("port")
                            .short('p')
                            .long("port")
                            .value_name("PORT")
                            .required_unless_present_any(&["file"])
                            .help("Define serial port. Example /dev/ttyUSB0 on Linux")
                    )
                    .arg(
                        Arg::new("baudrate")
                            .short('b')
                            .long("baud")
                            .required(false)
                            .value_name("Baudrate (u32)")
                            .help("Define serial port baud rate. Communications will not work if your U-Blox streams at a different data-rate. By default we use 115_200"),
                    )
                    .next_help_heading("Constellation selection - at lease one required!")
                    .arg(
                        Arg::new("gps")
                            .long("gps")
                            .action(ArgAction::SetTrue)
                            .help("Activate GPS constellation")
                            .required_unless_present_any(["file", "galileo", "beidou", "qzss", "glonass"]),
                    )
                    .arg(
                        Arg::new("galileo")
                            .long("galileo")
                            .action(ArgAction::SetTrue)
                            .help("Activate Galileo constellation")
                            .required_unless_present_any(["file", "gps", "beidou", "qzss", "glonass"]),
                    )
                    .arg(
                        Arg::new("bds")
                            .long("bds")
                            .action(ArgAction::SetTrue)
                            .help("Activate BDS (BeiDou) constellation")
                            .required_unless_present_any(["file", "galileo", "gps", "qzss", "glonass"]),
                    )
                    .arg(
                        Arg::new("qzss")
                            .long("qzss")
                            .action(ArgAction::SetTrue)
                            .help("Activate QZSS constellation")
                            .required_unless_present_any(["file", "galileo", "gps", "bds", "glonass"]),
                    )
                    .arg(
                        Arg::new("glonass")
                            .long("glonass")
                            .action(ArgAction::SetTrue)
                            .help("Activate Glonass constellation")
                            .required_unless_present_any(["file", "galileo", "gps", "bds", "qzss"]),
                    )
                    .next_help_heading("Signal selection - at least one required!")
                    .arg(
                        Arg::new("l1")
                            .long("l1")
                            .action(ArgAction::SetTrue)
                            .help("Activate L1 signal for all constellations")
                            .required_unless_present_any(["file", "l2", "l5"]),
                    )
                    .arg(
                        Arg::new("l2")
                            .long("l2")
                            .action(ArgAction::SetTrue)
                            .help("Activate L2 signal for all constellations")
                            .required_unless_present_any(["file", "l1", "l5"]),
                    )
                    .arg(
                        Arg::new("l5")
                            .long("l5")
                            .action(ArgAction::SetTrue)
                            .help("Activate L5 signal for all constellations. Requires F9 or F10 series.")
                            .required_unless_present_any(["file", "l1", "l2"]),
                    )
                    .next_help_heading("U-Blox configuration")
                    .arg(
                        Arg::new("profile")
                            .long("prof")
                            .action(ArgAction::Set)
                            .help("Define user profile. Default is set to \"portable\""),
                    )
                    .arg(
                        Arg::new("rx-clock")
                            .long("rx-clock")
                            .action(ArgAction::SetTrue)
                            .help("Resolve clock state and capture it. Disabled by default"),
                    )
                    .arg(
                        Arg::new("anti-spoofing")
                            .long("anti-spoofing")
                            .action(ArgAction::SetTrue)
                            .help("Makes sure anti jamming/spoofing is enabled. When enabled, it is automatically emphasized in the collected RINEX."))
                    .arg(
                        Arg::new("model")
                            .short('m')
                            .long("model")
                            .required(false)
                            .value_name("Model")
                            .help("Define u-Blox receiver model. For example \"u-Blox M8T\"")
                    )
                    .next_help_heading("File interface (Passive mode)")
                    .arg(
                        Arg::new("file")
                            .long("file")
                            .short('f')
                            .value_name("FILENAME")
                            .action(ArgAction::Append)
                            .required_unless_present_any(&["port"])
                            .help("Load a single file. Use as many as needed.
Each file descriptor is consumed one after the other. You might have to load them according
to their sampling chronology to make sure. Gzip file are supported but they must be terminated with '.gz'")
                    )
                    .next_help_heading("RINEX Collection")
                    .arg(
                        Arg::new("name")
                            .long("name")
                            .short('n')
                            .required(false)
                            .action(ArgAction::Set)
                            .help("Define a custom name. To respect standard naming conventions,
this should be a 4 letter code, usually named after your geodetic marker.
When not defined, the default value is \"UBXR\".")
                    )
                    .arg(
                        Arg::new("prefix")
                            .long("prefix")
                            .required(false)
                            .help("Custom directory prefix for output products. Default is none!"),
                    )
                    .arg(
                        Arg::new("period")
                            .long("period")
                            .short('p')
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Define snapshot (=collection) mode")
                    )
                    .arg(
                        Arg::new("v2")
                            .long("v2")
                            .action(ArgAction::SetTrue)
                            .help("Downgrade RINEX revision to V2. You can also upgrade to RINEX V4 with --v4.
We use V3 by default, because very few tools support V4, so we remain compatible.")
                    )
                    .arg(
                        Arg::new("v4")
                            .long("v4")
                            .action(ArgAction::SetTrue)
                            .help("Upgrade RINEX revision to V4. You can also downgrade to RINEX V2 with --v2.
We use V3 by default, because very few tools support V4, so we remain compatible.")
                    )
                    .arg(
                        Arg::new("long")
                            .short('l')
                            .action(ArgAction::SetTrue)
                            .help("Prefer long (V3 like) file names over short (V2) file names")
                    )
                    .arg(
                        Arg::new("country")
                            .short('c')
                            .action(ArgAction::Set)
                            .help("Specify country code (3 letter) in case of V3 file name. Default: \"FRA\"")
                    )
                    .arg(
                        Arg::new("agency")
                            .long("agency")
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Define name of your Agency, to be used in all Headers"),
                    )
                    .arg(
                        Arg::new("observer")
                            .long("observer")
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Define name of Observer, to be used in all Headers"),
                    )
                    .arg(
                        Arg::new("operator")
                            .long("operator")
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Define name of Operator, to be used in all Headers"),
                    )
                    .next_help_heading("Observations collection (signal sampling)")
                    .arg(
                        Arg::new("no-obs")
                            .long("no-obs")
                            .action(ArgAction::SetTrue)
                            .help("Disable Observation RINEX collection. You can use this if you intend to collect Ephemerides only for example"),
                    )
                    .arg(
                        Arg::new("sampling")
                            .short('s')
                            .long("sampling")
                            .required(false)
                            .help("Define sampling interval. Default value is 30s (standard low-rate RINEX).")
                    )
                    .arg(
                        Arg::new("no-phase")
                            .long("no-phase")
                            .action(ArgAction::SetTrue)
                            .help("Do not track signal phase")
                    )
                    .arg(
                        Arg::new("no-pr")
                            .long("no-pr")
                            .action(ArgAction::SetTrue)
                            .help("Do not decode pseudo range")
                    )
                    .arg(
                        Arg::new("no-dop")
                            .long("no-dop")
                            .action(ArgAction::SetTrue)
                            .help("Do not track doppler shifts")
                    )
                    .arg(
                        Arg::new("timescale")
                            .long("timescale")
                            .required(false)
                            .help("Express your observations in given Timescale.
Default value is GPST."
                    ))
                    .arg(
                        Arg::new("crx")
                            .long("crx")
                            .action(ArgAction::SetTrue)
                            .help("Activate CRINEX compression, for optimized RINEX size. Disabled by default."),
                    )
                    .arg(
                        Arg::new("gzip")
                            .long("gzip")
                            .action(ArgAction::SetTrue)
                            .help("Gzip compress the RINEX output.
You can combine this to CRINEX compression for effiency."))
                    .next_help_heading("Navigation messages collection")
                            .arg(
                                Arg::new("nav")
                                    .long("nav")
                                    .required(false)
                                    .action(ArgAction::SetTrue)
                                    .help("Activate Navigation messages collection, which is not enabled by default.")
                            )
                    .get_matches()
            },
        }
    }

    /// Returns User serial port specification
    pub fn serial_port(&self) -> Option<&String> {
        self.matches.get_one::<String>("port")
    }

    /// Input file paths
    pub fn filepaths(&self) -> Vec<&String> {
        if let Some(fp) = self.matches.get_many::<String>("file") {
            fp.collect()
        } else {
            Vec::new()
        }
    }

    /// Returns User baud rate specification
    pub fn baud_rate(&self) -> Option<u32> {
        let baud = self.matches.get_one::<String>("baudrate")?;
        let baud = baud
            .parse::<u32>()
            .unwrap_or_else(|e| panic!("Invalid baud rate value: {}", e));
        Some(baud)
    }

    fn gps(&self) -> bool {
        self.matches.get_flag("gps")
    }

    fn galileo(&self) -> bool {
        self.matches.get_flag("galileo")
    }

    fn bds(&self) -> bool {
        self.matches.get_flag("bds")
    }

    fn qzss(&self) -> bool {
        self.matches.get_flag("qzss")
    }

    fn glonass(&self) -> bool {
        self.matches.get_flag("glonass")
    }

    fn constellations(&self) -> Vec<Constellation> {
        let mut constellations = Vec::<Constellation>::with_capacity(4);

        if self.gps() {
            constellations.push(Constellation::GPS);
        }
        if self.galileo() {
            constellations.push(Constellation::Galileo);
        }
        if self.bds() {
            constellations.push(Constellation::BeiDou);
        }
        if self.qzss() {
            constellations.push(Constellation::QZSS);
        }
        if self.glonass() {
            constellations.push(Constellation::Glonass);
        }
        constellations
    }

    fn l1(&self) -> bool {
        self.matches.get_flag("l1")
    }

    fn l2(&self) -> bool {
        self.matches.get_flag("l2")
    }

    fn l5(&self) -> bool {
        self.matches.get_flag("l5")
    }

    fn no_dop(&self) -> bool {
        self.matches.get_flag("no-dop")
    }

    fn no_pr(&self) -> bool {
        self.matches.get_flag("no-pr")
    }

    fn no_phase(&self) -> bool {
        self.matches.get_flag("no-phase")
    }

    fn observables(&self) -> HashMap<Constellation, Vec<Observable>> {
        let v2 = self.matches.get_flag("v2");
        let mut ret = HashMap::new();

        for constell in self.constellations().iter() {
            if self.l1() {
                let mut values = match constell {
                    Constellation::GPS
                    | Constellation::Glonass
                    | Constellation::Galileo
                    | Constellation::QZSS => {
                        if v2 {
                            vec![
                                Observable::from_str("C1").unwrap(),
                                Observable::from_str("D1").unwrap(),
                                Observable::from_str("L1").unwrap(),
                            ]
                        } else {
                            vec![
                                Observable::from_str("C1C").unwrap(),
                                Observable::from_str("D1C").unwrap(),
                                Observable::from_str("L1C").unwrap(),
                            ]
                        }
                    },
                    _ => {
                        vec![]
                    },
                };

                if self.no_dop() {
                    values.retain(|code| !code.is_doppler_observable());
                }
                if self.no_phase() {
                    values.retain(|code| !code.is_phase_range_observable());
                }
                if self.no_pr() {
                    values.retain(|code| !code.is_pseudo_range_observable());
                }
                if !values.is_empty() {
                    ret.insert(*constell, values);
                }
            }

            if self.l2() {
                let mut values = match constell {
                    Constellation::GPS | Constellation::Glonass | Constellation::QZSS => {
                        if v2 {
                            vec![
                                Observable::from_str("C2").unwrap(),
                                Observable::from_str("D2").unwrap(),
                                Observable::from_str("L2").unwrap(),
                            ]
                        } else {
                            vec![
                                Observable::from_str("C2C").unwrap(),
                                Observable::from_str("D2C").unwrap(),
                                Observable::from_str("L2C").unwrap(),
                            ]
                        }
                    },
                    _ => {
                        vec![]
                    },
                };

                if self.no_dop() {
                    values.retain(|code| !code.is_doppler_observable());
                }
                if self.no_phase() {
                    values.retain(|code| !code.is_phase_range_observable());
                }
                if self.no_pr() {
                    values.retain(|code| !code.is_pseudo_range_observable());
                }
                if !values.is_empty() {
                    ret.insert(*constell, values);
                }
            }

            if self.l5() {
                let mut values = match constell {
                    Constellation::GPS | Constellation::Galileo | Constellation::QZSS => {
                        if v2 {
                            vec![
                                Observable::from_str("C5").unwrap(),
                                Observable::from_str("D5").unwrap(),
                                Observable::from_str("L5").unwrap(),
                            ]
                        } else {
                            vec![
                                Observable::from_str("C5C").unwrap(),
                                Observable::from_str("D5C").unwrap(),
                                Observable::from_str("L5C").unwrap(),
                            ]
                        }
                    },
                    _ => {
                        vec![]
                    },
                };

                if self.no_dop() {
                    values.retain(|code| !code.is_doppler_observable());
                }
                if self.no_phase() {
                    values.retain(|code| !code.is_phase_range_observable());
                }
                if self.no_pr() {
                    values.retain(|code| !code.is_pseudo_range_observable());
                }
                if !values.is_empty() {
                    ret.insert(*constell, values);
                }
            }
        }
        ret
    }

    fn timescale(&self) -> TimeScale {
        if let Some(ts) = self.matches.get_one::<String>("timescale") {
            let ts = TimeScale::from_str(ts.trim())
                .unwrap_or_else(|e| panic!("Invalid timescale: {}", e));
            ts
        } else {
            TimeScale::GPST
        }
    }

    fn sampling_period(&self) -> Duration {
        if let Some(sampling) = self.matches.get_one::<String>("sampling") {
            let dt = sampling
                .trim()
                .parse::<Duration>()
                .unwrap_or_else(|e| panic!("Invalid duration: {}", e));

            if dt.total_nanoseconds() < 50_000_000 {
                panic!("Sampling period is limited to 50ms");
            }
            dt
        } else {
            Duration::from_milliseconds(30_000.0)
        }
    }

    fn solutions_ratio(sampling_period: Duration) -> u16 {
        let period_ms = (sampling_period.total_nanoseconds() / 1_000_000) as u16;
        if period_ms > 10_000 {
            1
        } else if period_ms > 1_000 {
            2
        } else {
            10
        }
    }

    pub fn ublox_settings(&self) -> UbloxSettings {
        let sampling_period = self.sampling_period();
        UbloxSettings {
            l1: self.l1(),
            l2: self.l2(),
            l5: self.l5(),
            sampling_period,
            rawxm: !self.matches.get_flag("no-obs"),
            ephemeris: self.matches.get_flag("nav"),
            timescale: self.timescale(),
            constellations: self.constellations(),
            rx_clock: self.matches.get_flag("rx-clock"),
            solutions_ratio: Self::solutions_ratio(sampling_period),
            sn: None,
            firmware: None,
            model: if let Some(model) = self.matches.get_one::<String>("model") {
                Some(model.to_string())
            } else {
                None
            },
        }
    }

    pub fn rinex_settings(&self) -> RinexSettings {
        RinexSettings {
            short_filename: !self.matches.get_flag("long"),
            gzip: self.matches.get_flag("gzip"),
            crinex: self.matches.get_flag("crx"),
            timescale: self.timescale(),
            observables: self.observables(),
            major: if self.matches.get_flag("v4") {
                4
            } else if self.matches.get_flag("v2") {
                2
            } else {
                3
            },
            country: if let Some(country) = self.matches.get_one::<String>("country") {
                country.to_string()
            } else {
                "FRA".to_string()
            },
            agency: if let Some(agency) = self.matches.get_one::<String>("agency") {
                Some(agency.to_string())
            } else {
                None
            },
            operator: if let Some(operator) = self.matches.get_one::<String>("operator") {
                Some(operator.to_string())
            } else {
                None
            },
            prefix: if let Some(prefix) = self.matches.get_one::<String>("prefix") {
                Some(prefix.to_string())
            } else {
                None
            },
            name: if let Some(name) = self.matches.get_one::<String>("name") {
                name.to_string()
            } else {
                "UBXR".to_string()
            },
            period: if let Some(period) = self.matches.get_one::<String>("period") {
                let dt = period
                    .trim()
                    .parse::<Duration>()
                    .unwrap_or_else(|e| panic!("Invalid duration: {}", e));

                dt
            } else {
                Duration::from_hours(1.0)
            },
        }
    }
}
