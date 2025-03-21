use clap::{Arg, ArgAction, ArgMatches, ColorChoice, Command};
use rinex::prelude::{Constellation, Duration, TimeScale};

use crate::{collecter::settings::Settings as RinexSettings, UbloxSettings};

use std::str::FromStr;

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
                    .next_help_heading("Serial port")
                    .arg(
                        Arg::new("port")
                            .short('p')
                            .long("port")
                            .value_name("PORT")
                            .required(true)
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
                    .next_help_heading("U-Blox configuration")
                    .arg(
                        Arg::new("gps")
                            .long("gps")
                            .action(ArgAction::SetTrue)
                            .help("Activate GPS constellation (at least one required).")
                            .required_unless_present_any(["galileo", "beidou", "qzss", "glonass"]),
                    )
                    .arg(
                        Arg::new("galileo")
                            .long("galileo")
                            .action(ArgAction::SetTrue)
                            .help("Activate Galileo constellation (at least one required).")
                            .required_unless_present_any(["gps", "beidou", "qzss", "glonass"]),
                    )
                    .arg(
                        Arg::new("bds")
                            .long("bds")
                            .action(ArgAction::SetTrue)
                            .help("Activate BDS (BeiDou) constellation (at least one required).")
                            .required_unless_present_any(["galileo", "gps", "qzss", "glonass"]),
                    )
                    .arg(
                        Arg::new("qzss")
                            .long("qzss")
                            .action(ArgAction::SetTrue)
                            .help("Activate QZSS constellation (at least one required).")
                            .required_unless_present_any(["galileo", "gps", "bds", "glonass"]),
                    )
                    .arg(
                        Arg::new("glonass")
                            .long("glonass")
                            .action(ArgAction::SetTrue)
                            .help("Activate Glonass constellation (at least one required).")
                            .required_unless_present_any(["galileo", "gps", "bds", "qzss"]),
                    )
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
                        Arg::new("nav")
                            .long("nav")
                            .action(ArgAction::SetTrue)
                            .help("Activate Navigation RINEX collection. Use this to collect NAV RINEX file(s). File type is closely tied to enabled Constellation(s)."),
                    )
                    .arg(
                        Arg::new("no-obs")
                            .long("no-obs")
                            .action(ArgAction::SetTrue)
                            .help("Disable Observation RINEX collection. You can use this if you intend to collect Ephemerides only for example"),
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
                    .next_help_heading("Observation collection (signal sampling)")
                    .arg(
                        Arg::new("sampling")
                            .short('s')
                            .long("sampling")
                            .required(false)
                            .help("Define sampling interval. Default value is 30s (standard low-rate RINEX).")
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
                            .help("Activate Gzip compression."))
                    .get_matches()
            },
        }
    }

    /// Returns User serial port specification
    pub fn port(&self) -> &str {
        self.matches.get_one::<String>("port").unwrap()
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
            observables: Default::default(),
            sampling_period,
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
            obs_rinex: true,
            nav_rinex: false,
            short_filename: !self.matches.get_flag("long"),
            gzip: self.matches.get_flag("gzip"),
            crinex: self.matches.get_flag("crx"),
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
