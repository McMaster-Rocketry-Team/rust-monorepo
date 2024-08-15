#! /bin/bash

cargo run -- ozys /dev/ttyACM0 real-time | stdbuf -o0 awk '{print $1}' | ttyplot -M 1.5 -m 1.6 -t "Voltage" -u "V"
