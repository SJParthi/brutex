# Golden scenario `s014-entry-is-the-last-second`

PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed.

- format: `psrb-ref-golden/1`; written by crate `psrb_preview_ref`, session token `PSRB-PREVIEW-REF-20261106-1`
- seed: `0x0000000000000000`
- plants: the entry fills on the day's last traded second: unclosable at that second's worst print
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
day 2018-09-04 minutes=375 seconds=287
m 09:15 100000 100000 100000 100000 1000 x285
m 14:00 101000 101000 101000 101000 1000
m 14:01 101000 101000 101000 101000 0 x89
s 09:15:01 100000 100000 100000 100000 10 x285 every 60
s 14:00:01 101000 101000 101000 101000 10
s 14:01:30 101000 101009 100991 101000 10
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
candidates VO 1 count=1
c 0 2018-09-04 14:01:00 S01 LONG filled 14:01:30 101009 495 15:30:00 100991 unclosable 0 0 100700 100700 0 -8910 35041 dated -43951
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VO 3 count=1
c 0 2018-09-04 14:03:00 S01 LONG not-filled
candidates VO 5 count=1
c 0 2018-09-04 14:05:00 S01 LONG not-filled
candidates VO 15 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates VO 30 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates VO 60 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates AV2 1 count=1
c 0 2018-09-04 14:01:00 S01 LONG filled 14:01:30 101009 495 15:30:00 100991 unclosable 0 0 100800 100800 0 -8910 35041 dated -43951
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates AV2 3 count=1
c 0 2018-09-04 14:03:00 S01 LONG not-filled
candidates AV2 5 count=1
c 0 2018-09-04 14:05:00 S01 LONG not-filled
candidates AV2 15 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates AV2 30 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates AV2 60 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates VS 1 count=1
c 0 2018-09-04 14:01:00 S01 LONG filled 14:01:30 101009 495 15:30:00 100991 unclosable 0 0 100800 100800 0 -8910 35041 dated -43951
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VS 3 count=1
c 0 2018-09-04 14:03:00 S01 LONG not-filled
candidates VS 5 count=1
c 0 2018-09-04 14:05:00 S01 LONG not-filled
candidates VS 15 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates VS 30 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
candidates VS 60 count=1
c 0 2018-09-04 14:15:00 S01 LONG not-filled
row VO 1 LONG verdict=MISS trades=1 net=-43951 net-bp=-3 dd-bp=2 dd=43951 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-3 median=-3 best=-3 floor=-3 win-rate=0 avg-win=n/a avg-loss=-43951 pf=0 charges=35041
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-43951 trades=1 bp=-3 pass=no gap=50439510000
row VO 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 1 BOTH verdict=MISS trades=1 net=-43951 net-bp=-3 dd-bp=2 dd=43951 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-3 median=-3 best=-3 floor=-3 win-rate=0 avg-win=n/a avg-loss=-43951 pf=0 charges=35041
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-43951 trades=1 bp=-3 pass=no gap=50439510000
row VO 3 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 LONG verdict=MISS trades=1 net=-43951 net-bp=-3 dd-bp=2 dd=43951 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-3 median=-3 best=-3 floor=-3 win-rate=0 avg-win=n/a avg-loss=-43951 pf=0 charges=35041
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-43951 trades=1 bp=-3 pass=no gap=50439510000
row AV2 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 BOTH verdict=MISS trades=1 net=-43951 net-bp=-3 dd-bp=2 dd=43951 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-3 median=-3 best=-3 floor=-3 win-rate=0 avg-win=n/a avg-loss=-43951 pf=0 charges=35041
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-43951 trades=1 bp=-3 pass=no gap=50439510000
row AV2 3 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 LONG verdict=MISS trades=1 net=-43951 net-bp=-3 dd-bp=2 dd=43951 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-3 median=-3 best=-3 floor=-3 win-rate=0 avg-win=n/a avg-loss=-43951 pf=0 charges=35041
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-43951 trades=1 bp=-3 pass=no gap=50439510000
row VS 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 BOTH verdict=MISS trades=1 net=-43951 net-bp=-3 dd-bp=2 dd=43951 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-3 median=-3 best=-3 floor=-3 win-rate=0 avg-win=n/a avg-loss=-43951 pf=0 charges=35041
counts signals=1 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-43951 trades=1 bp=-3 pass=no gap=50439510000
row VS 3 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 BOTH verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=1 while-open=0 cut=0 not-filled=1 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
rank 1 VO 1 SHORT
rank 2 VO 3 LONG
rank 3 VO 3 SHORT
rank 4 VO 3 BOTH
rank 5 VO 5 LONG
rank 6 VO 5 SHORT
rank 7 VO 5 BOTH
rank 8 VO 15 LONG
rank 9 VO 15 SHORT
rank 10 VO 15 BOTH
rank 11 VO 30 LONG
rank 12 VO 30 SHORT
rank 13 VO 30 BOTH
rank 14 VO 60 LONG
rank 15 VO 60 SHORT
rank 16 VO 60 BOTH
rank 17 AV2 1 SHORT
rank 18 AV2 3 LONG
rank 19 AV2 3 SHORT
rank 20 AV2 3 BOTH
rank 21 AV2 5 LONG
rank 22 AV2 5 SHORT
rank 23 AV2 5 BOTH
rank 24 AV2 15 LONG
rank 25 AV2 15 SHORT
rank 26 AV2 15 BOTH
rank 27 AV2 30 LONG
rank 28 AV2 30 SHORT
rank 29 AV2 30 BOTH
rank 30 AV2 60 LONG
rank 31 AV2 60 SHORT
rank 32 AV2 60 BOTH
rank 33 VS 1 SHORT
rank 34 VS 3 LONG
rank 35 VS 3 SHORT
rank 36 VS 3 BOTH
rank 37 VS 5 LONG
rank 38 VS 5 SHORT
rank 39 VS 5 BOTH
rank 40 VS 15 LONG
rank 41 VS 15 SHORT
rank 42 VS 15 BOTH
rank 43 VS 30 LONG
rank 44 VS 30 SHORT
rank 45 VS 30 BOTH
rank 46 VS 60 LONG
rank 47 VS 60 SHORT
rank 48 VS 60 BOTH
rank 49 VO 1 LONG
rank 50 VO 1 BOTH
rank 51 AV2 1 LONG
rank 52 AV2 1 BOTH
rank 53 VS 1 LONG
rank 54 VS 1 BOTH
head 1 LONG AV2/VO trades=1000 net=base:-43951 dd-bp=1000 dd=1000
head 1 LONG VS/VO trades=1000 net=base:-43951 dd-bp=1000 dd=1000
head 1 LONG VS/AV2 trades=1000 net=base:-43951 dd-bp=1000 dd=1000
head 1 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 BOTH AV2/VO trades=1000 net=base:-43951 dd-bp=1000 dd=1000
head 1 BOTH VS/VO trades=1000 net=base:-43951 dd-bp=1000 dd=1000
head 1 BOTH VS/AV2 trades=1000 net=base:-43951 dd-bp=1000 dd=1000
head 3 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 BOTH AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 BOTH VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 BOTH VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 BOTH AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 BOTH VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 BOTH VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 BOTH AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 BOTH VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 BOTH VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 BOTH AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 BOTH VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 BOTH VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 BOTH AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 BOTH VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 BOTH VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
nearest-met-floor 0 VO 1 SHORT
```
