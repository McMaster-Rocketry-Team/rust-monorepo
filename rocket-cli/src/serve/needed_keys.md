
# ALL KEYS
## General
- rssi
- snr

## Telemetry
### flight
- flight state
- alt agl
- max alt agl
- vertical velocity
- tilt

### airbrakes (are these still necessary?)
- predicted apogee agl
- target apogee agl
- calibrated?
- enabled?
- commanded extension
- actual extension
- servo temp

### GPS
- num satellites
- unix clock ready
- lat
- lon

### VL (Void Lake?)
- vl battery V
- air temperature
- main continuity
- drogue continuity

### AMP
- online
- rebooted last 5s
- shared battery V
- out 1 status
- out 1 overwritten
- out 2 status
- out 2 overwritten
- out 3 status
- out 3 overwritten

### Icarus
- online
- rebooted last 5s

### OZYS
- online
- rebooted last 5s

### Payload (leave for now until payload communicates what they want us to monitor this year)

## LowPowerTelemetry

### GPS
- fixed? why is this not present in normal telemetry packet?
- num satellites
- lat
- lon

### VL
- vl battery
- air temperature

### AMP
- online
- shared battery V

### Payload (see above)

## LandedTelemetry

### GPS
- num satellites
- lat
- lon

### VL
- vl battery

### AMP
- online
- rebooted last 5s
- shared battery V
##### NOTE - these display differently here compared to normal Telemetry packet - why?
- out 1 status
- out 1 overwrote
- out 2 status
- out 2 overwrote
- out 3 status
- out 3 overwrote

## SelfTestResult
### VL
- imu ok
- baro ok
- mag ok
- gps ok
- sd ok
- can bus ok
- main continuity
- drogue continuity

### AMP
- status (seems to have a couple different subkeys)
- out 1 ok
- out 2 ok
- out 3 ok

### Icarus
- status (seems to have a couple different subkeys)

### OZYS
- status (seems to have a couple different subkeys)
- disk

### Payload (see above)










### BELOW IS PROCESSING OF ABOVE DATA

# Deduplicated Keys
## General
- rssi
- snr

## Telemetry
### flight
- flight stage
- alt agl
- max alt agl
- vertical velocity
- tilt
- air temp

### airbrakes (are these still necessary?)
- predicted apogee agl
- target apogee agl
- calibrated?
- enabled?
- commanded extension
- actual extension
- servo temp

### GPS
- fixed? why is this not present in normal telemetry packet?
- num satellites
- unix clock ready
- lat
- lon

### VL (Void Lake?)
- vl battery V
- air temperature
- main continuity
- drogue continuity
- imu ok
- baro ok
- mag ok
- gps ok
- sd ok
- can bus ok

### AMP
- online
- rebooted last 5s
- shared battery V
##### NOTE - these display differently in LandedTelemetry compared to normal Telemetry packet - why?
- out 1 status
- out 1 overwritten
- out 2 status
- out 2 overwritten
- out 3 status
- out 3 overwritten
- status (seems to have a couple different subkeys - see selftestresult)
- out 1 ok
- out 2 ok
- out 3 ok

### Icarus
- online
- rebooted last 5s
- status (seems to have a couple different subkeys - see selftestresult)

### OZYS
- online
- rebooted last 5s
- status (seems to have a couple different subkeys - see selftestresult)
- disk

### Payload (leave for now until payload communicates what they want us to monitor this year)



## Rocket Telemetry / Ground Station Downlink

### Rocket Telemetry / Ground Station Downlink / Link Quality ALL OK

| Key                          | Name                   | Type    | Unit | Range    | Enum       |
| ---------------------------- | ---------------------- | ------- | ---- | -------- | ---------- |
| `gsd_rssi`                   | RSSI                   | integer | dBm  | -140 … 0 | —          |
| `gsd_snr`                    | SNR                    | integer | dB   | -20 … 20 | —          |
| `gsd_seconds_since_received` | Time Since Last Packet | integer | s    | 0 … 120  | —          |
| `gsd_packet_type`            | Packet Type            | enum    | —    | —        | PacketType |

### Rocket Telemetry / Ground Station Downlink / GPS & Position

| Key                         | Name             | Type    | Unit  | Range      | Enum |
| --------------------------- | ---------------- | ------- | ----- | ---------- | ---- |
| `gsd_num_of_fix_satellites` | Satellites (fix) | integer | count | 0 … 31     | —    |
| `gsd_unix_clock_ready`      | Unix Clock Ready | boolean | —     | —          | Bool |
| `gsd_lat`                   | Latitude         | float   | deg   | -90 … 90   | —    |
| `gsd_lon`                   | Longitude        | float   | deg   | -180 … 180 | —    |
fixed?

