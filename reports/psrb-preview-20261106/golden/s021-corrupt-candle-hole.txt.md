# Golden scenario `s021-corrupt-candle-hole`

PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed.

- format: `psrb-ref-golden/1`; written by crate `psrb_preview_ref`, session token `PSRB-PREVIEW-REF-20261106-1`
- seed: `0x0000000000000000`
- plants: a minute with high below low: the candle is refused, a hole, and the jump right after it decides nothing
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
m 09:15 100000 100000 100000 100000 1000 x29
m 09:44 100000 99990 100010 100000 1000
m 09:45 101000 101000 101000 101000 1000 x345
s 09:15:01 100000 100000 100000 100000 10 x30 every 60
s 09:45:01 101000 101000 101000 101000 10 x345 every 60
day 2018-09-05 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-06 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-07 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-10 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-11 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-12 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-13 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-14 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-17 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-18 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-19 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-20 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-21 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-24 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-25 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-26 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-27 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
day 2018-09-28 minutes=375 seconds=375
m 09:15 101000 101000 101000 101000 1000 x375
s 09:15:01 101000 101000 101000 101000 10 x375 every 60
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
candidates VO 3 count=3
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 12:00:01 101000 stop 0 0 100700 101000 0 0 32681 dated -32681
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:01 101000 495 12:00:01 101000 stop 0 0 100730 101000 0 0 32681 dated -32681
c 2 2018-09-04 09:54:00 S01 LONG filled 09:54:01 101000 495 12:00:01 101000 stop 0 0 100757 101000 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VO 5 count=3
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101000 495 13:30:01 101000 stop 0 0 100700 101000 0 0 32681 dated -32681
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:01 101000 495 13:30:01 101000 stop 0 0 100730 101000 0 0 32681 dated -32681
c 2 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 13:30:01 101000 stop 0 0 100757 101000 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VO 15 count=3
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 15:08:01 101000 time 0 0 100700 100964 0 0 32681 dated -32681
c 1 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100730 100964 0 0 32681 dated -32681
c 2 2018-09-04 10:30:00 S01 LONG filled 10:30:01 101000 495 15:08:01 101000 time 0 0 100757 100964 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VO 30 count=3
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100700 100886 0 0 32681 dated -32681
c 1 2018-09-04 10:45:00 S01 LONG filled 10:45:01 101000 495 15:08:01 101000 time 0 0 100730 100886 0 0 32681 dated -32681
c 2 2018-09-04 11:15:00 S01 LONG filled 11:15:01 101000 495 15:08:01 101000 time 0 0 100757 100886 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VO 60 count=3
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100200 100805 0 0 32681 dated -32681
c 1 2018-09-04 11:15:00 S01 LONG filled 11:15:01 101000 495 15:08:01 101000 time 0 0 100730 100805 0 0 32681 dated -32681
c 2 2018-09-04 12:15:00 S01 LONG filled 12:15:01 101000 495 15:08:01 101000 time 0 0 100757 100805 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates AV2 3 count=1
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 12:00:01 101000 stop 0 0 100800 101000 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates AV2 5 count=1
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101000 495 13:30:01 101000 stop 0 0 100800 101000 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates AV2 15 count=1
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 15:08:01 101000 time 0 0 100800 100976 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates AV2 30 count=1
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100800 100924 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates AV2 60 count=1
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100300 100870 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VS 3 count=1
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:01 101000 495 12:00:01 101000 stop 0 0 100800 101000 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VS 5 count=1
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:01 101000 495 13:30:01 101000 stop 0 0 100800 101000 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VS 15 count=1
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:01 101000 495 15:08:01 101000 time 0 0 100800 100976 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VS 30 count=1
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100800 100924 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
candidates VS 60 count=1
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:01 101000 495 15:08:01 101000 time 0 0 100300 100870 0 0 32681 dated -32681
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 0 0 32681 today-worst 4000 3070 100 12500 1500 1291 1250 225 0 0 23936 dated-precise 31204 today-precise 22461
row VO 1 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 1 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 5 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 15 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 30 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 60 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VO 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 1 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:13,CCI:19,EMA:199
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:13,CCI:19,EMA:199
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:13,CCI:19,EMA:199
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 5 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 15 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 30 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 60 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row AV2 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 1 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:13,CCI:19,EMA:199
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:13,CCI:19,EMA:199
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:9,BB:19,RSI:13,WR:13,CCI:19,EMA:199
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 5 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=1 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 15 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 30 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 60 LONG verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
row VS 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 BOTH verdict=MISS trades=1 net=-32681 net-bp=-2 dd-bp=1 dd=32681 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-32681 pf=0 charges=32681
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=1 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-32681 trades=1 bp=-2 pass=no gap=50326810000
rank 1 VO 1 LONG
rank 2 VO 1 SHORT
rank 3 VO 1 BOTH
rank 4 VO 3 SHORT
rank 5 VO 5 SHORT
rank 6 VO 15 SHORT
rank 7 VO 30 SHORT
rank 8 VO 60 SHORT
rank 9 AV2 1 LONG
rank 10 AV2 1 SHORT
rank 11 AV2 1 BOTH
rank 12 AV2 3 SHORT
rank 13 AV2 5 SHORT
rank 14 AV2 15 SHORT
rank 15 AV2 30 SHORT
rank 16 AV2 60 SHORT
rank 17 VS 1 LONG
rank 18 VS 1 SHORT
rank 19 VS 1 BOTH
rank 20 VS 3 SHORT
rank 21 VS 5 SHORT
rank 22 VS 15 SHORT
rank 23 VS 30 SHORT
rank 24 VS 60 SHORT
rank 25 VO 3 LONG
rank 26 VO 3 BOTH
rank 27 VO 5 LONG
rank 28 VO 5 BOTH
rank 29 VO 15 LONG
rank 30 VO 15 BOTH
rank 31 VO 30 LONG
rank 32 VO 30 BOTH
rank 33 VO 60 LONG
rank 34 VO 60 BOTH
rank 35 AV2 3 LONG
rank 36 AV2 3 BOTH
rank 37 AV2 5 LONG
rank 38 AV2 5 BOTH
rank 39 AV2 15 LONG
rank 40 AV2 15 BOTH
rank 41 AV2 30 LONG
rank 42 AV2 30 BOTH
rank 43 AV2 60 LONG
rank 44 AV2 60 BOTH
rank 45 VS 3 LONG
rank 46 VS 3 BOTH
rank 47 VS 5 LONG
rank 48 VS 5 BOTH
rank 49 VS 15 LONG
rank 50 VS 15 BOTH
rank 51 VS 30 LONG
rank 52 VS 30 BOTH
rank 53 VS 60 LONG
rank 54 VS 60 BOTH
head 1 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 BOTH AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 BOTH VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 BOTH VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 LONG AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 3 LONG VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 3 LONG VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 3 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 BOTH AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 3 BOTH VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 3 BOTH VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 5 LONG AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 5 LONG VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 5 LONG VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 5 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 BOTH AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 5 BOTH VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 5 BOTH VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 15 LONG AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 15 LONG VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 15 LONG VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 15 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 BOTH AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 15 BOTH VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 15 BOTH VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 30 LONG AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 30 LONG VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 30 LONG VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 30 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 BOTH AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 30 BOTH VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 30 BOTH VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 60 LONG AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 60 LONG VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 60 LONG VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 60 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 BOTH AV2/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 60 BOTH VS/VO trades=1000 net=base:-32681 dd-bp=1000 dd=1000
head 60 BOTH VS/AV2 trades=1000 net=base:-32681 dd-bp=1000 dd=1000
nearest-met-floor 0 VO 1 LONG
```
