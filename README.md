UBX2RINEX
=========

[![Rust](https://github.com/rtk-rs/ubx2rinex/actions/workflows/rust.yml/badge.svg)](https://github.com/rtk-rs/ubx2rinex/actions/workflows/rust.yml)
[![Rust](https://github.com/rtk-rs/ubx2rinex/actions/workflows/daily.yml/badge.svg)](https://github.com/rtk-rs/ubx2rinex/actions/workflows/daily.yml)
[![crates.io](https://img.shields.io/crates/v/ubx2rinex.svg)](https://crates.io/crates/ubx2rinex)

[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/rtk-rs/ubx2rinex/blob/main/LICENSE)

`ubx2rinex` is a small command line utility to deserialize
a U-Blox data stream into standardized RINEX file(s).

:warning: this tool is work in progress.

## Licensing

This application is part of the [RTK-rs framework](https://github.com/rtk-rs) which
is delivered under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.

## Install from Cargo

You can directly install the tool from Cargo with internet access:

```bash
cargo install ubx2rinex
```

## Build from sources

Download the version you are interested in:

```bash
git clone https://github.com/rtk-rs/ubx2rinex
```

And build it using cargo:

```bash
cargo build --all-features -r
```

## Getting started

The most basic deployment consists in connecting to your U-blox and using the default collection
mode (OBS RINEX Only)

```bash
ubx2rinex -p /dev/ttyUSB1
```

## Application logs

`ubx2rinex` uses the Rust logger for tracing events in real-time and not disturb the collection process.  
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

OBS RINEX Collection
====================

The default `ubx2rinex` collection mode is Observation RINEX collection.  
Using this mode, you can use your U-Blox as a real-time signal source (sampler)
which is then collected as [Receiver Independent EXchange](https://github.com/rtk-rs/rinex)
for distribution, post processing and much more. The default RINEX format garantees 17 digits 
of precision on sampled signal and 14 digits on the local clock state.

Observation RINEX collection is the default mode and deploys at all-times, unless you
use the `--no-obs` flag, which will turn this collection option. 

## Observation RINEX Timescale

:warning: Observation RINEX express timestamps and clock states in a specific [GNSS Timescale](https://github.com/rtk-rs/gnss),
not UTC. 

`ubx2rinex` is smart, it will adapt the main Timescale to [your Constellation choices](#Constellation).
NB: we use GPS only by default (for simplicity).

Since Observation RINEX collection is the default mode of this tool, the default timescale is therefore GPS time.

Snapshot and collecting options
===============================

Since we're collecting a real-time stream, it is important to define how we collect the data.  

`ubx2rinex` default collecting option is Standardized RINEX. Standard RINEX files
are published on a daily basis, they last 24 hours, use a 30s sampling interval. That means

- the first signal observation is released everyday at midnight 00:00:00 in the main Timescale
- the last signal observation is released everyday at 23:59:30 in the main Timescale