### Rocket Telemetry / Ground Station Downlink / Flight Dynamics

| Key                    | Name             | Type  | Unit | Range       | Enum        |
| ---------------------- | ---------------- | ----- | ---- | ----------- | ----------- |
| `gsd_flight_stage`     | Flight Stage     | enum  | —    | —           | FlightStage |
| `gsd_altitude_agl`     | Altitude AGL     | float | m    | -100 … 7000 | —           |
| `gsd_max_altitude_agl` | Max Altitude AGL | float | m    | -100 … 7000 | —           |
| `gsd_air_speed`        | Air Speed        | float | m/s  | 0 … 400     | —           |
| `gsd_max_air_speed`    | Max Air Speed    | float | m/s  | 0 … 400     | —           |
max air speed needed?
| `gsd_tilt_deg`         | Tilt             | float | deg  | -90 … 90    | —           |
| `gsd_air_temperature`  | Air Temperature  | float | °C   | -10 … 85    | —           |

### Rocket Telemetry / Ground Station Downlink / Vehicle Power & Pyro ALL OK

| Key                          | Name                   | Type    | Unit | Range     | Enum |
| ---------------------------- | ---------------------- | ------- | ---- | --------- | ---- |
| `gsd_vl_battery_v`           | VL Battery Voltage     | float   | V    | 2.5 … 8.5 | —    |
| `gsd_shared_battery_v`       | Shared Battery Voltage | float   | V    | 2.5 … 8.5 | —    |
| `gsd_pyro_main_continuity`   | Main Pyro Continuity   | boolean | —    | —         | Bool |
| `gsd_pyro_drogue_continuity` | Drogue Pyro Continuity | boolean | —    | —         | Bool |

### Rocket Telemetry / Ground Station Downlink / AMP (Power Distribution)

| Key                           | Name                   | Type    | Unit | Range | Enum              |
| ----------------------------- | ---------------------- | ------- | ---- | ----- | ----------------- |
| `gsd_amp_online`              | AMP Online             | boolean | —    | —     | Bool              |
| `gsd_amp_rebooted_in_last_5s` | AMP Rebooted (<5 s)    | boolean | —    | —     | Bool              |
| `gsd_amp_out1`                | AMP Output 1 Status    | enum    | —    | —     | PowerOutputStatus |
| `gsd_amp_out1_overwrote`      | AMP Output 1 Overwrote | boolean | —    | —     | Bool              |
| `gsd_amp_out2`                | AMP Output 2 Status    | enum    | —    | —     | PowerOutputStatus |
| `gsd_amp_out2_overwrote`      | AMP Output 2 Overwrote | boolean | —    | —     | Bool              |
| `gsd_amp_out3`                | AMP Output 3 Status    | enum    | —    | —     | PowerOutputStatus |
| `gsd_amp_out3_overwrote`      | AMP Output 3 Overwrote | boolean | —    | —     | Bool              |
remove these last two?
| `gsd_amp_out4`                | AMP Output 4 Status    | enum    | —    | —     | PowerOutputStatus |
| `gsd_amp_out4_overwrote`      | AMP Output 4 Overwrote | boolean | —    | —     | Bool              |

### Rocket Telemetry / Ground Station Downlink / Bulkheads

Are the photoresistors being used this year? They would also need to be added to packet sending + receiving

#### Rocket Telemetry / Ground Station Downlink / Bulkheads / Main Bulkhead

| Key                                     | Name                          | Type    | Unit | Range      | Enum |
| --------------------------------------- | ----------------------------- | ------- | ---- | ---------- | ---- |
| `gsd_main_bulkhead_online`              | Main Bulkhead Online          | boolean | —    | —          | Bool |
| `gsd_main_bulkhead_rebooted_in_last_5s` | Main Bulkhead Rebooted (<5 s) | boolean | —    | —          | Bool |
| `gsd_main_bulkhead_brightness`          | Main Bulkhead Brightness      | float   | lux  | 0 … 100000 | —    |

#### Rocket Telemetry / Ground Station Downlink / Bulkheads / Drogue Bulkhead

| Key                                       | Name                            | Type    | Unit | Range      | Enum |
| ----------------------------------------- | ------------------------------- | ------- | ---- | ---------- | ---- |
| `gsd_drogue_bulkhead_online`              | Drogue Bulkhead Online          | boolean | —    | —          | Bool |
| `gsd_drogue_bulkhead_rebooted_in_last_5s` | Drogue Bulkhead Rebooted (<5 s) | boolean | —    | —          | Bool |
| `gsd_drogue_bulkhead_brightness`          | Drogue Bulkhead Brightness      | float   | lux  | 0 … 100000 | —    |

