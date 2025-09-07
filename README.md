UBX2RINEX
=========

[![Rust](https://github.com/nav-solutions/ubx2rinex/actions/workflows/rust.yml/badge.svg)](https://github.com/nav-solutions/ubx2rinex/actions/workflows/rust.yml)
[![Rust](https://github.com/nav-solutions/ubx2rinex/actions/workflows/daily.yml/badge.svg)](https://github.com/nav-solutions/ubx2rinex/actions/workflows/daily.yml)
[![crates.io](https://img.shields.io/crates/v/ubx2rinex.svg)](https://crates.io/crates/ubx2rinex)

[![MRSV](https://img.shields.io/badge/MSRV-1.82.0-orange?style=for-the-badge)](https://github.com/rust-lang/rust/releases/tag/1.82.0)
[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/nav-solutions/ubx2rinex/blob/main/LICENSE)

`ubx2rinex` is a small command line utility to deserialize
a U-Blox data stream into standardized RINEX file(s).

## Licensing

This application is part of the [nav-solutions framework](https://github.com/nav-solutions) and
is licensed under [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.

## Constellations

Supported constellations: 

- GPS (US)
- Galileo (EU)
- Glonass (RU)
- BeiDou (CH)
- QZSS (Japan)
- IRNSS/NAV-IC (India)
- SBAS

## Signals

Supported signals/modulations:

- GPS L1 C/A
- SBAS L1 C/A
- QZSS L1 C/A
- Galileo E1 (C+B)
- Glonass L1
- GPS L2 (M+L)
- QZSS L2 (M+L)
- Glonass L2
- Galileo E5A (I+Q)
- Galileo E5B (I+Q)
- QZSS L5 (I+Q)
- BDS B1i (D1+D2)
- BDS B2i (D1+D2)
- BDS B1C
- BDS B2A
- QZSS L1 S
- IRNSS/NAV-IC L5 (A)

## U-Blox protocol

Supported UBX protocol versions

- V14
- V23 (Default) 
- V27
- V31

You need to select one at least. When building with default features, you obtain the default UBX protocol.
You cannot build this application with `--all-features` until further notice, because you can only select 
one particular revision of the UBX protocol.

## Install from Cargo

Install the latest official release from the worldwide portal directly:

```bash
cargo install ubx2rinex
```

# Build from sources

Download the version you are interested in:

```bash
git clone https://github.com/nav-solutions/ubx2rinex
```

Grab our test data if you're interested in the .UBX files:

```bash
git clone --recurse-submodules https://github.com/nav-solutions/ubx2rinex
```

Build the latest version with default UBX protocol:

```bash
cargo build -r
```

Example: build for protocol v31 specifically

```bash
cargo build -r --no-default-features --features ubx31
```

## Getting started

This application works from a UBX (U-Blox protocol) stream. It can be managed and
handled in real-time, when connected to a U-Blox receiver (we call this the active mode),
or be a UBX snapshot (so called UBX files) that you previously capteured, we call this the passive mode.

Connecting and operating a GNSS module requires more knowledge and involes more options.
Mostly, to configure the hardware and operate correctly. One example would be the selection
of the desired Constellation, and navigation signals. You select this mode of operation by
connecting to a device with `-p,--port` (mandatory).
In active mode, you must select at least one constellation and one signal. For example, `--gps` and `--l1`.

When operating in passive mode, all hardware related options no longer apply.
You select this mode of operation by loading at least one file with `-f,--file` (mandatory).
For example:

```bash
ubx2rinex -f data/UBX/F9T-L2-5min.ubx.gz
```

Will deserialize any data to be encountered. But you can select Constellations specifically,
for example:

```bash
ubx2rinex -f data/UBX/F9T-L2-5min.ubx.gz --gps
```

Will only deserialize GPS data.

In any case, either `-p,--port` or `-f,--file` is required and they are mutually exclusive:
you cannot operate in both modes at the same time.

## Connecting to a U-Blox receiver

To deploy in active mode, you must at least select one Constellation and one signal.  
We propose one flag per constellation, modern UBlox supports tracking of up to 3 constellations.
We offer one flag per signal.

The default opmode is Observation RINEX collection. 
In the following example, we will collect all L1 observations for GPS

```bash
RUST_LOG=trace ubx2rinex -p /dev/ttyUSB1 --gps --l1
[2025-02-23T10:48:22Z INFO  ubx2rinex] Connected to U-Blox
[2025-02-23T10:48:22Z DEBUG ubx2rinex] Software version: EXT CORE 3.01 (111141)
[2025-02-23T10:48:22Z DEBUG ubx2rinex] Firmware version: 00080000
```

Not defining a baud rate value means you are using our 115_200 default value.

In summary, the mandatory flags are:
- `-p,--port` to define your serial port
- at least one constellation activation flags, like `--gps`
- define a specific baud rate if you want

To determine your U-Blox port on linux, for example:

```bash
dmesg | tail -n 20
```

<img src="docs/ports-listing.png" alt="Serial Port listing" width="500" />

Follow through this tutorial to understand all the options we offer, especially:

- the application [default behavior](#default-behavior)
- [U-Blox configuration options](#u-blox-configuration)
- [select your constellation](#constellation)
- [Observation RINEX collection](#obs-rinex-collection)

## :warning: M8 Series usage

:warning: Until further notice :warning:

This application is compatible with M8 series device but does not offer
any option (as of today) to reprogram the Constellation / Signal settings. You will have
to use a third party tool to actually reconfigure your device. 

For example, M8T factory settings are GPS (L1) and GLO (L1).
We would deploy like this:

```bash
ubx2rinex -p /dev/ttyUSB1 --gps --glonass --l1
```

Any other constellation flags has no effect. Selecting other signals has no effect.
Removing L1 signal would create invalid RINEX.

## Deserializing UBX files

You can use `UBX2RINEX` to deserialize your UBX snapshots to OBS, NAV and OBS+NAV RINEX files.  
This mode is selected by loading _at least_ one UBX file into the interface with `-f,--file`.   
We support `gzip` compressed UBX files as well, but their name must be terminated with `.ubx`.

Deploying `UBX2RINEX` in passive mode:

```bash
ubx2rinex -f /tmp/snapshot1.ubx -f /tmp/snapshot2.ubx.gz 
```

Note that constellation and signal selection are no longer required, they no longer apply.
If you happen to use one of those flags, the application will not crash, it will simply do not generate anything.  

Any options related to data collection still applies to the passive mode.

## Application logs

`UBX2RINEX` uses the Rust logger for tracing events in real-time and not disturb the collection process.  
To activate the application logs, simply define the `$RUST_LOG` variable:

```bash
export RUST_LOG=info
```

Several sensitivity options exist:

- info
- error
- debug
- trace

U-Blox configuration
====================

U-Blox receivers are very user friendly yet still require a little knowledge to operate.  
That is especially true to advanced use cases. 

The most basic configuration your need to understand, is how to parametrize the streaming options
of your U-Blox device. `ubx2rinex` allows partial reconfiguration of the U-Blox receiver:

(1) Define the streaming interface(s) and options
(2) Customize the receiver for this application's need

(1): means you can actually use `ubx2rinex` to parametrize how your U-Blox streams.
It is also necessary to activate streaming on at least the USB/UART port that you intend to use.

(2): configuring the receiver, in particular what frames it will transmit, will modify the RINEX content
we are able to collect obviously.

## USB/UART port setup

TODO

## UBX streaming setup

TODO

RINEX Collection
================

`ubx2rinex` is a collecter, in the sense that it gathers a real-time stream from your u-Blox,
and dumps it into supported RINEX formats. It is important to keep in mind that, in order to format
a meaningful (and correct) RINEX header, we can only redact it after completion of a first entire epoch,
every time a new gathering period starts.

File name conventions
=====================

`UBX2RINEX` will generate short (V2) standardized RINEX file names by default,
even though we generate a V3 file format by default.

```bash
ubx2rinex -f data/UBX/F9T-L2-5min.ubx.gz --gps
```

This is because the V3 file format is the most common and normal RINEX format.
Yet, it is impossible to generate a valid V3 file name by default.

To switch to V3 file name by default, simply use `-l,--long`, which means
you prefer longer filenames. But to obtain a valid file name, you should specify
a country code as well:

```bash
ubx2rinex -l --gps -c USA -f data/UBX/F9T-L2-5min.ubx.gz
```

Note that country codes are always 3 letters.

The receiver model also impacts the standardized V2/V3 standardized filename.
For example, here we emphasize that this is a F9T receiver model, and that applies
to each standard:

```bash
ubx2rinex -m F9T --gps -c USA -f data/UBX/F9T-L2-5min.ubx.gz
ubx2rinex -l -m F9T --gps -c USA -f data/UBX/F9T-L2-5min.ubx.gz
```

You can generate a completely custom name and not use the standard generator:

```bash
ubx2rinex -n CUSTOM --gps -f data/UBX/F9T-L2-5min.ubx.gz
```

And still take advantage of session customization, for example:

```bash
ubx2rinex -n CUSTOM -m F9T --gps -f data/UBX/F9T-L2-5min.ubx.gz
```

You can select a destination folder with `--prefix`, which applies to either
customized or standardize names:

```bash
ubx2rinex --prefix /tmp -m F9T --gps -f data/UBX/F9T-L2-5min.ubx.gz
ubx2rinex --prefix /tmp -n CUSTOM -m F9T --gps -f data/UBX/F9T-L2-5min.ubx.gz
```

Signal Collection
=================

The default `ubx2rinex` collection mode is Observation RINEX collection.  

Using this mode, you can use your U-Blox as a real-time signal source (sampler)
which is then collected as [Receiver Independent EXchange](https://github.com/nav-solutions/rinex)
for distribution, post processing and much more. The default RINEX format garantees 17 digits 
of precision on sampled signal and 14 digits on the local clock state.

Observation RINEX collection is the default mode and deploys at all-times, unless you
use the `--no-obs` flag, which will disable this mode: 

```bash
./target/release/ubx2rinex \
        -p /dev/ttyACM0 \
        -b 115200 \
        --gps \
        --l1 \
        --rx-clock \
        -m "M8T u-Blox"

[2025-02-25T20:30:01Z DEBUG ubx2rinex::device] U-Blox Software version: EXT CORE 3.01 (111141)
[2025-02-25T20:30:01Z DEBUG ubx2rinex::device] U-Blox Firmware version: 00080000
[2025-02-25T20:30:01Z INFO  ubx2rinex::device] Enabled constellations: GPS, Glonass, 
[2025-02-25T20:30:01Z INFO  ubx2rinex::device] Supported constellations: GPS, Galileo, BeiDou, Glonass, 
[2025-02-25T20:30:01Z DEBUG ubx2rinex] UBX-NAV-EOE enabled
[2025-02-25T20:30:02Z DEBUG ubx2rinex] UBX-NAV-PVT enabled
[2025-02-25T20:30:02Z DEBUG ubx2rinex] UBX-NAV-CLK enabled
[2025-02-25T20:30:02Z DEBUG ubx2rinex] UBX-NAV-PVT enabled
[2025-02-25T20:30:02Z DEBUG ubx2rinex] Measurement rate is 30 s (Gps)
[2025-02-25T20:30:03Z INFO  ubx2rinex] Observation RINEX mode deployed
[2025-02-25T20:30:03Z INFO  ubx2rinex] 2025-02-25T20:30:18.313672551 GPST - program deployed
[2025-02-25T20:30:12Z TRACE ubx2rinex::collecter] 2025-02-25T20:30:30.006000000 GPST - (G02 RAWX) - pr=2.2654484E7 cp=1.1905012E8 dop=-1.1215144E3 cno=25
[2025-02-25T20:30:12Z TRACE ubx2rinex::collecter] 2025-02-25T20:30:30.006000000 GPST - (G03 RAWX) - pr=2.1846652E7 cp=1.1480491E8 dop=6.1150366E2 cno=37
[2025-02-25T20:30:12Z TRACE ubx2rinex::collecter] 2025-02-25T20:30:30.006000000 GPST - (R19 RAWX) - pr=2.4613278E7 cp=1.3166441E8 dop=-3.8444370E3 cno=23
[2025-02-25T20:30:12Z DEBUG ubx2rinex] NAV PVT: NavPvt { itow: 246630000, year: 2025, month: 2, day: 25, hour: 20, min: 30, sec: 12, valid: 55, time_accuracy: 50, nanosecond: 371019, fix_type: Fix3D, flags: NavPvtFlags(GPS_FIX_OK), flags2: NavPvtFlags2(RESERVED1 | RESERVED3 | CONFIRMED_AVAI | CONFIRMED_DATE | CONFIRMED_TIME), num_satellites: 8, lon: 4.635316899999999, lat: 43.6876077, height_meters: 65.992, height_msl: 18.155, horiz_accuracy: 21079, vert_accuracy: 24241, vel_north: 0.081, vel_east: 0.092, vel_down: -0.129, ground_speed: 0.123, heading: 0.0, speed_accuracy_estimate: 0.20400000000000001, heading_accuracy_estimate: 162.7699, pdop: 204, reserved1: [0, 0, 74, 100, 35, 0], heading_of_vehicle: 0.0, magnetic_declination: 0.0, magnetic_declination_accuracy: 0.0 }
[2025-02-25T20:30:12Z DEBUG ubx2rinex] nav-sat: NavSat { itow: 246630000, version: 1, num_svs: 21, reserved: [0, 0], [...] }
[2025-02-25T20:30:12Z DEBUG ubx2rinex] NAV CLK: NavClock { itow: 246630000, clk_b: 628984, clk_d: 187, t_acc: 50, f_acc: 736 }
[...]
```

## Observation RINEX Timescale

:warning: Observation RINEX express timestamps and clock states in a specific [GNSS Timescale](https://github.com/nav-solutions/gnss),
not UTC. 

`ubx2rinex` is smart, it will adapt the main Timescale to [your Constellation choices](#Constellation).

Receiver clock state collection
===============================

Observation RINEX allows describing the receiver clock state with 14 digits precision.  
This is optional and disabled by default. If you are interested in capturing and distributing your local
clock state, you should turn activate this option with `--rx-clock`.

Sampling period
===============

When collecting signal observations, it is important to define your sampling period. The default sampling period is set to 30s, which is compatible with standard Observation RINEX publications.

You can use any custom value above 50ms. 

In this example, we reduce the sampling period to 1s:

```bash
ubx2rinex -p /dev/ttyACM0 \
          --gps \
          -s "1 s"
```

NAV RINEX Collection
====================

This tool supports NAV RINEX files collection, it is activated with `--nav`.  
This is currently limited to GPS and QZSS constellation and ephemeris messages. We hope to unlock Galileo, BDS and Glonass message soon. Note that this mode, because it is very particular, is not activated by default.
The default mode of operation is Observation RINEX collection.

Note that you can disable OBS RINEX collection with `--no-obs`, this is particularly useful
if you're only interested in dumping NAV RINEX files.

The selected RINEX revision impacts the navigation file content severaly. This is mostly due
to the fact the RINEX format was not particularly well designed for navigation frames, until V3+.
Note that our default revision is _V3_, while we correctly support V4. This is a design choice,
to make the default revision compatible with most post-processing tools, as very few actually
accept RINEX V4 correctly. We distinguish two different cases:

- V2, V3(Default): when operating in default revision, or selecting `--v2` specifically,
the tool redacts a legacy Navigation file, where legacy navigation messages are described.
The ionosphere and orientation models are daily (24h timeframe) and described in the file header.

- V4: when `--v4` is specifically selected, navigation messages are once again updated regularly,
more messages are introduced and supported.
But the ionosphere and orientation models are also updated regularly. This is most suited
for high precision navigation. Once this tool supports more than ephemeris messages, we can
take advantage of this.

RINEX Colection: Snapshot period
================================

The snapshot period defines how often we release a RINEX of each kind.
When the snapshot is being released, the file handled is released and the file is ready to be distributed or post processed.

By default, the snapshot period is set to Daily, which is compatible with standard RINEX publications.  

Several options exist (you can only select one at once):

- `--hourly` for Hourly RINEX publication
- `--quarterly` for one publication every 6 hours
- `--noon` for one publication every 12 hours
- `--custom $dt` for custom publication period. Every valid `Duration` description may apply. For example, these are all valid durations: `--period  

NB: 

- the first signal observation is released everyday at midnight 00:00:00 in the main Timescale
- the last signal observation is released everyday at 23:59:30 in the main Timescale

RINEX Collection: file rotation and descriptors
===============================================

`ubx2rinex` owns the File descriptor with write access (only) until the end of this period.  
That means you can only read the file until the end of the period.
Deleting the file descriptor while the program is actively collecting will the program to panic.

At the end of each period, the file descriptor is released and you can fully process it.   
The file pointer is then incremented, using standard naming conventions.

`ubx2rinex` will provide content "as soon as" it exists (+/- some file descriptor access, that we
try to keep efficient). This means that exploitation of this program is compatible with real-time
watching of the file being produced and each new symbol is published fairly quickly.

Program interruption and release
================================

`ubx2rinex` does not support Ctrl+C interruption cleanly as of today.

Other customizations
====================

- Define your name as `Operator` in RINEX terminology,
with `--operator myself`
- Define your name as `Observer` in RINEX terminology,
with `--observer myself`
- Define your agency (publisher) with `--agency myagency`
- Define the country code (3 letter) of your agency with `--country ABC`

no-std
======

This application required `std` and there is no plan to make it `no-std` compatible in near future.
