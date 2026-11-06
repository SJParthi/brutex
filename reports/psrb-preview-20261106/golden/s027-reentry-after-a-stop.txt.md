# Golden scenario `s027-reentry-after-a-stop`

PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed.

- format: `psrb-ref-golden/1`; written by crate `psrb_preview_ref`, session token `PSRB-PREVIEW-REF-20261106-1`
- seed: `0x0000000000000000`
- plants: a jump, a stop-out on a dip, a second jump later: the stock re-enters with no cap
- money in paisa; times IST; `x<n>` repeats a line n times (minutes every 60 s, seconds every `every` s)

## Inputs

```text
window 2018-09 2018-09
calendar weekdays=full weekends=closed
membership-probe opened=yes reliance-2018-09-03=yes reliance-2026-09-30=yes syrma-2026-09-30=no
membership-carry-from -
membership-listed S01
stock S01
zerodha-months 2018-07 2018-08 2018-09
gdfl-months 2018-09
gdfl-refused -
withheld -
day 2018-07-02 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-03 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-04 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-05 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-06 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-09 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-10 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-11 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-12 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-13 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-16 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-17 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-18 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-19 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-20 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-23 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-24 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-25 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-26 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-27 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-30 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-07-31 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-01 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-02 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-03 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-06 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-07 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-08 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-09 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-10 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-13 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-14 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-15 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-16 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-17 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-20 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-21 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-22 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-23 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-24 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-27 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-28 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-29 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-30 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-08-31 minutes=375 seconds=0
m 09:15 100000 100000 100000 100000 1000 x375
day 2018-09-03 minutes=375 seconds=375
m 09:15 100000 100000 100000 100000 1000 x375
s 09:15:01 100000 100000 100000 100000 10 x375 every 60
day 2018-09-04 minutes=375 seconds=375
m 09:15 100000 100000 100000 100000 1000 x30
m 09:45 101000 101000 101000 101000 1000 x75
m 11:00 100600 100600 100600 100600 1000 x60
m 12:00 103000 103000 103000 103000 1000 x210
s 09:15:01 100000 100000 100000 100000 10 x30 every 60
s 09:45:01 101000 101000 101000 101000 10 x75 every 60
s 11:00:01 100600 100600 100600 100600 10 x60 every 60
s 12:00:01 103000 103000 103000 103000 10 x210 every 60
day 2018-09-05 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-06 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-07 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-10 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-11 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-12 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-13 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-14 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-17 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-18 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-19 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-20 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-21 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-24 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-25 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-26 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-27 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
day 2018-09-28 minutes=375 seconds=375
m 09:15 103000 103000 103000 103000 1000 x375
s 09:15:01 103000 103000 103000 103000 10 x375 every 60
end-stock
```

## Expected outputs