### Rocket Telemetry / Ground Station Downlink / ICARUS — Air Brakes
Air brakes not being used this year, keys stay in frontend but won't be used

| Key                                             | Name                           | Type    | Unit | Range    | Enum |
| ----------------------------------------------- | ------------------------------ | ------- | ---- | -------- | ---- |
| `gsd_icarus_online`                             | ICARUS Online                  | boolean | —    | —        | Bool |
| `gsd_icarus_rebooted_in_last_5s`                | ICARUS Rebooted (<5 s)         | boolean | —    | —        | Bool |
| `gsd_air_brakes_commanded_extension_percentage` | Air Brakes Commanded Extension | integer | %    | 0 … 100  | —    |
| `gsd_air_brakes_actual_extension_percentage`    | Air Brakes Actual Extension    | integer | %    | 0 … 100  | —    |
| `gsd_air_brakes_servo_temp`                     | Air Brakes Servo Temp          | float   | °C   | -10 … 85 | —    |

### Rocket Telemetry / Ground Station Downlink / OZYS 1

| Key                             | Name                   | Type    | Unit | Range | Enum |
| ------------------------------- | ---------------------- | ------- | ---- | ----- | ---- |
| `gsd_ozys1_online`              | OZYS 1 Online          | boolean | —    | —     | Bool |
| `gsd_ozys1_rebooted_in_last_5s` | OZYS 1 Rebooted (<5 s) | boolean | —    | —     | Bool |

### Rocket Telemetry / Ground Station Downlink / OZYS 2

| Key                             | Name                   | Type    | Unit | Range | Enum |
| ------------------------------- | ---------------------- | ------- | ---- | ----- | ---- |
| `gsd_ozys2_online`              | OZYS 2 Online          | boolean | —    | —     | Bool |
| `gsd_ozys2_rebooted_in_last_5s` | OZYS 2 Rebooted (<5 s) | boolean | —    | —     | Bool |


TO ADD
Icarus and OZYS Status
OZYS Disk

EVERYTHING BELOW HERE IS TO BE UPDATED WHEN PAYLOAD TELL US WHAT THEY WANT

### Rocket Telemetry / Ground Station Downlink / Payload EPS

#### Rocket Telemetry / Ground Station Downlink / Payload EPS / EPS 1

| Key                             | Name                       | Type    | Unit | Range    | Enum              |
| ------------------------------- | -------------------------- | ------- | ---- | -------- | ----------------- |
| `gsd_eps1_online`               | EPS 1 Online               | boolean | —    | —        | Bool              |
| `gsd_eps1_rebooted_in_last_5s`  | EPS 1 Rebooted (<5 s)      | boolean | —    | —        | Bool              |
| `gsd_eps1_battery1_v`           | EPS 1 Battery 1 Voltage    | float   | V    | 2 … 4.5  | —                 |
| `gsd_eps1_battery1_temperature` | EPS 1 Battery 1 Temp       | float   | °C   | 10 … 85  | —                 |
| `gsd_eps1_battery2_v`           | EPS 1 Battery 2 Voltage    | float   | V    | 2 … 4.5  | —                 |
| `gsd_eps1_battery2_temperature` | EPS 1 Battery 2 Temp       | float   | °C   | 10 … 85  | —                 |
| `gsd_eps1_output_3v3_current`   | EPS 1 3V3 Output Current   | integer | mA   | 0 … 2000 | —                 |
| `gsd_eps1_output_3v3_status`    | EPS 1 3V3 Output Status    | enum    | —    | —        | PowerOutputStatus |
| `gsd_eps1_output_3v3_overwrote` | EPS 1 3V3 Output Overwrote | boolean | —    | —        | Bool              |
| `gsd_eps1_output_5v_current`    | EPS 1 5V Output Current    | integer | mA   | 0 … 2000 | —                 |
| `gsd_eps1_output_5v_status`     | EPS 1 5V Output Status     | enum    | —    | —        | PowerOutputStatus |
| `gsd_eps1_output_5v_overwrote`  | EPS 1 5V Output Overwrote  | boolean | —    | —        | Bool              |
| `gsd_eps1_output_9v_current`    | EPS 1 9V Output Current    | integer | mA   | 0 … 2000 | —                 |
| `gsd_eps1_output_9v_status`     | EPS 1 9V Output Status     | enum    | —    | —        | PowerOutputStatus |
| `gsd_eps1_output_9v_overwrote`  | EPS 1 9V Output Overwrote  | boolean | —    | —        | Bool              |

#### Rocket Telemetry / Ground Station Downlink / Payload EPS / EPS 2

