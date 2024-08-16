#! /bin/bash

cargo run -q -- ozys "$1" real-time $2 2> /dev/null | stdbuf -oL -eL ttyplot -M 1.604 -m 1.614 -t "Voltage" -u "V"