```text
stock S01 span=2018-09-03..2018-09-28 warmup=2018-07 refused=none fold-errors=- membership-unknown=0
fold S01 k=1 holes=0 outside-session=0
fold S01 k=3 holes=0 outside-session=0
fold S01 k=5 holes=0 outside-session=0
fold S01 k=15 holes=0 outside-session=0
fold S01 k=30 holes=0 outside-session=0
fold S01 k=60 holes=0 outside-session=0
candidates VO 1 count=6
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:01 101000 495 10:30:01 101000 stop 0 0 100700 101000 0 0 32681 dated -32681
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:01 101000 495 10:30:01 101000 stop 0 0 100730 101000 0 0 32681 dated -32681
c 2 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 10:30:01 101000 stop 0 0 100757 101000 0 0 32681 dated -32681
c 3 2018-09-04 12:01:00 S01 LONG filled 12:01:01 103000 485 12:54:01 103000 stop 0 0 102280 103000 0 0 32678 dated -32678
c 4 2018-09-04 12:02:00 S01 LONG filled 12:02:01 103000 485 12:54:01 103000 stop 0 0 102352 103000 0 0 32678 dated -32678
c 5 2018-09-04 12:03:00 S01 LONG filled 12:03:01 103000 485 12:54:01 103000 stop 0 0 102418 103000 0 0 32678 dated -32678
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
f 3 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VO 3 count=6
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 11:00:01 100600 stop 0 0 100700 100979 0 -198000 32672 dated -230672
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:01 101000 495 11:00:01 100600 stop 0 0 100730 100979 0 -198000 32672 dated -230672
c 2 2018-09-04 09:54:00 S01 LONG filled 09:54:01 101000 495 11:00:01 100600 stop 0 0 100757 100979 0 -198000 32672 dated -230672
c 3 2018-09-04 12:03:00 S01 LONG filled 12:03:01 103000 485 14:42:01 103000 stop 0 0 102265 103000 0 0 32678 dated -32678
c 4 2018-09-04 12:06:00 S01 LONG filled 12:06:01 103000 485 14:42:01 103000 stop 0 0 102337 103000 0 0 32678 dated -32678
c 5 2018-09-04 12:09:00 S01 LONG filled 12:09:01 103000 485 14:42:01 103000 stop 0 0 102403 103000 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 3 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VO 5 count=6
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101000 495 11:00:01 100600 stop 0 0 100700 100934 0 -198000 32672 dated -230672
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:01 101000 495 11:00:01 100600 stop 0 0 100730 100934 0 -198000 32672 dated -230672
c 2 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 11:00:01 100600 stop 0 0 100757 100934 0 -198000 32672 dated -230672
c 3 2018-09-04 12:05:00 S01 LONG filled 12:05:01 103000 485 15:08:01 103000 time 0 0 102229 102985 0 0 32678 dated -32678
c 4 2018-09-04 12:10:00 S01 LONG filled 12:10:01 103000 485 15:08:01 103000 time 0 0 102307 102985 0 0 32678 dated -32678
c 5 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 102376 102985 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 3 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VO 15 count=6
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 11:00:01 100600 stop 0 0 100700 100805 0 -198000 32672 dated -230672
c 1 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 11:00:01 100600 stop 0 0 100730 100805 0 -198000 32672 dated -230672
c 2 2018-09-04 10:30:00 S01 LONG filled 10:30:01 101000 495 11:00:01 100600 stop 0 0 100757 100805 0 -198000 32672 dated -230672
c 3 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 102088 102715 0 0 32678 dated -32678
c 4 2018-09-04 12:30:00 S01 LONG filled 12:30:01 103000 485 15:08:01 103000 time 0 0 102178 102715 0 0 32678 dated -32678
c 5 2018-09-04 12:45:00 S01 LONG filled 12:45:01 103000 485 15:08:01 103000 time 0 0 102259 102715 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 3 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VO 30 count=5
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 11:00:01 100600 stop 0 0 100700 100730 0 -198000 32672 dated -230672
c 1 2018-09-04 10:45:00 S01 LONG filled 10:45:01 101000 495 11:00:01 100600 stop 0 0 100730 100730 0 -198000 32672 dated -230672
c 2 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 100786 102403 0 0 32678 dated -32678
c 3 2018-09-04 12:45:00 S01 LONG filled 12:45:01 103000 485 15:08:01 103000 time 0 0 102088 102403 0 0 32678 dated -32678
c 4 2018-09-04 13:15:00 S01 LONG filled 13:15:01 103000 485 15:08:01 103000 time 0 0 102181 102403 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 2 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VO 60 count=4
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 103000 time 0 0 100200 102133 0 990000 32919 dated 957081
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 100729 102133 0 0 32678 dated -32678
c 2 2018-09-04 13:15:00 S01 LONG filled 13:15:01 103000 485 15:08:01 103000 time 0 0 102037 102133 0 0 32678 dated -32678
c 3 2018-09-04 14:15:00 S01 LONG filled 14:15:01 103000 485 15:08:01 103000 time 0 0 102133 102133 0 0 32678 dated -32678
f 0 dated-worst 4000 3282 151 12700 10000 1311 1250 225 0 0 32919 today-worst 4000 3100 101 12700 1500 1296 1250 225 0 0 24172 dated-precise 31344 today-precise 22597
candidates AV2 1 count=2
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:01 101000 495 10:30:01 101000 stop 0 0 100800 101000 0 0 32681 dated -32681
c 1 2018-09-04 12:01:00 S01 LONG filled 12:01:01 103000 485 12:54:01 103000 stop 0 0 102520 103000 0 0 32678 dated -32678
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates AV2 3 count=2
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 11:00:01 100600 stop 0 0 100800 100986 0 -198000 32672 dated -230672
c 1 2018-09-04 12:03:00 S01 LONG filled 12:03:01 103000 485 14:42:01 103000 stop 0 0 102510 103000 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates AV2 5 count=2
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101000 495 11:00:01 100600 stop 0 0 100800 100956 0 -198000 32672 dated -230672
c 1 2018-09-04 12:05:00 S01 LONG filled 12:05:01 103000 485 15:08:01 103000 time 0 0 102486 102990 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates AV2 15 count=2
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 11:00:01 100600 stop 0 0 100800 100870 0 -198000 32672 dated -230672
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 102392 102810 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates AV2 30 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 11:00:01 100600 stop 0 0 100800 100820 0 -198000 32672 dated -230672
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 101124 102602 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates AV2 60 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 103000 time 0 0 100300 102422 0 990000 32919 dated 957081
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 101086 102422 0 0 32678 dated -32678
f 0 dated-worst 4000 3282 151 12700 10000 1311 1250 225 0 0 32919 today-worst 4000 3100 101 12700 1500 1296 1250 225 0 0 24172 dated-precise 31344 today-precise 22597
candidates VS 1 count=2
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:01 101000 495 10:30:01 101000 stop 0 0 100800 101000 0 0 32681 dated -32681
c 1 2018-09-04 12:01:00 S01 LONG filled 12:01:01 103000 485 12:54:01 103000 stop 0 0 102520 103000 0 0 32678 dated -32678
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VS 3 count=2
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 11:00:01 100600 stop 0 0 100800 100986 0 -198000 32672 dated -230672
c 1 2018-09-04 12:03:00 S01 LONG filled 12:03:01 103000 485 14:42:01 103000 stop 0 0 102510 103000 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VS 5 count=2
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101000 495 11:00:01 100600 stop 0 0 100800 100956 0 -198000 32672 dated -230672
c 1 2018-09-04 12:05:00 S01 LONG filled 12:05:01 103000 485 15:08:01 103000 time 0 0 102486 102990 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VS 15 count=2
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 11:00:01 100600 stop 0 0 100800 100870 0 -198000 32672 dated -230672
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 102392 102810 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VS 30 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 11:00:01 100600 stop 0 0 100800 100820 0 -198000 32672 dated -230672
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 101124 102602 0 0 32678 dated -32678
f 0 dated-worst 4000 3243 150 12500 10000 1304 1250 225 0 0 32672 today-worst 4000 3064 100 12500 1500 1290 1250 225 0 0 23929 dated-precise 31176 today-precise 22454
f 1 dated-worst 4000 3248 150 12500 10000 1305 1250 225 0 0 32678 today-worst 4000 3068 100 12500 1500 1290 1250 225 0 0 23933 dated-precise 31192 today-precise 22457
candidates VS 60 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 103000 time 0 0 100300 102422 0 990000 32919 dated 957081
c 1 2018-09-04 12:15:00 S01 LONG filled 12:15:01 103000 485 15:08:01 103000 time 0 0 101086 102422 0 0 32678 dated -32678
f 0 dated-worst 4000 3282 151 12700 10000 1311 1250 225 0 0 32919 today-worst 4000 3100 101 12700 1500 1296 1250 225 0 0 24172 dated-precise 31344 today-precise 22597
row VO 1 LONG verdict=MISS trades=2 net=-65359 net-bp=-4 dd-bp=3 dd=65359 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32680 pf=0 charges=65359
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-65359 trades=2 bp=-4 pass=no gap=50653590000
row VO 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 1 BOTH verdict=MISS trades=2 net=-65359 net-bp=-4 dd-bp=3 dd=65359 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32680 pf=0 charges=65359
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-65359 trades=2 bp=-4 pass=no gap=50653590000
row VO 3 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 5 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 15 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 30 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=5 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=5 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VO 60 LONG verdict=MISS trades=1 net=957081 net-bp=47 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=1 worst=47 median=47 best=47 floor=47 win-rate=1000 avg-win=957081 avg-loss=n/a pf=n/a charges=32919
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=957081 trades=1 bp=47 pass=no gap=40429190000
row VO 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 BOTH verdict=MISS trades=1 net=957081 net-bp=47 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=1 worst=47 median=47 best=47 floor=47 win-rate=1000 avg-win=957081 avg-loss=n/a pf=n/a charges=32919
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=957081 trades=1 bp=47 pass=no gap=40429190000
row AV2 1 LONG verdict=MISS trades=2 net=-65359 net-bp=-4 dd-bp=3 dd=65359 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32680 pf=0 charges=65359
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65359 trades=2 bp=-4 pass=no gap=50653590000
row AV2 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 BOTH verdict=MISS trades=2 net=-65359 net-bp=-4 dd-bp=3 dd=65359 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32680 pf=0 charges=65359
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65359 trades=2 bp=-4 pass=no gap=50653590000
row AV2 3 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 5 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 15 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 30 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row AV2 60 LONG verdict=MISS trades=1 net=957081 net-bp=47 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=1 worst=47 median=47 best=47 floor=47 win-rate=1000 avg-win=957081 avg-loss=n/a pf=n/a charges=32919
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=957081 trades=1 bp=47 pass=no gap=40429190000
row AV2 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 BOTH verdict=MISS trades=1 net=957081 net-bp=47 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=1 worst=47 median=47 best=47 floor=47 win-rate=1000 avg-win=957081 avg-loss=n/a pf=n/a charges=32919
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=957081 trades=1 bp=47 pass=no gap=40429190000
row VS 1 LONG verdict=MISS trades=2 net=-65359 net-bp=-4 dd-bp=3 dd=65359 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32680 pf=0 charges=65359
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65359 trades=2 bp=-4 pass=no gap=50653590000
row VS 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 BOTH verdict=MISS trades=2 net=-65359 net-bp=-4 dd-bp=3 dd=65359 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32680 pf=0 charges=65359
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65359 trades=2 bp=-4 pass=no gap=50653590000
row VS 3 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 5 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 15 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 30 LONG verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 BOTH verdict=MISS trades=2 net=-263350 net-bp=-14 dd-bp=13 dd=263350 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-14 median=-14 best=-14 floor=-14 win-rate=0 avg-win=n/a avg-loss=-131675 pf=0 charges=65350
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-263350 trades=2 bp=-14 pass=no gap=52633500000
row VS 60 LONG verdict=MISS trades=1 net=957081 net-bp=47 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=1 worst=47 median=47 best=47 floor=47 win-rate=1000 avg-win=957081 avg-loss=n/a pf=n/a charges=32919
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=957081 trades=1 bp=47 pass=no gap=40429190000
row VS 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 BOTH verdict=MISS trades=1 net=957081 net-bp=47 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=1 worst=47 median=47 best=47 floor=47 win-rate=1000 avg-win=957081 avg-loss=n/a pf=n/a charges=32919
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=957081 trades=1 bp=47 pass=no gap=40429190000
rank 1 VO 60 LONG
rank 2 VO 60 BOTH
rank 3 AV2 60 LONG
rank 4 AV2 60 BOTH
rank 5 VS 60 LONG
rank 6 VS 60 BOTH
rank 7 VO 1 SHORT
rank 8 VO 3 SHORT
rank 9 VO 5 SHORT
rank 10 VO 15 SHORT
rank 11 VO 30 SHORT
rank 12 VO 60 SHORT
rank 13 AV2 1 SHORT
rank 14 AV2 3 SHORT
rank 15 AV2 5 SHORT
rank 16 AV2 15 SHORT
rank 17 AV2 30 SHORT
rank 18 AV2 60 SHORT
rank 19 VS 1 SHORT
rank 20 VS 3 SHORT
rank 21 VS 5 SHORT
rank 22 VS 15 SHORT
rank 23 VS 30 SHORT
rank 24 VS 60 SHORT
rank 25 VO 1 LONG
rank 26 VO 1 BOTH
rank 27 AV2 1 LONG
rank 28 AV2 1 BOTH
rank 29 VS 1 LONG
rank 30 VS 1 BOTH
rank 31 VO 3 LONG
rank 32 VO 3 BOTH
rank 33 VO 5 LONG
rank 34 VO 5 BOTH
rank 35 VO 15 LONG
rank 36 VO 15 BOTH
rank 37 VO 30 LONG
rank 38 VO 30 BOTH
rank 39 AV2 3 LONG
rank 40 AV2 3 BOTH
rank 41 AV2 5 LONG
rank 42 AV2 5 BOTH
rank 43 AV2 15 LONG
rank 44 AV2 15 BOTH
rank 45 AV2 30 LONG
rank 46 AV2 30 BOTH
rank 47 VS 3 LONG
rank 48 VS 3 BOTH
rank 49 VS 5 LONG
rank 50 VS 5 BOTH
rank 51 VS 15 LONG
rank 52 VS 15 BOTH
rank 53 VS 30 LONG
rank 54 VS 30 BOTH
head 1 LONG AV2/VO trades=1000 net=base:-65359 dd-bp=1000 dd=1000
head 1 LONG VS/VO trades=1000 net=base:-65359 dd-bp=1000 dd=1000
head 1 LONG VS/AV2 trades=1000 net=base:-65359 dd-bp=1000 dd=1000
head 1 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 BOTH AV2/VO trades=1000 net=base:-65359 dd-bp=1000 dd=1000
head 1 BOTH VS/VO trades=1000 net=base:-65359 dd-bp=1000 dd=1000
head 1 BOTH VS/AV2 trades=1000 net=base:-65359 dd-bp=1000 dd=1000
head 3 LONG AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 3 LONG VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 3 LONG VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 3 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 BOTH AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 3 BOTH VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 3 BOTH VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 5 LONG AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 5 LONG VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 5 LONG VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 5 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 BOTH AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 5 BOTH VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 5 BOTH VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 15 LONG AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 15 LONG VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 15 LONG VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 15 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 BOTH AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 15 BOTH VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 15 BOTH VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 30 LONG AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 30 LONG VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 30 LONG VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 30 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 BOTH AV2/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 30 BOTH VS/VO trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 30 BOTH VS/AV2 trades=1000 net=base:-263350 dd-bp=1000 dd=1000
head 60 LONG AV2/VO trades=1000 net=1000 dd-bp=base:0 dd=base:0
head 60 LONG VS/VO trades=1000 net=1000 dd-bp=base:0 dd=base:0
head 60 LONG VS/AV2 trades=1000 net=1000 dd-bp=base:0 dd=base:0
head 60 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 BOTH AV2/VO trades=1000 net=1000 dd-bp=base:0 dd=base:0
head 60 BOTH VS/VO trades=1000 net=1000 dd-bp=base:0 dd=base:0
head 60 BOTH VS/AV2 trades=1000 net=1000 dd-bp=base:0 dd=base:0
nearest-met-floor 47 VO 60 LONG
```
