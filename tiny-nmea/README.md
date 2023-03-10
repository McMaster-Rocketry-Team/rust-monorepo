# Tiny NMEA

A tiny NMEA parser for embedded systems. Works in `#[no_std]` and don't need a memory allocator.

## Supported Sentences

- GSV
- GLL

## Example

```rust
use tiny_nmea::NMEA;
use heapless::String;

let mut nmea = NMEA::new();
nmea.update(&String::from("$GNGLL,4315.68533,N,07955.20234,W,080023.000,A,A*5D\r\n"));
info!("longitude: {}", nmea.longitude.unwrap());
```

## Sample Data

The `nmea.txt` file in this directory contains around 20 minutes of NMEA data from a GPS receiver. At around 13000 line, the GPS receiver obtained a fix.

## TODO

- [ ] Error handling