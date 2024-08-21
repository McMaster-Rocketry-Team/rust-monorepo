```sh

cargo run -- detect 2> /dev/null

cargo run -- vl /dev/tty.usbmodem1301 set-flight-profile ./test-configs/flight-profile.json 2> /dev/null
cargo run -- vl /dev/tty.usbmodem1301 set-device-config ./test-configs/avionics.json 2> /dev/null

cargo run -- vl /dev/tty.usbmodem1301 set-device-config ./test-configs/gcm.json 2> /dev/null
cargo run -- vl /dev/tty.usbmodem1301 gcm-send-uplink low-power-mode-on 2> /dev/null
cargo run -- vl /dev/tty.usbmodem1301 gcm-send-uplink soft-arm 2> /dev/null
cargo run -- vl /dev/tty.usbmodem1301 gcm-send-uplink manual-trigger-deployment 2> /dev/null
cargo run -- vl /dev/tty.usbmodem1301 gcm-listen 2> /dev/null
```