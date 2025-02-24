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

The most basic deployment consists in connecting to your U-blox to a serial port, 
defining the UBX Uart port on your device (assuming your USB/UART is connect to the correct interface),
activating at least one constellation (always required), 

```bash
RUST_LOG=trace ubx2rinex -p /dev/ttyUSB1 --gps
./target/release/ubx2rinex -p /dev/ttyACM0 --gps
[2025-02-23T10:48:22Z INFO  ubx2rinex] Connected to U-Blox
[2025-02-23T10:48:22Z DEBUG ubx2rinex] Software version: EXT CORE 3.01 (111141)
[2025-02-23T10:48:22Z DEBUG ubx2rinex] Firmware version: 00080000
```

Not defining any collection option nor selecting a mode of operation, will deploy the default
behavior, which is Observation RINEX collection, with default options.

Not defining a baud rate value means you are using our 115_200 default value.

In summary, the mandatory flags are:
- `-p,--port` to define your serial port
- at least one constellation activation flags, like `--gps`
- define a specific baud rate if you want

To determine your U-Blox port on linux, for example:

```bash
dmesg | tail -n 20
```

<img src="docs/ports-listing.png" alt="Serial Port listing" width="300" />

Follow through this tutorial to understand all the options we offer, especially:

- the application [default behavior](#default-behavior)
- [U-Blox configuration options](#u-blox-configuration)
- [select your constellation](#constellation)
- [Observation RINEX collection](#obs-rinex-collection)

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

RINEX Collection
================

`ubx2rinex` is a collecter, in the sense that it gathers a real-time stream from your u-Blox,
and dumps it into supported RINEX formats. It is important to keep in mind that, in order to format
a meaningful (and correct) RINEX header, we can only redact it after completion of a first entire epoch,
every time a new gathering period starts.

Signal Collection
=================

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

Snapshot period
===============

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

Snapshot period interruption
============================

`ubx2rinex` does not support graceful interruption. If you abort the ongoing period by killing this program, you may wind-up with an incomplete epoch at the very end.
