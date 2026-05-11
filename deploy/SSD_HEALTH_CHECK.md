# SSD Health Check

Reference for verifying the state of NVMe SSDs on a fleet host. Use this when commissioning a new server (to confirm the drives are what was ordered and not pre-loved), and when investigating disk-related production issues.

## Install tools

```bash
sudo apt update && sudo apt install -y nvme-cli smartmontools sysstat
```

- `nvme-cli` provides `nvme`, the NVMe-native command for SMART, identify, and admin operations. Prefer over smartctl when available.
- `smartmontools` provides `smartctl`, a generic SMART tool that also works for SATA SSDs and HDDs.
- `sysstat` provides `iostat`, `mpstat`, `sar` for runtime I/O latency and saturation.

## Identify drives

```bash
lsblk
sudo nvme list
```

`lsblk` shows partitions and where they're mounted. `nvme list` shows controllers, their model strings, firmware versions, and capacities.

The model string is what tells you the drive's class. Examples seen in the fleet:

- `KCD71RUG3T84`, `KCD8XRUG3T84`, `KCD81RUG1T92` — Kioxia CD7 / CD8 datacenter U.3 NVMe (TLC, PLP).
- `MZQL21T9HCJR`, `MZQL23T8HCLS`, `MZQLB3T8HALS` — Samsung PM9A3 / PM983 datacenter U.2 NVMe (TLC, PLP).
- `CT4000P3PSSD8` — Crucial P3 consumer M.2 NVMe (QLC, DRAM-less, **no PLP**).
- `Samsung 990 PRO` — consumer-pro M.2 NVMe (TLC, DRAM, **no PLP**).

PLP (power-loss protection) is the property that lets fsync ack immediately because a capacitor guarantees flush-on-power-loss. Datacenter drives have it. Consumer drives don't, and exhibit 200–400 ms fsync tail latencies under bursty writes. The class of the drive is more important than the brand: if a model isn't on the datacenter list, look it up before deploying production state to it.

## Quick health summary (NVMe)

For each drive:

```bash
sudo nvme smart-log /dev/nvme0n1
```

Loop over all drives:

```bash
for d in /dev/nvme[0-9]n1; do
  echo "--- $d ---"
  sudo nvme smart-log "$d"
done
```

The full smart-log output is verbose. The fields below are the ones that matter.

### Fields and how to read them

| Field                                 | Meaning                                                                                                                                                                                                                 | Healthy     | Concern                              | Red flag                                          |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ------------------------------------ | ------------------------------------------------- |
| `critical_warning`                    | Bitmask of self-reported warnings (spare low, temp, reliability, read-only, backup failure).                                                                                                                            | `0`         | —                                    | non-zero — read the bits                          |
| `temperature`                         | Current composite temperature (°C).                                                                                                                                                                                     | < 60 °C     | 60–70 °C                             | > 80 °C sustained                                 |
| `available_spare`                     | Percentage of overprovisioned spare blocks remaining. New drive = 100 %.                                                                                                                                                | > 50 %      | 10–50 %                              | at or below `available_spare_threshold`           |
| `available_spare_threshold`           | The percentage at which the drive triggers a warning. Usually 10 %.                                                                                                                                                     | —           | —                                    | —                                                 |
| `percentage_used`                     | Drive's self-reported endurance consumption. 0 % = new, 100 % = at rated TBW. Drive can usually continue past 100 % but is past warranty endurance.                                                                     | < 50 %      | 50–80 %                              | > 80 % nearing end of life; > 100 % past warranty |
| `Data Units Written`                  | Total TB written (1 unit = 512,000 bytes). Compare to the rated TBW for the model.                                                                                                                                      | < rated TBW | approaching rated TBW                | > rated TBW                                       |
| `power_on_hours`                      | Total hours the drive has been powered. 5-year warranty ≈ 43,800 h.                                                                                                                                                     | < warranty  | nearing warranty                     | > warranty                                        |
| `power_cycles`                        | Power on/off count. Not normally a concern by itself.                                                                                                                                                                   | —           | very high count on a "new" drive     | —                                                 |
| `unsafe_shutdowns`                    | Count of unclean power-offs. PLP drives shrug these off; data integrity is preserved. High count alone is fine; high count + media_errors is not.                                                                       | any         | many on consumer drive               | many + non-zero `media_errors`                    |
| `media_errors`                        | NAND-level uncorrectable errors. **Should always be zero.**                                                                                                                                                             | `0`         | —                                    | any non-zero — replace                            |
| `num_err_log_entries`                 | Count of entries in the controller error log. Some firmware logs benign events (e.g. smartctl probing unsupported log pages produces "Invalid Field in Command"). Inspect the log content before treating as a problem. | low         | high — inspect with `nvme error-log` | repeated NVM/media-level errors                   |
| `warning_temperature_time`            | Cumulative minutes spent above the warning threshold.                                                                                                                                                                   | 0           | non-zero — check airflow             | growing during normal operation                   |
| `critical_composite_temperature_time` | Cumulative minutes above critical threshold.                                                                                                                                                                            | 0           | —                                    | non-zero                                          |