| Key                             | Name                       | Type    | Unit | Range    | Enum              |
| ------------------------------- | -------------------------- | ------- | ---- | -------- | ----------------- |
| `gsd_eps2_online`               | EPS 2 Online               | boolean | —    | —        | Bool              |
| `gsd_eps2_rebooted_in_last_5s`  | EPS 2 Rebooted (<5 s)      | boolean | —    | —        | Bool              |
| `gsd_eps2_battery1_v`           | EPS 2 Battery 1 Voltage    | float   | V    | 2 … 4.5  | —                 |
| `gsd_eps2_battery1_temperature` | EPS 2 Battery 1 Temp       | float   | °C   | 10 … 85  | —                 |
| `gsd_eps2_battery2_v`           | EPS 2 Battery 2 Voltage    | float   | V    | 2 … 4.5  | —                 |
| `gsd_eps2_battery2_temperature` | EPS 2 Battery 2 Temp       | float   | °C   | 10 … 85  | —                 |
| `gsd_eps2_output_3v3_current`   | EPS 2 3V3 Output Current   | integer | mA   | 0 … 2000 | —                 |
| `gsd_eps2_output_3v3_status`    | EPS 2 3V3 Output Status    | enum    | —    | —        | PowerOutputStatus |
| `gsd_eps2_output_3v3_overwrote` | EPS 2 3V3 Output Overwrote | boolean | —    | —        | Bool              |
| `gsd_eps2_output_5v_current`    | EPS 2 5V Output Current    | integer | mA   | 0 … 2000 | —                 |
| `gsd_eps2_output_5v_status`     | EPS 2 5V Output Status     | enum    | —    | —        | PowerOutputStatus |
| `gsd_eps2_output_5v_overwrote`  | EPS 2 5V Output Overwrote  | boolean | —    | —        | Bool              |
| `gsd_eps2_output_9v_current`    | EPS 2 9V Output Current    | integer | mA   | 0 … 2000 | —                 |
| `gsd_eps2_output_9v_status`     | EPS 2 9V Output Status     | enum    | —    | —        | PowerOutputStatus |
| `gsd_eps2_output_9v_overwrote`  | EPS 2 9V Output Overwrote  | boolean | —    | —        | Bool              |

### Rocket Telemetry / Ground Station Downlink / Avionics Status

| Key                                              | Name                                   | Type    | Unit | Range | Enum |
| ------------------------------------------------ | -------------------------------------- | ------- | ---- | ----- | ---- |
| `gsd_payload_activation_pcb_online`              | Payload Activation PCB Online          | boolean | —    | —     | Bool |
| `gsd_payload_activation_pcb_rebooted_in_last_5s` | Payload Activation PCB Rebooted (<5 s) | boolean | —    | —     | Bool |
| `gsd_rocket_wifi_online`                         | Rocket WiFi Online                     | boolean | —    | —     | Bool |
| `gsd_rocket_wifi_rebooted_in_last_5s`            | Rocket WiFi Rebooted (<5 s)            | boolean | —    | —     | Bool |

## Enumerations

Enum and boolean signals carry one of these numeric `Value`s; OpenMCT renders the
matching `String`. Send the **Value**, not the string.

### Bool

| Value | String |
| ----- | ------ |
| 0     | FALSE  |
| 1     | TRUE   |

### FlightStage

| Value | String         |
| ----- | -------------- |
| 0     | LowPower       |
| 1     | SelfTest       |
| 2     | Armed          |
| 3     | PoweredAscent  |
| 4     | Coasting       |
| 5     | DrogueDeployed |
| 6     | MainDeployed   |
| 7     | Landed         |

### PacketType

| Value | String            |
| ----- | ----------------- |
| 0     | GPSBeacon         |
| 1     | Telemetry         |
| 2     | LowPowerTelemetry |
| 3     | LandedTelemetry   |
| 4     | SelfTestResult    |
| 5     | Ack               |

### PowerOutputOverwrite

| Value | String       |
| ----- | ------------ |
| 0     | No Overwrite |
| 1     | Enable       |
| 2     | Disable      |

### PowerOutputStatus

| Value | String    |
| ----- | --------- |
| 0     | Disabled  |
| 1     | PowerGood |
| 2     | PowerBad  |

## Appendix — demo-only keys

These belong to the `FakeDataGenerator` that drives the Avionics Dashboard battery
overlay. They are **not** part of the rocket telemetry catalog and a real backend
does not need to emit them.

| Key                     | Used by                               |
| ----------------------- | ------------------------------------- |
| `vl_battery_v_gps`      | VL Battery Voltage overlay (GPS fix)  |
| `vl_battery_v_received` | VL Battery Voltage overlay (received) |
