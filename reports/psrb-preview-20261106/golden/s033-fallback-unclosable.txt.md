# Golden scenario `s033-fallback-unclosable`

PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed.

- format: `psrb-ref-golden/1`; written by crate `psrb_preview_ref`, session token `PSRB-PREVIEW-REF-20261106-1`
- seed: `0x0000000000000000`
- plants: a fallback day whose minutes are padded from 11:00: unclosable at the worst minute since entry
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
day 2018-09-04 minutes=375 seconds=0
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
m 10:00 101015 101015 101015 101015 1000
m 10:01 101016 101016 101016 101016 1000
m 10:02 101017 101017 101017 101017 1000
m 10:03 101018 101018 101018 101018 1000
m 10:04 101019 101019 101019 101019 1000
m 10:05 101020 101020 101020 101020 1000
m 10:06 101021 101021 101021 101021 1000
m 10:07 101022 101022 101022 101022 1000
m 10:08 101023 101023 101023 101023 1000
m 10:09 101024 101024 101024 101024 1000
m 10:10 101025 101025 101025 101025 1000
m 10:11 101026 101026 101026 101026 1000
m 10:12 101027 101027 101027 101027 1000
m 10:13 101028 101028 101028 101028 1000
m 10:14 101029 101029 101029 101029 1000
m 10:15 101030 101030 101030 101030 1000
m 10:16 101031 101031 101031 101031 1000
m 10:17 101032 101032 101032 101032 1000
m 10:18 101033 101033 101033 101033 1000
m 10:19 101034 101034 101034 101034 1000
m 10:20 101035 101035 101035 101035 1000
m 10:21 101036 101036 101036 101036 1000
m 10:22 101037 101037 101037 101037 1000
m 10:23 101038 101038 101038 101038 1000
m 10:24 101039 101039 101039 101039 1000
m 10:25 101040 101040 101040 101040 1000
m 10:26 101041 101041 101041 101041 1000
m 10:27 101042 101042 101042 101042 1000
m 10:28 101043 101043 101043 101043 1000
m 10:29 101044 101044 101044 101044 1000
m 10:30 101045 101045 101045 101045 1000
m 10:31 101046 101046 101046 101046 1000
m 10:32 101047 101047 101047 101047 1000
m 10:33 101048 101048 101048 101048 1000
m 10:34 101049 101049 101049 101049 1000
m 10:35 101050 101050 101050 101050 1000
m 10:36 101051 101051 101051 101051 1000
m 10:37 101052 101052 101052 101052 1000
m 10:38 101053 101053 101053 101053 1000
m 10:39 101054 101054 101054 101054 1000
m 10:40 101055 101055 101055 101055 1000
m 10:41 101056 101056 101056 101056 1000
m 10:42 101057 101057 101057 101057 1000
m 10:43 101058 101058 101058 101058 1000
m 10:44 101059 101059 101059 101059 1000
m 10:45 101060 101060 101060 101060 1000
m 10:46 101061 101061 101061 101061 1000
m 10:47 101062 101062 101062 101062 1000
m 10:48 101063 101063 101063 101063 1000
m 10:49 101064 101064 101064 101064 1000
m 10:50 101065 101065 101065 101065 1000
m 10:51 101066 101066 101066 101066 1000
m 10:52 101067 101067 101067 101067 1000
m 10:53 101068 101068 101068 101068 1000
m 10:54 101069 101069 101069 101069 1000
m 10:55 101070 101070 101070 101070 1000
m 10:56 101071 101071 101071 101071 1000
m 10:57 101072 101072 101072 101072 1000
m 10:58 101073 101073 101073 101073 1000
m 10:59 101074 101074 101074 101074 1000
m 11:00 101074 101074 101074 101074 0 x270
day 2018-09-05 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-06 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-07 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-10 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-11 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-12 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-13 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-14 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-17 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-18 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-19 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-20 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-21 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-24 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-25 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-26 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-27 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
day 2018-09-28 minutes=375 seconds=375
m 09:15 101074 101074 101074 101074 1000 x375
s 09:15:01 101074 101074 101074 101074 10 x375 every 60
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
candidates VO 1 count=4
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:59 101001 495 15:30:00 101001 unclosable 1 0 100700 101071 0 0 35041 dated -35041
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:59 101002 495 15:30:00 101002 unclosable 1 0 100731 101071 0 0 35041 dated -35041
c 2 2018-09-04 09:48:00 S01 LONG filled 09:48:59 101003 495 15:30:00 101003 unclosable 1 0 100759 101071 0 0 35041 dated -35041
c 3 2018-09-04 09:49:00 S01 LONG filled 09:49:59 101004 495 15:30:00 101004 unclosable 1 0 100784 101071 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VO 3 count=4
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:59 101003 495 15:30:00 101003 unclosable 1 0 100701 101043 0 0 35041 dated -35041
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:59 101006 495 15:30:00 101006 unclosable 1 0 100734 101043 0 0 35041 dated -35041
c 2 2018-09-04 09:54:00 S01 LONG filled 09:54:59 101009 495 15:30:00 101009 unclosable 1 0 100764 101043 0 0 35041 dated -35041
c 3 2018-09-04 09:57:00 S01 LONG filled 09:57:59 101012 494 15:30:00 101012 unclosable 1 0 100791 101043 0 0 35033 dated -35033
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VO 5 count=4
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:59 101005 495 15:30:00 101005 unclosable 1 0 100702 100994 0 0 35041 dated -35041
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:59 101010 495 15:30:00 101010 unclosable 1 0 100737 100994 0 0 35041 dated -35041
c 2 2018-09-04 10:00:00 S01 LONG filled 10:00:59 101015 494 15:30:00 101015 unclosable 1 0 100766 100994 0 0 35033 dated -35033
c 3 2018-09-04 10:05:00 S01 LONG filled 10:05:59 101020 494 15:30:00 101020 unclosable 1 0 100795 100994 0 0 35033 dated -35033
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VO 15 count=4
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:59 101015 494 15:30:00 101015 unclosable 1 0 100704 100854 0 0 35033 dated -35033
c 1 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100746 100854 0 0 35033 dated -35033
c 2 2018-09-04 10:30:00 S01 LONG filled 10:30:59 101045 494 15:30:00 101045 unclosable 1 0 100785 100854 0 0 35034 dated -35034
c 3 2018-09-04 10:45:00 S01 LONG filled 10:45:59 101060 494 15:30:00 101060 unclosable 1 0 100821 100854 0 0 35035 dated -35035
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33538 today-precise 24814
candidates VO 30 count=3
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100708 100806 0 0 35033 dated -35033
c 1 2018-09-04 10:45:00 S01 LONG filled 10:45:59 101060 494 15:30:00 101060 unclosable 1 0 100759 100806 0 0 35035 dated -35035
c 2 2018-09-04 11:15:00 S01 LONG not-filled
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33540 today-precise 24814
candidates VO 60 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100208 100761 0 0 35033 dated -35033
c 1 2018-09-04 11:15:00 S01 LONG not-filled
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33540 today-precise 24814
candidates AV2 1 count=2
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:59 101001 495 15:30:00 101001 unclosable 1 0 100800 101072 0 0 35041 dated -35041
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:59 101002 495 15:30:00 101002 unclosable 1 0 100821 101072 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates AV2 3 count=2
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:59 101003 495 15:30:00 101003 unclosable 1 0 100801 101053 0 0 35041 dated -35041
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:59 101006 495 15:30:00 101006 unclosable 1 0 100824 101053 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates AV2 5 count=2
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:59 101005 495 15:30:00 101005 unclosable 1 0 100802 101020 0 0 35041 dated -35041
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:59 101010 495 15:30:00 101010 unclosable 1 0 100827 101020 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates AV2 15 count=2
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:59 101015 494 15:30:00 101015 unclosable 1 0 100805 100925 0 0 35033 dated -35033
c 1 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100838 100925 0 0 35033 dated -35033
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33538 today-precise 24814
candidates AV2 30 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100810 100893 0 0 35033 dated -35033
c 1 2018-09-04 10:45:00 S01 LONG filled 10:45:59 101060 494 15:30:00 101060 unclosable 1 0 100854 100893 0 0 35035 dated -35035
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33540 today-precise 24814
candidates AV2 60 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100310 100858 0 0 35033 dated -35033
c 1 2018-09-04 11:15:00 S01 LONG not-filled
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33540 today-precise 24814
candidates VS 1 count=2
c 0 2018-09-04 09:46:00 S01 LONG filled 09:46:59 101001 495 15:30:00 101001 unclosable 1 0 100800 101072 0 0 35041 dated -35041
c 1 2018-09-04 09:47:00 S01 LONG filled 09:47:59 101002 495 15:30:00 101002 unclosable 1 0 100821 101072 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VS 3 count=2
c 0 2018-09-04 09:48:00 S01 LONG filled 09:48:59 101003 495 15:30:00 101003 unclosable 1 0 100801 101053 0 0 35041 dated -35041
c 1 2018-09-04 09:51:00 S01 LONG filled 09:51:59 101006 495 15:30:00 101006 unclosable 1 0 100824 101053 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VS 5 count=2
c 0 2018-09-04 09:50:00 S01 LONG filled 09:50:59 101005 495 15:30:00 101005 unclosable 1 0 100802 101020 0 0 35041 dated -35041
c 1 2018-09-04 09:55:00 S01 LONG filled 09:55:59 101010 495 15:30:00 101010 unclosable 1 0 100827 101020 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VS 15 count=2
c 0 2018-09-04 10:00:00 S01 LONG filled 10:00:59 101015 494 15:30:00 101015 unclosable 1 0 100805 100925 0 0 35033 dated -35033
c 1 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100838 100925 0 0 35033 dated -35033
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33538 today-precise 24814
candidates VS 30 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100810 100893 0 0 35033 dated -35033
c 1 2018-09-04 10:45:00 S01 LONG filled 10:45:59 101060 494 15:30:00 101060 unclosable 1 0 100854 100893 0 0 35035 dated -35035
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33540 today-precise 24814
candidates VS 60 count=2
c 0 2018-09-04 10:15:00 S01 LONG filled 10:15:59 101030 494 15:30:00 101030 unclosable 1 0 100310 100858 0 0 35033 dated -35033
c 1 2018-09-04 11:15:00 S01 LONG not-filled
f 0 dated-worst 4000 3244 150 12500 10000 1304 1250 225 2000 360 35033 today-worst 4000 3064 100 12500 1500 1290 1250 225 2000 360 26289 dated-precise 33540 today-precise 24814
row VO 1 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 1 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 3 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 5 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 15 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VO 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VO 30 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VO 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VO 60 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VO 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row AV2 1 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 3 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 5 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 15 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row AV2 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row AV2 30 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row AV2 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row AV2 60 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row AV2 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VS 1 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 1 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 3 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 3 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 5 LONG verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 5 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 15 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VS 15 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VS 30 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VS 30 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VS 60 LONG verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
row VS 60 SHORT verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 BOTH verdict=MISS trades=1 net=-35033 net-bp=-2 dd-bp=1 dd=35033 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35033 pf=0 charges=35033
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=1 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35033 trades=1 bp=-2 pass=no gap=50350330000
rank 1 VO 1 SHORT
rank 2 VO 3 SHORT
rank 3 VO 5 SHORT
rank 4 VO 15 SHORT
rank 5 VO 30 SHORT
rank 6 VO 60 SHORT
rank 7 AV2 1 SHORT
rank 8 AV2 3 SHORT
rank 9 AV2 5 SHORT
rank 10 AV2 15 SHORT
rank 11 AV2 30 SHORT
rank 12 AV2 60 SHORT
rank 13 VS 1 SHORT
rank 14 VS 3 SHORT
rank 15 VS 5 SHORT
rank 16 VS 15 SHORT
rank 17 VS 30 SHORT
rank 18 VS 60 SHORT
rank 19 VO 15 LONG
rank 20 VO 15 BOTH
rank 21 VO 30 LONG
rank 22 VO 30 BOTH
rank 23 VO 60 LONG
rank 24 VO 60 BOTH
rank 25 AV2 15 LONG
rank 26 AV2 15 BOTH
rank 27 AV2 30 LONG
rank 28 AV2 30 BOTH
rank 29 AV2 60 LONG
rank 30 AV2 60 BOTH
rank 31 VS 15 LONG
rank 32 VS 15 BOTH
rank 33 VS 30 LONG
rank 34 VS 30 BOTH
rank 35 VS 60 LONG
rank 36 VS 60 BOTH
rank 37 VO 1 LONG
rank 38 VO 1 BOTH
rank 39 VO 3 LONG
rank 40 VO 3 BOTH
rank 41 VO 5 LONG
rank 42 VO 5 BOTH
rank 43 AV2 1 LONG
rank 44 AV2 1 BOTH
rank 45 AV2 3 LONG
rank 46 AV2 3 BOTH
rank 47 AV2 5 LONG
rank 48 AV2 5 BOTH
rank 49 VS 1 LONG
rank 50 VS 1 BOTH
rank 51 VS 3 LONG
rank 52 VS 3 BOTH
rank 53 VS 5 LONG
rank 54 VS 5 BOTH
head 1 LONG AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 LONG VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 LONG VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 LONG AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 LONG VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 LONG VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 LONG AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 LONG VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 LONG VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 LONG AV2/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 15 LONG VS/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 15 LONG VS/AV2 trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 15 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 BOTH AV2/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 15 BOTH VS/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 15 BOTH VS/AV2 trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 30 LONG AV2/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 30 LONG VS/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 30 LONG VS/AV2 trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 30 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 BOTH AV2/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 30 BOTH VS/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 30 BOTH VS/AV2 trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 60 LONG AV2/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 60 LONG VS/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 60 LONG VS/AV2 trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 60 SHORT AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 BOTH AV2/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 60 BOTH VS/VO trades=1000 net=base:-35033 dd-bp=1000 dd=1000
head 60 BOTH VS/AV2 trades=1000 net=base:-35033 dd-bp=1000 dd=1000
nearest-met-floor 0 VO 1 SHORT
```
