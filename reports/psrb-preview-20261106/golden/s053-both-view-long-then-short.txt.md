# Golden scenario `s053-both-view-long-then-short`

PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed.

- format: `psrb-ref-golden/1`; written by crate `psrb_preview_ref`, session token `PSRB-PREVIEW-REF-20261106-1`
- seed: `0x0000000000000000`
- plants: a long open, then a short signal on the same stock: BOTH counts it while open, SHORT takes it
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
m 09:45 101000 101000 101000 101000 1000
m 09:46 101001 101001 101001 101001 1000
m 09:47 101002 101002 101002 101002 1000
m 09:48 101003 101003 101003 101003 1000
m 09:49 101004 101004 101004 101004 1000
m 09:50 101005 101005 101005 101005 1000
m 09:51 101006 101006 101006 101006 1000
m 09:52 101007 101007 101007 101007 1000
m 09:53 101008 101008 101008 101008 1000
m 09:54 101009 101009 101009 101009 1000
m 09:55 101010 101010 101010 101010 1000
m 09:56 101011 101011 101011 101011 1000
m 09:57 101012 101012 101012 101012 1000
m 09:58 101013 101013 101013 101013 1000
m 09:59 101014 101014 101014 101014 1000
m 10:00 98000 98000 98000 98000 1000 x330
s 09:15:01 100000 100000 100000 100000 10 x30 every 60
s 09:45:01 101000 101000 101000 101000 10
s 09:46:01 101001 101001 101001 101001 10
s 09:47:01 101002 101002 101002 101002 10
s 09:48:01 101003 101003 101003 101003 10
s 09:49:01 101004 101004 101004 101004 10
s 09:50:01 101005 101005 101005 101005 10
s 09:51:01 101006 101006 101006 101006 10
s 09:52:01 101007 101007 101007 101007 10
s 09:53:01 101008 101008 101008 101008 10
s 09:54:01 101009 101009 101009 101009 10
s 09:55:01 101010 101010 101010 101010 10
s 09:56:01 101011 101011 101011 101011 10
s 09:57:01 101012 101012 101012 101012 10
s 09:58:01 101013 101013 101013 101013 10
s 09:59:01 101014 101014 101014 101014 10
s 10:00:01 98000 98000 98000 98000 10 x330 every 60
day 2018-09-05 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-06 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-07 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-10 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-11 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-12 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-13 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-14 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-17 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-18 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-19 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-20 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-21 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-24 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-25 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-26 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-27 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
day 2018-09-28 minutes=375 seconds=375
m 09:15 98000 98000 98000 98000 1000 x375
s 09:15:01 98000 98000 98000 98000 10 x375 every 60
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
candidates VO 1 count=7
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:01 101001 495 10:00:01 98000 stop 0 0 100700 100945 0 -1485495 32321 dated -1517816
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:01 101002 495 10:00:01 98000 stop 0 0 100731 100945 0 -1485990 32321 dated -1518311
c 2 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101003 495 10:00:01 98000 stop 0 0 100759 100945 0 -1486485 32321 dated -1518806
c 3 2018-09-04 09:49:00 S01 LONG filled 09:49:01 101004 495 10:00:01 98000 stop 0 0 100784 100945 0 -1486980 32321 dated -1519301
c 4 2018-09-04 10:01:00 S01 SHORT filled 10:01:01 98000 510 10:56:01 98000 stop 0 0 98966 98000 0 0 32679 dated -32679
c 5 2018-09-04 10:02:00 S01 SHORT filled 10:02:01 98000 510 10:56:01 98000 stop 0 0 98870 98000 0 0 32679 dated -32679
c 6 2018-09-04 10:03:00 S01 SHORT filled 10:03:01 98000 510 10:56:01 98000 stop 0 0 98783 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 4 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VO 3 count=7
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101003 495 10:00:01 98000 stop 0 0 100701 100815 0 -1486485 32321 dated -1518806
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:01 101006 495 10:00:01 98000 stop 0 0 100734 100815 0 -1487970 32321 dated -1520291
c 2 2018-09-04 09:54:00 S01 LONG filled 09:54:01 101009 495 10:00:01 98000 stop 0 0 100764 100815 0 -1489455 32321 dated -1521776
c 3 2018-09-04 09:57:00 S01 LONG filled 09:57:01 101012 494 10:00:01 98000 stop 0 0 100791 100815 0 -1487928 32245 dated -1520173
c 4 2018-09-04 10:03:00 S01 SHORT filled 10:03:01 98000 510 12:51:01 98000 stop 0 0 99083 98000 0 0 32679 dated -32679
c 5 2018-09-04 10:06:00 S01 SHORT filled 10:06:01 98000 510 12:51:01 98000 stop 0 0 98975 98000 0 0 32679 dated -32679
c 6 2018-09-04 10:09:00 S01 SHORT filled 10:09:01 98000 510 12:51:01 98000 stop 0 0 98876 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 4 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VO 5 count=6
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101005 495 10:00:01 98000 stop 0 0 100702 100766 0 -1487475 32321 dated -1519796
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:01 101010 495 10:00:01 98000 stop 0 0 100737 100766 0 -1489950 32321 dated -1522271
c 2 2018-09-04 10:00:00 S01 LONG filled 10:00:01 98000 510 10:01:01 98000 stop 0 0 100766 100766 0 0 32679 dated -32679
c 3 2018-09-04 10:05:00 S01 SHORT filled 10:05:01 98000 510 14:50:01 98000 stop 0 0 99125 98000 0 0 32679 dated -32679
c 4 2018-09-04 10:10:00 S01 SHORT filled 10:10:01 98000 510 14:50:01 98000 stop 0 0 99011 98000 0 0 32679 dated -32679
c 5 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 14:50:01 98000 stop 0 0 98912 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 3 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VO 15 count=4
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 98000 510 10:01:01 98000 stop 0 0 100704 100704 0 0 32679 dated -32679
c 1 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 99176 98159 0 0 32679 dated -32679
c 2 2018-09-04 10:30:00 S01 SHORT filled 10:30:01 98000 510 15:08:01 98000 time 0 0 99059 98159 0 0 32679 dated -32679
c 3 2018-09-04 10:45:00 S01 SHORT filled 10:45:01 98000 510 15:08:01 98000 time 0 0 98954 98159 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
f 1 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VO 30 count=3
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 100000 98348 0 0 32679 dated -32679
c 1 2018-09-04 10:45:00 S01 SHORT filled 10:45:01 98000 510 15:08:01 98000 time 0 0 98813 98348 0 0 32679 dated -32679
c 2 2018-09-04 11:15:00 S01 SHORT filled 11:15:01 98000 510 15:08:01 98000 time 0 0 98732 98348 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VO 60 count=3
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 100000 98591 0 0 32679 dated -32679
c 1 2018-09-04 11:15:00 S01 SHORT filled 11:15:01 98000 510 15:08:01 98000 time 0 0 98813 98591 0 0 32679 dated -32679
c 2 2018-09-04 12:15:00 S01 SHORT filled 12:15:01 98000 510 15:08:01 98000 time 0 0 98732 98591 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates AV2 1 count=3
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:01 101001 495 10:00:01 98000 stop 0 0 100800 100968 0 -1485495 32321 dated -1517816
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:01 101002 495 10:00:01 98000 stop 0 0 100821 100968 0 -1485990 32321 dated -1518311
c 2 2018-09-04 10:01:00 S01 SHORT filled 10:01:01 98000 510 10:56:01 98000 stop 0 0 98644 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 2 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates AV2 3 count=3
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101003 495 10:00:01 98000 stop 0 0 100801 100881 0 -1486485 32321 dated -1518806
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:01 101006 495 10:00:01 98000 stop 0 0 100824 100881 0 -1487970 32321 dated -1520291
c 2 2018-09-04 10:03:00 S01 SHORT filled 10:03:01 98000 510 12:51:01 98000 stop 0 0 98722 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 2 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates AV2 5 count=3
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101005 495 10:00:01 98000 stop 0 0 100802 100848 0 -1487475 32321 dated -1519796
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:01 101010 495 10:00:01 98000 stop 0 0 100827 100848 0 -1489950 32321 dated -1522271
c 2 2018-09-04 10:05:00 S01 SHORT filled 10:05:01 98000 510 14:50:01 98000 stop 0 0 98750 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 2 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates AV2 15 count=2
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 98000 510 10:01:01 98000 stop 0 0 100805 100805 0 0 32679 dated -32679
c 1 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 98784 98106 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
f 1 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates AV2 30 count=1
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 100000 98232 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates AV2 60 count=1
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 100000 98394 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VS 1 count=3
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:01 101001 495 10:00:01 98000 stop 0 0 100800 100968 0 -1485495 32321 dated -1517816
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:01 101002 495 10:00:01 98000 stop 0 0 100821 100968 0 -1485990 32321 dated -1518311
c 2 2018-09-04 10:01:00 S01 SHORT filled 10:01:01 98000 510 10:56:01 98000 stop 0 0 98644 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 2 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VS 3 count=3
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101003 495 10:00:01 98000 stop 0 0 100801 100881 0 -1486485 32321 dated -1518806
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:01 101006 495 10:00:01 98000 stop 0 0 100824 100881 0 -1487970 32321 dated -1520291
c 2 2018-09-04 10:03:00 S01 SHORT filled 10:03:01 98000 510 12:51:01 98000 stop 0 0 98722 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 2 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VS 5 count=3
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101005 495 10:00:01 98000 stop 0 0 100802 100848 0 -1487475 32321 dated -1519796
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:01 101010 495 10:00:01 98000 stop 0 0 100827 100848 0 -1489950 32321 dated -1522271
c 2 2018-09-04 10:05:00 S01 SHORT filled 10:05:01 98000 510 14:50:01 98000 stop 0 0 98750 98000 0 0 32679 dated -32679
f 0 dated-worst 4000 3202 148 12300 9900 1296 1250 225 0 0 32321 today-worst 4000 3024 99 12300 1500 1282 1250 225 0 0 23680 dated-precise 30797 today-precise 22205
f 2 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VS 15 count=2
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 98000 510 10:01:01 98000 stop 0 0 100805 100805 0 0 32679 dated -32679
c 1 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 98784 98106 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
f 1 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VS 30 count=1
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 100000 98232 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
candidates VS 60 count=1
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98000 510 15:08:01 98000 time 0 0 100000 98394 0 0 32679 dated -32679
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 0 0 32679 today-worst 4000 3069 100 12500 1500 1290 1250 225 0 0 23934 dated-precise 31200 today-precise 22459
row VO 1 LONG verdict=MISS trades=1 net=-1517816 net-bp=-76 dd-bp=75 dd=1517816 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1517816 pf=0 charges=32321
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1517816 trades=1 bp=-76 pass=no gap=65178160000
row VO 1 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 4
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 1 BOTH verdict=MISS trades=2 net=-1550495 net-bp=-78 dd-bp=77 dd=1550495 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-775248 pf=0 charges=65000
counts signals=7 while-open=5 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 4
month 2018-09 capital=200000000 net=-1550495 trades=2 bp=-78 pass=no gap=65504950000
row VO 3 LONG verdict=MISS trades=1 net=-1518806 net-bp=-76 dd-bp=75 dd=1518806 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1518806 pf=0 charges=32321
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1518806 trades=1 bp=-76 pass=no gap=65188060000
row VO 3 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 4
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 3 BOTH verdict=MISS trades=2 net=-1551485 net-bp=-78 dd-bp=77 dd=1551485 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-775743 pf=0 charges=65000
counts signals=7 while-open=5 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 4
month 2018-09 capital=200000000 net=-1551485 trades=2 bp=-78 pass=no gap=65514850000
row VO 5 LONG verdict=MISS trades=1 net=-1519796 net-bp=-76 dd-bp=75 dd=1519796 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1519796 pf=0 charges=32321
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1519796 trades=1 bp=-76 pass=no gap=65197960000
row VO 5 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 3
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 5 BOTH verdict=MISS trades=2 net=-1552475 net-bp=-78 dd-bp=77 dd=1552475 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-776238 pf=0 charges=65000
counts signals=6 while-open=4 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 3
month 2018-09 capital=200000000 net=-1552475 trades=2 bp=-78 pass=no gap=65524750000
row VO 15 LONG verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 15 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 1
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 15 BOTH verdict=MISS trades=2 net=-65358 net-bp=-4 dd-bp=3 dd=65358 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=65358
counts signals=4 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65358 trades=2 bp=-4 pass=no gap=50653580000
row VO 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 30 BOTH verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VO 60 BOTH verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 1 LONG verdict=MISS trades=1 net=-1517816 net-bp=-76 dd-bp=75 dd=1517816 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1517816 pf=0 charges=32321
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1517816 trades=1 bp=-76 pass=no gap=65178160000
row AV2 1 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 2
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 1 BOTH verdict=MISS trades=2 net=-1550495 net-bp=-78 dd-bp=77 dd=1550495 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-775248 pf=0 charges=65000
counts signals=3 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-1550495 trades=2 bp=-78 pass=no gap=65504950000
row AV2 3 LONG verdict=MISS trades=1 net=-1518806 net-bp=-76 dd-bp=75 dd=1518806 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1518806 pf=0 charges=32321
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1518806 trades=1 bp=-76 pass=no gap=65188060000
row AV2 3 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 2
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 3 BOTH verdict=MISS trades=2 net=-1551485 net-bp=-78 dd-bp=77 dd=1551485 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-775743 pf=0 charges=65000
counts signals=3 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-1551485 trades=2 bp=-78 pass=no gap=65514850000
row AV2 5 LONG verdict=MISS trades=1 net=-1519796 net-bp=-76 dd-bp=75 dd=1519796 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1519796 pf=0 charges=32321
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1519796 trades=1 bp=-76 pass=no gap=65197960000
row AV2 5 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 2
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 5 BOTH verdict=MISS trades=2 net=-1552475 net-bp=-78 dd-bp=77 dd=1552475 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-776238 pf=0 charges=65000
counts signals=3 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-1552475 trades=2 bp=-78 pass=no gap=65524750000
row AV2 15 LONG verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 15 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 1
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 15 BOTH verdict=MISS trades=2 net=-65358 net-bp=-4 dd-bp=3 dd=65358 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=65358
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65358 trades=2 bp=-4 pass=no gap=50653580000
row AV2 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 30 BOTH verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row AV2 60 BOTH verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 1 LONG verdict=MISS trades=1 net=-1517816 net-bp=-76 dd-bp=75 dd=1517816 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1517816 pf=0 charges=32321
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1517816 trades=1 bp=-76 pass=no gap=65178160000
row VS 1 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 2
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 1 BOTH verdict=MISS trades=2 net=-1550495 net-bp=-78 dd-bp=77 dd=1550495 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-775248 pf=0 charges=65000
counts signals=3 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-1550495 trades=2 bp=-78 pass=no gap=65504950000
row VS 3 LONG verdict=MISS trades=1 net=-1518806 net-bp=-76 dd-bp=75 dd=1518806 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1518806 pf=0 charges=32321
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1518806 trades=1 bp=-76 pass=no gap=65188060000
row VS 3 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 2
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 3 BOTH verdict=MISS trades=2 net=-1551485 net-bp=-78 dd-bp=77 dd=1551485 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-775743 pf=0 charges=65000
counts signals=3 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-1551485 trades=2 bp=-78 pass=no gap=65514850000
row VS 5 LONG verdict=MISS trades=1 net=-1519796 net-bp=-76 dd-bp=75 dd=1519796 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-76 median=-76 best=-76 floor=-76 win-rate=0 avg-win=n/a avg-loss=-1519796 pf=0 charges=32321
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-1519796 trades=1 bp=-76 pass=no gap=65197960000
row VS 5 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 2
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 5 BOTH verdict=MISS trades=2 net=-1552475 net-bp=-78 dd-bp=77 dd=1552475 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-78 median=-78 best=-78 floor=-78 win-rate=0 avg-win=n/a avg-loss=-776238 pf=0 charges=65000
counts signals=3 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=2 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 2
month 2018-09 capital=200000000 net=-1552475 trades=2 bp=-78 pass=no gap=65524750000
row VS 15 LONG verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 15 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 1
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 15 BOTH verdict=MISS trades=2 net=-65358 net-bp=-4 dd-bp=3 dd=65358 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-4 median=-4 best=-4 floor=-4 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=65358
counts signals=2 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0 1
month 2018-09 capital=200000000 net=-65358 trades=2 bp=-4 pass=no gap=50653580000
row VS 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 30 BOTH verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 SHORT verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
row VS 60 BOTH verdict=MISS trades=1 net=-32679 net-bp=-2 dd-bp=1 dd=32679 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32679 pf=0 charges=32679
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32679 trades=1 bp=-2 pass=no gap=50326790000
rank 1 VO 30 LONG
rank 2 VO 60 LONG
rank 3 AV2 30 LONG
rank 4 AV2 60 LONG
rank 5 VS 30 LONG
rank 6 VS 60 LONG
rank 7 VO 1 SHORT
rank 8 VO 3 SHORT
rank 9 VO 5 SHORT
rank 10 VO 15 LONG
rank 11 VO 15 SHORT
rank 12 VO 30 SHORT
rank 13 VO 30 BOTH
rank 14 VO 60 SHORT
rank 15 VO 60 BOTH
rank 16 AV2 1 SHORT
rank 17 AV2 3 SHORT
rank 18 AV2 5 SHORT
rank 19 AV2 15 LONG
rank 20 AV2 15 SHORT
rank 21 AV2 30 SHORT
rank 22 AV2 30 BOTH
rank 23 AV2 60 SHORT
rank 24 AV2 60 BOTH
rank 25 VS 1 SHORT
rank 26 VS 3 SHORT
rank 27 VS 5 SHORT
rank 28 VS 15 LONG
rank 29 VS 15 SHORT
rank 30 VS 30 SHORT
rank 31 VS 30 BOTH
rank 32 VS 60 SHORT
rank 33 VS 60 BOTH
rank 34 VO 15 BOTH
rank 35 AV2 15 BOTH
rank 36 VS 15 BOTH
rank 37 VO 1 LONG
rank 38 AV2 1 LONG
rank 39 VS 1 LONG
rank 40 VO 3 LONG
rank 41 AV2 3 LONG
rank 42 VS 3 LONG
rank 43 VO 5 LONG
rank 44 AV2 5 LONG
rank 45 VS 5 LONG
rank 46 VO 1 BOTH
rank 47 AV2 1 BOTH
rank 48 VS 1 BOTH
rank 49 VO 3 BOTH
rank 50 AV2 3 BOTH
rank 51 VS 3 BOTH
rank 52 VO 5 BOTH
rank 53 AV2 5 BOTH
rank 54 VS 5 BOTH
head 1 LONG AV2/VO trades=1000 net=base:-1517816 dd-bp=1000 dd=1000
head 1 LONG VS/VO trades=1000 net=base:-1517816 dd-bp=1000 dd=1000
head 1 LONG VS/AV2 trades=1000 net=base:-1517816 dd-bp=1000 dd=1000
head 1 SHORT AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 1 SHORT VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 1 SHORT VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 1 BOTH AV2/VO trades=1000 net=base:-1550495 dd-bp=1000 dd=1000
head 1 BOTH VS/VO trades=1000 net=base:-1550495 dd-bp=1000 dd=1000
head 1 BOTH VS/AV2 trades=1000 net=base:-1550495 dd-bp=1000 dd=1000
head 3 LONG AV2/VO trades=1000 net=base:-1518806 dd-bp=1000 dd=1000
head 3 LONG VS/VO trades=1000 net=base:-1518806 dd-bp=1000 dd=1000
head 3 LONG VS/AV2 trades=1000 net=base:-1518806 dd-bp=1000 dd=1000
head 3 SHORT AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 3 SHORT VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 3 SHORT VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 3 BOTH AV2/VO trades=1000 net=base:-1551485 dd-bp=1000 dd=1000
head 3 BOTH VS/VO trades=1000 net=base:-1551485 dd-bp=1000 dd=1000
head 3 BOTH VS/AV2 trades=1000 net=base:-1551485 dd-bp=1000 dd=1000
head 5 LONG AV2/VO trades=1000 net=base:-1519796 dd-bp=1000 dd=1000
head 5 LONG VS/VO trades=1000 net=base:-1519796 dd-bp=1000 dd=1000
head 5 LONG VS/AV2 trades=1000 net=base:-1519796 dd-bp=1000 dd=1000
head 5 SHORT AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 5 SHORT VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 5 SHORT VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 5 BOTH AV2/VO trades=1000 net=base:-1552475 dd-bp=1000 dd=1000
head 5 BOTH VS/VO trades=1000 net=base:-1552475 dd-bp=1000 dd=1000
head 5 BOTH VS/AV2 trades=1000 net=base:-1552475 dd-bp=1000 dd=1000
head 15 LONG AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 15 LONG VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 15 LONG VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 15 SHORT AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 15 SHORT VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 15 SHORT VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 15 BOTH AV2/VO trades=1000 net=base:-65358 dd-bp=1000 dd=1000
head 15 BOTH VS/VO trades=1000 net=base:-65358 dd-bp=1000 dd=1000
head 15 BOTH VS/AV2 trades=1000 net=base:-65358 dd-bp=1000 dd=1000
head 30 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 30 SHORT VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 30 SHORT VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 30 BOTH AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 30 BOTH VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 30 BOTH VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 60 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 60 SHORT VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 60 SHORT VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 60 BOTH AV2/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 60 BOTH VS/VO trades=1000 net=base:-32679 dd-bp=1000 dd=1000
head 60 BOTH VS/AV2 trades=1000 net=base:-32679 dd-bp=1000 dd=1000
nearest-met-floor 0 VO 30 LONG
```