To inspect the controller error log itself (when `num_err_log_entries` is non-zero):

```bash
sudo nvme error-log /dev/nvme0n1
```

Note the `Status Field` and `Error Count`. A repeating `Invalid Field in Command` is benign (smartctl polling an unsupported log page). NVM-level errors (media, integrity) are not.

## Detecting used / refurbished drives at commissioning

For a server billed as new, expect on every drive:

- `power_on_hours` < a few hundred (factory burn-in only).
- `Data Units Written` < a few hundred GB (initial RAID resync may add ~3–4 TB on the root array, which is normal — but the rest should be near zero).
- `percentage_used` = 0 %.
- All three values roughly symmetric across drives in the same server.

Asymmetry between drives in the same chassis is the giveaway. If `nvme0n1` shows 11 hours and `nvme1n1` shows 25,000 hours, one is a pull from a returned server. Push back on the vendor for replacement before deploying state. The drive will still likely work — datacenter NVMe past 25 k hours typically has years of life left — but it isn't what was paid for, and the wear delta will skew future capacity planning.

## Generic SMART (smartctl) — works on SATA SSDs too

For SATA SSDs, or for fields not reported by `nvme smart-log`:

```bash
sudo smartctl -a /dev/sda           # SATA
sudo smartctl -a /dev/nvme0n1       # NVMe (works but redundant with nvme-cli)
```

Key fields differ on SATA. Look for:

- `Reallocated_Sector_Ct` — should be 0.
- `Wear_Leveling_Count` or `Media_Wearout_Indicator` — vendor-specific endurance gauge.
- `Power_On_Hours`, `Total_LBAs_Written`.
- `UDMA_CRC_Error_Count` — non-zero indicates cable/connector issues.

Run a self-test if the controller supports it:

```bash
sudo smartctl -t short /dev/sda     # ~2 minutes
sudo smartctl -t long /dev/sda      # hours, scans all blocks
sudo smartctl -l selftest /dev/sda  # check results
```

## Latency under load

When a drive looks healthy in SMART but you suspect performance issues (slow fsync tails, the slow-blocks failure mode):

```bash
sudo iostat -xmt 1 30
```

Run for at least 30 seconds during representative workload. Columns to watch:

- `r_await`, `w_await` — read/write completion time in ms. Datacenter NVMe steady-state is typically < 2 ms. Tails > 10 ms warrant investigation; > 100 ms means a real problem.
- `aqu-sz` — average queue depth. Persistently > 1 with high `await` indicates the device can't keep up with offered load.
- `%util` — fraction of time the device had at least one outstanding request. Sustained > 90 % means saturation.

`iostat` averages across the sampling interval and won't always catch a single 400 ms fsync stall buried in a 1 s window. For a more direct fsync benchmark:

```bash
dd if=/dev/zero of=/tmp/fsync-test bs=4k count=1000 oflag=dsync && rm /tmp/fsync-test
```

Each write is forced to disk before the next begins. Healthy datacenter NVMe sustains > 50 MB/s; PLP-less consumer drives often manage only 5–20 MB/s under this pattern, with occasional multi-hundred-ms stalls.

## RAID state (mdadm)

The fleet uses Linux md software RAID (RAID1 on AX162, RAID6 on AX102):

```bash
cat /proc/mdstat
```

Read the bracketed status:

- `[N/N] [UUUU…]` — all `N` members up. Healthy.
- `[N/M] [_U…]` with M < N — degraded; underscores mark missing/failed members.
- `resync = X%` — initial bitmap-aware sync after a fresh array build. Normal; system is fully usable while it runs.
- `recovery = X%` — rebuild onto a replaced drive after a failure. Normal during recovery.

Red flags:

- Degraded state lasting beyond expected rebuild time (a fresh array on empty drives finishes in a few hours; a rebuild after replacement scales with drive size).
- A member shown as `(F)` (faulty) or `(S)` (spare not yet active).
- Reconstructed array members with non-zero `media_errors` on the surviving drives — risk of unrecoverable read error during rebuild.

To inspect a specific array in detail:

```bash
sudo mdadm --detail /dev/md3
```

## Quick red-flag checklist

Push back to the vendor or replace the drive proactively if any of these are true:

- `critical_warning` is non-zero.
- `media_errors` is non-zero.
- `percentage_used` > 80 %, with significant remaining workload time expected.
- `Data Units Written` exceeds the rated TBW for the model.
- `available_spare` is at or below `available_spare_threshold`.
- Sustained `w_await` or `f_await` > 100 ms during normal workload (and SMART otherwise looks fine — usually means a PLP-less drive in a fsync-heavy role).
- An RAID array is degraded with no resilver in progress.
- "New" server delivered with one drive showing tens of thousands of `power_on_hours` and petabyte-scale `Data Units Written`.
