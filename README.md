UBX2RINEX
=========

[![Rust](https://github.com/rtk-rs/ubx2rinex/actions/workflows/rust.yml/badge.svg)](https://github.com/rtk-rs/ubx2rinex/actions/workflows/rust.yml)
[![Rust](https://github.com/rtk-rs/ubx2rinex/actions/workflows/daily.yml/badge.svg)](https://github.com/rtk-rs/ubx2rinex/actions/workflows/daily.yml)
[![crates.io](https://img.shields.io/crates/v/ubx2rinex.svg)](https://crates.io/crates/ubx2rinex)

[![License](https://img.shields.io/badge/license-MPL_2.0-orange?style=for-the-badge&logo=mozilla)](https://github.com/rtk-rs/ubx2rinex/blob/main/LICENSE)

`ubx2rinex` is a small command line utility to deserialize
a U-Blox data stream into standardized RINEX file(s).

:warning: this tool is work in progress.

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

## Licensing

This application is part of the [RTK-rs framework](https://github.com/rtk-rs) which
is delivered under the [Mozilla V2 Public](https://www.mozilla.org/en-US/MPL/2.0) license.
