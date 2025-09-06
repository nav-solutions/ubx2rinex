use clap::{Arg, ArgAction, ArgMatches, ColorChoice, Command};
use rinex::prelude::{Constellation, Duration, Observable, TimeScale};

use crate::{
    collecter::settings::{HealthMask, Settings as RinexSettings},
    utils::SignalCarrier,
    UbloxSettings,
};

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
                    .next_help_heading("Constellation selection")
                    .arg(
                        Arg::new("gps")
                            .long("gps")
                            .action(ArgAction::SetTrue)
                            .help("Activate GPS constellation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "galileo", "beidou", "qzss", "glonass", "sbas", "irnss"]),
                    )
                    .arg(
                        Arg::new("galileo")
                            .long("galileo")
                            .action(ArgAction::SetTrue)
                            .help("Activate Galileo constellation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "gps", "beidou", "qzss", "glonass", "sbas", "irnss"]),
                    )
                    .arg(
                        Arg::new("bds")
                            .long("bds")
                            .action(ArgAction::SetTrue)
                            .help("Activate BDS (BeiDou) constellation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "galileo", "gps", "qzss", "glonass", "sbas", "irnss"]),
                    )
                    .arg(
                        Arg::new("qzss")
                            .long("qzss")
                            .action(ArgAction::SetTrue)
                            .help("Activate QZSS constellation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "galileo", "gps", "bds", "glonass", "sbas", "irnss"]),
                    )
                    .arg(
                        Arg::new("glonass")
                            .long("glonass")
                            .action(ArgAction::SetTrue)
                            .help("Activate Glonass constellation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "galileo", "gps", "bds", "qzss", "sbas", "irnss"]),
                    )
                    .arg(
                        Arg::new("sbas")
                            .long("sbas")
                            .action(ArgAction::SetTrue)
                            .help("Activate SBAS augmentation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "galileo", "gps", "bds", "qzss", "glonass", "irnss"]),
                    )
                    .arg(
                        Arg::new("irnss")
                            .long("irnss")
                            .action(ArgAction::SetTrue)
                            .help("Activate IRNSS/NAVIC constellation.
When working from UBX files, this serves as a data filter.")
                            .required_unless_present_any(["file", "galileo", "gps", "bds", "qzss", "glonass", "sbas"]),
                    )
                    .next_help_heading("Signal selection")
                    .arg(
                        Arg::new("l1")
                            .long("l1")
                            .action(ArgAction::SetTrue)
                            .help("Activate L1 signal for all constellations. Not required when operating from UBX files.")
                            .required_unless_present_any(["file", "l2", "l5"]),
                    )
                    .arg(
                        Arg::new("l2")
                            .long("l2")
                            .action(ArgAction::SetTrue)
                            .help("Activate L2 signal for all constellations. Not required when operating from UBX files.")
                            .required_unless_present_any(["file", "l1", "l5"]),
                    )
                    .arg(
                        Arg::new("l5")
                            .long("l5")
                            .action(ArgAction::SetTrue)
                            .help("Activate L5 signal for all constellations. Requires F9 or F10 series. Not required when operating from UBX files")
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
                            .value_name("Receiver model/name/label")
                            .help("Define the name or label of this receiver. Customizes your RINEX content. For example \"M8T\" when using an undefined M8-T device.")
                    )
                    .arg(
                        Arg::new("antenna")
                            .short('a')
                            .long("antenna")
                            .required(false)
                            .value_name("Receiver antenna model/name/label")
                            .help("Define the name or label of antenna attached to this receiver.
Customizes your RINEX content."))
                    .next_help_heading("File interface (Passive mode)")
                    .arg(
                        Arg::new("file")
                            .long("file")
                            .short('f')
                            .value_name("FILENAME")
                            .action(ArgAction::Append)
                            .required_unless_present_any(&["port"])
                            .help("Load a single UBX file. You can load as many as needed.
Each file descriptor is consumed one after the other (no priority). To obtain valid results,
you might have to load them in correct chronological order (sampling order).
Gzip compressed UBX files are natively supported but they must be terminated with '.gz'.
You still have to select the constellation you are interested in (at least one).
You don't have to select a signal.")
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
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Define snapshot (=collection) period.
The snapshot period defines the total duration of your RINEX file and how often it is released.
Our default snapshot period is set to 1 hour.
Modify this value to 24hours for standard daily files, with --period \"24 h\".
Other example, 12h period: --period \"12 h\".
Other example, half hour period: --period \"30 mins\".")
                    )
                    .arg(
                        Arg::new("v2")
                            .long("v2")
                            .action(ArgAction::SetTrue)
                            .help("Downgrade RINEX revision to V2. You can also upgrade to RINEX V4 with --v4.
We use V3 by default, because very few tools support V4 properly to this day.
You should not use --v2 with multi band devices (>M8).")
                    )
                    .arg(
                        Arg::new("v4")
                            .long("v4")
                            .action(ArgAction::SetTrue)
                            .help("Upgrade RINEX revision to V4. You can also downgrade to RINEX V2 with --v2.
We use V3 by default, because very few tools support V4 properly to this day.")
                    )
                    .arg(
                        Arg::new("long")
                            .short('l')
                            .long("long")
                            .action(ArgAction::SetTrue)
                            .help("Prefer long (V3 like) file names over short (V2) file names.
You must define a Country code to obtain a valid file name.")
                    )
                    .arg(
                        Arg::new("gzip")
                            .long("gzip")
                            .action(ArgAction::SetTrue)
                            .help("Gzip compress the RINEX output.
You can combine this to CRINEX compression for maximal signal storage effiency."))
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
                    .arg(
                        Arg::new("comment")
                            .long("comment")
                            .action(ArgAction::Set)
                            .required(false)
                            .help("Add one custom comment to your RINEX Header,
to be wrapped into several lines if it exceeds 60 characters."))
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
                    .next_help_heading("Navigation messages collection")
                            .arg(
                                Arg::new("nav")
                                    .long("nav")
                                    .required(false)
                                    .action(ArgAction::SetTrue)
                                    .help("Activate Navigation messages collection, which is not enabled by default.")
                            )
                            .arg(
                                Arg::new("nav-period")
                                    .long("nav-period")
                                    .required(false)
                                    .action(ArgAction::Set)
                                    .help("Define how often Navigation messages (ephemeris, etc..) are dumped into resulting RINEX.
Dumping period is always aligned to midnight. When time is reached, we dump the first message received of each kind (no fancy logic).
This value needs to be under the message validity for correct post processed navigation.
By default, we use a 2h message rate, which is more than enough considering all navigation messages.
But you can customize that, either to reduce the output file size, or increase the message rate.
Example --nav-period \"1 hour\" to reduce to 1hr message period.
Example --nav-period \"30 mins\" to reduce to 30min message period."))
                            .arg(
                                Arg::new("healthy-only")
                                    .long("healthy")
                                    .required(false)
                                    .action(ArgAction::SetTrue)
                                    .help("Dump messages for healthy satellites only.
This is currently limited to the Navigation message collection and does not impact signal collection."))
                            .arg(
                                Arg::new("unhealthy-only")
                                    .long("unhealthy")
                                    .required(false)
                                    .action(ArgAction::SetTrue)
                                    .help("Dump messages for unhealthy or beta-tested satellites only.
This is currently limited to the Navigation message collection and does not impact signal collection."))
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

    fn sbas(&self) -> bool {
        self.matches.get_flag("sbas")
    }

    fn irnss(&self) -> bool {
        self.matches.get_flag("irnss")
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
        if self.sbas() {
            constellations.push(Constellation::SBAS);
        }
        if self.irnss() {
            constellations.push(Constellation::IRNSS);
        }

        if self.serial_port().is_none() {
            // we're in passive mode
            if constellations.is_empty() {
                // no user choice: activate everything
                for constellation in [
                    Constellation::GPS,
                    Constellation::Galileo,
                    Constellation::QZSS,
                    Constellation::BeiDou,
                    Constellation::SBAS,
                    Constellation::Glonass,
                    Constellation::IRNSS,
                ] {
                    constellations.push(constellation);
                }
            }
        }

        constellations
    }

    fn l1(&self) -> bool {
        self.matches.get_flag("l1") | self.matches.contains_id("file")
    }

    fn l2(&self) -> bool {
        self.matches.get_flag("l2") | self.matches.contains_id("file")
    }

    fn l5(&self) -> bool {
        self.matches.get_flag("l5") | self.matches.contains_id("file")
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

        let mut gps_observables = vec![];
        let mut gal_observables = vec![];
        let mut glo_observables = vec![];
        let mut irnss_observables = vec![];
        let mut bds_observables = vec![];
        let mut sbas_observables = vec![];
        let mut qzss_observables = vec![];

        let mut ret = HashMap::<Constellation, Vec<Observable>>::new();

        let constellations = self.constellations();

        if self.l1() {
            if constellations.contains(&Constellation::GPS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L1_CA.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L1 observable");

                    gps_observables.push(observable);
                }

                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L1_CA.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-C1 observable");

                    gps_observables.push(observable);
                }

                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::GPS_L1_CA.to_doppler_observable(v2))
                            .expect("internal error: invalid GPS-D1 observable");

                    gps_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::Galileo) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GAL_E1_C.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GAL-L1 observable");

                    gal_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::GAL_E1_B.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GAL-L1 observable");

                    gal_observables.push(observable);
                }

                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GAL_E1_C.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GAL-C1 observable");

                    gal_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::GAL_E1_B.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GAL-C1 observable");

                    gal_observables.push(observable);
                }

                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::GAL_E1_C.to_doppler_observable(v2))
                            .expect("internal error: invalid GAL-D1 observable");

                    gal_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::GAL_E1_B.to_doppler_observable(v2))
                            .expect("internal error: invalid GAL-D1 observable");

                    gal_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::BeiDou) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B1I_D1.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B1I_D2.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B1I_D1.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B1I_D2.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::BDS_B1I_D1.to_doppler_observable(v2))
                            .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::BDS_B1I_D2.to_doppler_observable(v2))
                            .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::SBAS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::SBAS_L1_CA.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid SBAS-L1 observable");

                    sbas_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::SBAS_L1_CA.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid SBAS-C1 observable");

                    sbas_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::SBAS_L1_CA.to_doppler_observable(v2))
                            .expect("internal error: invalid SBAS-C1 observable");

                    sbas_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::QZSS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L1_CA.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-L1 observable");

                    qzss_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L1_S.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-L1 observable");

                    qzss_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L1_CA.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-C1 observable");

                    qzss_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L1_S.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-C1 observable");

                    qzss_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::QZSS_L1_CA.to_doppler_observable(v2))
                            .expect("internal error: invalid QZSS-D1 observable");

                    qzss_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::QZSS_L1_S.to_doppler_observable(v2))
                            .expect("internal error: invalid QZSS-D1 observable");

                    qzss_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::Glonass) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GLO_L1_OF.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GLO-L1 observable");

                    glo_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GLO_L1_OF.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GLO-C1 observable");

                    glo_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::GLO_L1_OF.to_doppler_observable(v2))
                            .expect("internal error: invalid GLO-D1 observable");

                    glo_observables.push(observable);
                }
            }
        }

        if self.l2() {
            if constellations.contains(&Constellation::GPS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L2_CL.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L2 observable");

                    gps_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L2_CM.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L2 observable");

                    gps_observables.push(observable);
                }

                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L2_CL.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L2 observable");

                    gps_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L2_CM.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L2 observable");

                    gps_observables.push(observable);
                }

                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::GPS_L2_CL.to_doppler_observable(v2))
                            .expect("internal error: invalid GPS-D2 observable");

                    gps_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::GPS_L2_CM.to_doppler_observable(v2))
                            .expect("internal error: invalid GPS-D2 observable");

                    gps_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::BeiDou) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B2I_D1.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B2 observable");

                    bds_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B2I_D2.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B2 observable");

                    bds_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B2I_D1.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::BDS_B2I_D2.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid BDS-B1 observable");

                    bds_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::BDS_B2I_D1.to_doppler_observable(v2))
                            .expect("internal error: invalid BDS-B2 observable");

                    bds_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::BDS_B2I_D2.to_doppler_observable(v2))
                            .expect("internal error: invalid BDS-B2 observable");

                    bds_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::QZSS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L2_CL.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-L2 observable");

                    qzss_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L2_CM.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-L2 observable");

                    qzss_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L2_CL.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-C2 observable");

                    qzss_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L2_CM.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-C2 observable");

                    qzss_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::QZSS_L2_CL.to_doppler_observable(v2))
                            .expect("internal error: invalid QZSS-D2 observable");

                    qzss_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::QZSS_L2_CM.to_doppler_observable(v2))
                            .expect("internal error: invalid QZSS-D2 observable");

                    qzss_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::Glonass) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GLO_L2_OF.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GLO-L2 observable");

                    glo_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GLO_L2_OF.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GLO-C2 observable");

                    glo_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::GLO_L2_OF.to_doppler_observable(v2))
                            .expect("internal error: invalid GLO-D2 observable");

                    glo_observables.push(observable);
                }
            }
        }

        if self.l5() {
            if constellations.contains(&Constellation::GPS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L5_I.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L5 observable");

                    gps_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L5_Q.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-L5 observable");

                    gps_observables.push(observable);
                }

                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L5_I.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-C5 observable");

                    gps_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::GPS_L5_Q.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid GPS-C5 observable");

                    gps_observables.push(observable);
                }

                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::GPS_L5_I.to_doppler_observable(v2))
                            .expect("internal error: invalid GPS-D5 observable");

                    gps_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::GPS_L5_Q.to_doppler_observable(v2))
                            .expect("internal error: invalid GPS-D5 observable");

                    gps_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::QZSS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L5_I.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-L5 observable");

                    qzss_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L5_Q.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-L5 observable");

                    qzss_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L5_I.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-C5 observable");

                    qzss_observables.push(observable);

                    let observable = Observable::from_str(
                        &SignalCarrier::QZSS_L5_Q.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid QZSS-C5 observable");

                    qzss_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::QZSS_L5_I.to_doppler_observable(v2))
                            .expect("internal error: invalid QZSS-D5 observable");

                    qzss_observables.push(observable);

                    let observable =
                        Observable::from_str(&SignalCarrier::QZSS_L5_Q.to_doppler_observable(v2))
                            .expect("internal error: invalid QZSS-D5 observable");

                    qzss_observables.push(observable);
                }
            }

            if constellations.contains(&Constellation::IRNSS) {
                if !self.no_phase() {
                    let observable = Observable::from_str(
                        &SignalCarrier::NAVIC_L5_A.to_phase_range_observable(v2),
                    )
                    .expect("internal error: invalid NAVIC-L5 observable");

                    irnss_observables.push(observable);
                }
                if !self.no_pr() {
                    let observable = Observable::from_str(
                        &SignalCarrier::NAVIC_L5_A.to_pseudo_range_observable(v2),
                    )
                    .expect("internal error: invalid NAVIC-C5 observable");

                    irnss_observables.push(observable);
                }
                if !self.no_dop() {
                    let observable =
                        Observable::from_str(&SignalCarrier::NAVIC_L5_A.to_doppler_observable(v2))
                            .expect("internal error: invalid NAVIC-D5 observable");

                    irnss_observables.push(observable);
                }
            }
        }

        for observable in gps_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::GPS) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::GPS, vec![observable.clone()]);
            }
        }

        for observable in gal_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::Galileo) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::Galileo, vec![observable.clone()]);
            }
        }

        for observable in sbas_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::SBAS) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::SBAS, vec![observable.clone()]);
            }
        }

        for observable in bds_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::BeiDou) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::BeiDou, vec![observable.clone()]);
            }
        }

        for observable in qzss_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::QZSS) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::QZSS, vec![observable.clone()]);
            }
        }

        for observable in glo_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::Glonass) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::Glonass, vec![observable.clone()]);
            }
        }

        for observable in irnss_observables.iter() {
            if let Some(observables) = ret.get_mut(&Constellation::IRNSS) {
                observables.push(observable.clone());
            } else {
                ret.insert(Constellation::IRNSS, vec![observable.clone()]);
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
            antenna: if let Some(antenna) = self.matches.get_one::<String>("antenna") {
                Some(antenna.to_string())
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
            header_comment: if let Some(comment) = self.matches.get_one::<String>("comment") {
                Some(comment.to_string())
            } else {
                None
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
                period.trim().parse::<Duration>().unwrap_or_else(|e| {
                    panic!("not a valid duration: {}", e);
                })
            } else {
                Duration::from_hours(1.0)
            },
            nav_period: if let Some(period) = self.matches.get_one::<String>("nav-period") {
                period.trim().parse::<Duration>().unwrap_or_else(|e| {
                    panic!("not a valid duration: {}", e);
                })
            } else {
                Duration::from_hours(2.0)
            },
            health_mask: {
                if self.matches.get_flag("healthy-only") {
                    HealthMask::HealthyOnly
                } else if self.matches.get_flag("unhealthy-only") {
                    HealthMask::UnhealthyOnly
                } else {
                    HealthMask::Any
                }
            },
        }
    }
}
