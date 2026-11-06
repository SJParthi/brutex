# Golden scenario `s013-halted-afternoon-short-unclosable`

PREVIEW: a synthetic reference scenario for differential testing of the PSRB preview tool; not the PSRB result; every price and trade here is generated with a fixed seed.

- format: `psrb-ref-golden/1`; written by crate `psrb_preview_ref`, session token `PSRB-PREVIEW-REF-20261106-1`
- seed: `0x0000000000000000`
- plants: the mirror: a short halted from 11:00 ends at its highest high since entry
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
day 2018-09-04 minutes=375 seconds=105
m 09:15 100000 100000 100000 100000 1000 x30
m 09:45 99000 99000 99000 99000 1000
m 09:46 98999 98999 98999 98999 1000
m 09:47 98998 98998 98998 98998 1000
m 09:48 98997 98997 98997 98997 1000
m 09:49 98996 98996 98996 98996 1000
m 09:50 98995 98995 98995 98995 1000
m 09:51 98994 98994 98994 98994 1000
m 09:52 98993 98993 98993 98993 1000
m 09:53 98992 98992 98992 98992 1000
m 09:54 98991 98991 98991 98991 1000
m 09:55 98990 98990 98990 98990 1000
m 09:56 98989 98989 98989 98989 1000
m 09:57 98988 98988 98988 98988 1000
m 09:58 98987 98987 98987 98987 1000
m 09:59 98986 98986 98986 98986 1000
m 10:00 98985 98985 98985 98985 1000
m 10:01 98984 98984 98984 98984 1000
m 10:02 98983 98983 98983 98983 1000
m 10:03 98982 98982 98982 98982 1000
m 10:04 98981 98981 98981 98981 1000
m 10:05 98980 98980 98980 98980 1000
m 10:06 98979 98979 98979 98979 1000
m 10:07 98978 98978 98978 98978 1000
m 10:08 98977 98977 98977 98977 1000
m 10:09 98976 98976 98976 98976 1000
m 10:10 98975 98975 98975 98975 1000
m 10:11 98974 98974 98974 98974 1000
m 10:12 98973 98973 98973 98973 1000
m 10:13 98972 98972 98972 98972 1000
m 10:14 98971 98971 98971 98971 1000
m 10:15 98970 98970 98970 98970 1000
m 10:16 98969 98969 98969 98969 1000
m 10:17 98968 98968 98968 98968 1000
m 10:18 98967 98967 98967 98967 1000
m 10:19 98966 98966 98966 98966 1000
m 10:20 98965 98965 98965 98965 1000
m 10:21 98964 98964 98964 98964 1000
m 10:22 98963 98963 98963 98963 1000
m 10:23 98962 98962 98962 98962 1000
m 10:24 98961 98961 98961 98961 1000
m 10:25 98960 98960 98960 98960 1000
m 10:26 98959 98959 98959 98959 1000
m 10:27 98958 98958 98958 98958 1000
m 10:28 98957 98957 98957 98957 1000
m 10:29 98956 98956 98956 98956 1000
m 10:30 98955 98955 98955 98955 1000
m 10:31 98954 98954 98954 98954 1000
m 10:32 98953 98953 98953 98953 1000
m 10:33 98952 98952 98952 98952 1000
m 10:34 98951 98951 98951 98951 1000
m 10:35 98950 98950 98950 98950 1000
m 10:36 98949 98949 98949 98949 1000
m 10:37 98948 98948 98948 98948 1000
m 10:38 98947 98947 98947 98947 1000
m 10:39 98946 98946 98946 98946 1000
m 10:40 98945 98945 98945 98945 1000
m 10:41 98944 98944 98944 98944 1000
m 10:42 98943 98943 98943 98943 1000
m 10:43 98942 98942 98942 98942 1000
m 10:44 98941 98941 98941 98941 1000
m 10:45 98940 98940 98940 98940 1000
m 10:46 98939 98939 98939 98939 1000
m 10:47 98938 98938 98938 98938 1000
m 10:48 98937 98937 98937 98937 1000
m 10:49 98936 98936 98936 98936 1000
m 10:50 98935 98935 98935 98935 1000
m 10:51 98934 98934 98934 98934 1000
m 10:52 98933 98933 98933 98933 1000
m 10:53 98932 98932 98932 98932 1000
m 10:54 98931 98931 98931 98931 1000
m 10:55 98930 98930 98930 98930 1000
m 10:56 98929 98929 98929 98929 1000
m 10:57 98928 98928 98928 98928 1000
m 10:58 98927 98927 98927 98927 1000
m 10:59 98926 98926 98926 98926 1000
m 11:00 98926 98926 98926 98926 0 x270
s 09:15:01 100000 100000 100000 100000 10 x30 every 60
s 09:45:01 99000 99000 99000 99000 10
s 09:46:01 98999 98999 98999 98999 10
s 09:47:01 98998 98998 98998 98998 10
s 09:48:01 98997 98997 98997 98997 10
s 09:49:01 98996 98996 98996 98996 10
s 09:50:01 98995 98995 98995 98995 10
s 09:51:01 98994 98994 98994 98994 10
s 09:52:01 98993 98993 98993 98993 10
s 09:53:01 98992 98992 98992 98992 10
s 09:54:01 98991 98991 98991 98991 10
s 09:55:01 98990 98990 98990 98990 10
s 09:56:01 98989 98989 98989 98989 10
s 09:57:01 98988 98988 98988 98988 10
s 09:58:01 98987 98987 98987 98987 10
s 09:59:01 98986 98986 98986 98986 10
s 10:00:01 98985 98985 98985 98985 10
s 10:01:01 98984 98984 98984 98984 10
s 10:02:01 98983 98983 98983 98983 10
s 10:03:01 98982 98982 98982 98982 10
s 10:04:01 98981 98981 98981 98981 10
s 10:05:01 98980 98980 98980 98980 10
s 10:06:01 98979 98979 98979 98979 10
s 10:07:01 98978 98978 98978 98978 10
s 10:08:01 98977 98977 98977 98977 10
s 10:09:01 98976 98976 98976 98976 10
s 10:10:01 98975 98975 98975 98975 10
s 10:11:01 98974 98974 98974 98974 10
s 10:12:01 98973 98973 98973 98973 10
s 10:13:01 98972 98972 98972 98972 10
s 10:14:01 98971 98971 98971 98971 10
s 10:15:01 98970 98970 98970 98970 10
s 10:16:01 98969 98969 98969 98969 10
s 10:17:01 98968 98968 98968 98968 10
s 10:18:01 98967 98967 98967 98967 10
s 10:19:01 98966 98966 98966 98966 10
s 10:20:01 98965 98965 98965 98965 10
s 10:21:01 98964 98964 98964 98964 10
s 10:22:01 98963 98963 98963 98963 10
s 10:23:01 98962 98962 98962 98962 10
s 10:24:01 98961 98961 98961 98961 10
s 10:25:01 98960 98960 98960 98960 10
s 10:26:01 98959 98959 98959 98959 10
s 10:27:01 98958 98958 98958 98958 10
s 10:28:01 98957 98957 98957 98957 10
s 10:29:01 98956 98956 98956 98956 10
s 10:30:01 98955 98955 98955 98955 10
s 10:31:01 98954 98954 98954 98954 10
s 10:32:01 98953 98953 98953 98953 10
s 10:33:01 98952 98952 98952 98952 10
s 10:34:01 98951 98951 98951 98951 10
s 10:35:01 98950 98950 98950 98950 10
s 10:36:01 98949 98949 98949 98949 10
s 10:37:01 98948 98948 98948 98948 10
s 10:38:01 98947 98947 98947 98947 10
s 10:39:01 98946 98946 98946 98946 10
s 10:40:01 98945 98945 98945 98945 10
s 10:41:01 98944 98944 98944 98944 10
s 10:42:01 98943 98943 98943 98943 10
s 10:43:01 98942 98942 98942 98942 10
s 10:44:01 98941 98941 98941 98941 10
s 10:45:01 98940 98940 98940 98940 10
s 10:46:01 98939 98939 98939 98939 10
s 10:47:01 98938 98938 98938 98938 10
s 10:48:01 98937 98937 98937 98937 10
s 10:49:01 98936 98936 98936 98936 10
s 10:50:01 98935 98935 98935 98935 10
s 10:51:01 98934 98934 98934 98934 10
s 10:52:01 98933 98933 98933 98933 10
s 10:53:01 98932 98932 98932 98932 10
s 10:54:01 98931 98931 98931 98931 10
s 10:55:01 98930 98930 98930 98930 10
s 10:56:01 98929 98929 98929 98929 10
s 10:57:01 98928 98928 98928 98928 10
s 10:58:01 98927 98927 98927 98927 10
s 10:59:01 98926 98926 98926 98926 10
day 2018-09-05 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-06 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-07 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-10 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-11 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-12 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-13 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-14 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-17 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-18 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-19 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-20 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-21 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-24 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-25 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-26 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-27 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
day 2018-09-28 minutes=375 seconds=375
m 09:15 98926 98926 98926 98926 1000 x375
s 09:15:01 98926 98926 98926 98926 10 x375 every 60
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
c 0 2018-09-04 09:46:00 S01 SHORT filled 09:46:01 98999 505 15:30:00 98999 unclosable 0 0 99300 98929 0 0 35041 dated -35041
c 1 2018-09-04 09:47:00 S01 SHORT filled 09:47:01 98998 505 15:30:00 98998 unclosable 0 0 99269 98929 0 0 35041 dated -35041
c 2 2018-09-04 09:48:00 S01 SHORT filled 09:48:01 98997 505 15:30:00 98997 unclosable 0 0 99241 98929 0 0 35041 dated -35041
c 3 2018-09-04 09:49:00 S01 SHORT filled 09:49:01 98996 505 15:30:00 98996 unclosable 0 0 99216 98929 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VO 3 count=4
c 0 2018-09-04 09:48:00 S01 SHORT filled 09:48:01 98997 505 15:30:00 98997 unclosable 0 0 99299 98957 0 0 35041 dated -35041
c 1 2018-09-04 09:51:00 S01 SHORT filled 09:51:01 98994 505 15:30:00 98994 unclosable 0 0 99266 98957 0 0 35041 dated -35041
c 2 2018-09-04 09:54:00 S01 SHORT filled 09:54:01 98991 505 15:30:00 98991 unclosable 0 0 99236 98957 0 0 35041 dated -35041
c 3 2018-09-04 09:57:00 S01 SHORT filled 09:57:01 98988 505 15:30:00 98988 unclosable 0 0 99209 98957 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VO 5 count=4
c 0 2018-09-04 09:50:00 S01 SHORT filled 09:50:01 98995 505 15:30:00 98995 unclosable 0 0 99298 99006 0 0 35041 dated -35041
c 1 2018-09-04 09:55:00 S01 SHORT filled 09:55:01 98990 505 15:30:00 98990 unclosable 0 0 99263 99006 0 0 35041 dated -35041
c 2 2018-09-04 10:00:00 S01 SHORT filled 10:00:01 98985 505 15:30:00 98985 unclosable 0 0 99234 99006 0 0 35041 dated -35041
c 3 2018-09-04 10:05:00 S01 SHORT filled 10:05:01 98980 505 15:30:00 98980 unclosable 0 0 99205 99006 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33563 today-precise 24821
candidates VO 15 count=4
c 0 2018-09-04 10:00:00 S01 SHORT filled 10:00:01 98985 505 15:30:00 98985 unclosable 0 0 99296 99146 0 0 35041 dated -35041
c 1 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99254 99146 0 0 35039 dated -35039
c 2 2018-09-04 10:30:00 S01 SHORT filled 10:30:01 98955 505 15:30:00 98955 unclosable 0 0 99215 99146 0 0 35038 dated -35038
c 3 2018-09-04 10:45:00 S01 SHORT filled 10:45:01 98940 505 15:30:00 98940 unclosable 0 0 99179 99146 0 0 35038 dated -35038
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33561 today-precise 24819
candidates VO 30 count=3
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99291 99194 0 0 35039 dated -35039
c 1 2018-09-04 10:45:00 S01 SHORT filled 10:45:01 98940 505 15:30:00 98940 unclosable 0 0 99240 99194 0 0 35038 dated -35038
c 2 2018-09-04 11:15:00 S01 SHORT not-filled
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 2000 360 35039 today-worst 4000 3069 100 12500 1500 1290 1250 225 2000 360 26294 dated-precise 33560 today-precise 24819
candidates VO 60 count=2
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99791 99239 0 0 35039 dated -35039
c 1 2018-09-04 11:15:00 S01 SHORT not-filled
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 2000 360 35039 today-worst 4000 3069 100 12500 1500 1290 1250 225 2000 360 26294 dated-precise 33560 today-precise 24819
candidates AV2 1 count=2
c 0 2018-09-04 09:46:00 S01 SHORT filled 09:46:01 98999 505 15:30:00 98999 unclosable 0 0 99200 98928 0 0 35041 dated -35041
c 1 2018-09-04 09:47:00 S01 SHORT filled 09:47:01 98998 505 15:30:00 98998 unclosable 0 0 99179 98928 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates AV2 3 count=2
c 0 2018-09-04 09:48:00 S01 SHORT filled 09:48:01 98997 505 15:30:00 98997 unclosable 0 0 99199 98947 0 0 35041 dated -35041
c 1 2018-09-04 09:51:00 S01 SHORT filled 09:51:01 98994 505 15:30:00 98994 unclosable 0 0 99176 98947 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates AV2 5 count=2
c 0 2018-09-04 09:50:00 S01 SHORT filled 09:50:01 98995 505 15:30:00 98995 unclosable 0 0 99198 98980 0 0 35041 dated -35041
c 1 2018-09-04 09:55:00 S01 SHORT filled 09:55:01 98990 505 15:30:00 98990 unclosable 0 0 99173 98980 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33563 today-precise 24821
candidates AV2 15 count=2
c 0 2018-09-04 10:00:00 S01 SHORT filled 10:00:01 98985 505 15:30:00 98985 unclosable 0 0 99195 99075 0 0 35041 dated -35041
c 1 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99162 99075 0 0 35039 dated -35039
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33561 today-precise 24819
candidates AV2 30 count=2
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99189 99107 0 0 35039 dated -35039
c 1 2018-09-04 10:45:00 S01 SHORT filled 10:45:01 98940 505 15:30:00 98940 unclosable 0 0 99145 99107 0 0 35038 dated -35038
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 2000 360 35039 today-worst 4000 3069 100 12500 1500 1290 1250 225 2000 360 26294 dated-precise 33560 today-precise 24819
candidates AV2 60 count=2
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99689 99142 0 0 35039 dated -35039
c 1 2018-09-04 11:15:00 S01 SHORT not-filled
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 2000 360 35039 today-worst 4000 3069 100 12500 1500 1290 1250 225 2000 360 26294 dated-precise 33560 today-precise 24819
candidates VS 1 count=2
c 0 2018-09-04 09:46:00 S01 SHORT filled 09:46:01 98999 505 15:30:00 98999 unclosable 0 0 99200 98928 0 0 35041 dated -35041
c 1 2018-09-04 09:47:00 S01 SHORT filled 09:47:01 98998 505 15:30:00 98998 unclosable 0 0 99179 98928 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VS 3 count=2
c 0 2018-09-04 09:48:00 S01 SHORT filled 09:48:01 98997 505 15:30:00 98997 unclosable 0 0 99199 98947 0 0 35041 dated -35041
c 1 2018-09-04 09:51:00 S01 SHORT filled 09:51:01 98994 505 15:30:00 98994 unclosable 0 0 99176 98947 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33564 today-precise 24821
candidates VS 5 count=2
c 0 2018-09-04 09:50:00 S01 SHORT filled 09:50:01 98995 505 15:30:00 98995 unclosable 0 0 99198 98980 0 0 35041 dated -35041
c 1 2018-09-04 09:55:00 S01 SHORT filled 09:55:01 98990 505 15:30:00 98990 unclosable 0 0 99173 98980 0 0 35041 dated -35041
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33563 today-precise 24821
candidates VS 15 count=2
c 0 2018-09-04 10:00:00 S01 SHORT filled 10:00:01 98985 505 15:30:00 98985 unclosable 0 0 99195 99075 0 0 35041 dated -35041
c 1 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99162 99075 0 0 35039 dated -35039
f 0 dated-worst 4000 3250 150 12500 10000 1306 1250 225 2000 360 35041 today-worst 4000 3070 100 12500 1500 1291 1250 225 2000 360 26296 dated-precise 33561 today-precise 24819
candidates VS 30 count=2
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99189 99107 0 0 35039 dated -35039
c 1 2018-09-04 10:45:00 S01 SHORT filled 10:45:01 98940 505 15:30:00 98940 unclosable 0 0 99145 99107 0 0 35038 dated -35038
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 2000 360 35039 today-worst 4000 3069 100 12500 1500 1290 1250 225 2000 360 26294 dated-precise 33560 today-precise 24819
candidates VS 60 count=2
c 0 2018-09-04 10:15:00 S01 SHORT filled 10:15:01 98970 505 15:30:00 98970 unclosable 0 0 99689 99142 0 0 35039 dated -35039
c 1 2018-09-04 11:15:00 S01 SHORT not-filled
f 0 dated-worst 4000 3249 150 12500 10000 1305 1250 225 2000 360 35039 today-worst 4000 3069 100 12500 1500 1290 1250 225 2000 360 26294 dated-precise 33560 today-precise 24819
row VO 1 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 1 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 1 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 3 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 3 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 3 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 5 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 5 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 5 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 15 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 15 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 15 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=4 while-open=3 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VO 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 30 SHORT verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VO 30 BOTH verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=3 while-open=2 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VO 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VO 60 SHORT verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VO 60 BOTH verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row AV2 1 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 1 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 1 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 3 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 3 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 3 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 5 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 5 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 5 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 15 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 15 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 15 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row AV2 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 30 SHORT verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row AV2 30 BOTH verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row AV2 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row AV2 60 SHORT verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row AV2 60 BOTH verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VS 1 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 1 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 1 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 3 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 3 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 3 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 5 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 5 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 5 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 15 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 15 SHORT verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 15 BOTH verdict=MISS trades=1 net=-35041 net-bp=-2 dd-bp=1 dd=35041 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35041 pf=0 charges=35041
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35041 trades=1 bp=-2 pass=no gap=50350410000
row VS 30 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 30 SHORT verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VS 30 BOTH verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VS 60 LONG verdict=MISS trades=0 net=0 net-bp=0 dd-bp=0 dd=0 dd-peak=start dd-low=none months=1 months-pass=0 months-positive=0 worst=0 median=0 best=0 floor=0 win-rate=n/a avg-win=n/a avg-loss=n/a pf=n/a charges=0
counts signals=0 while-open=0 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=0 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted -
month 2018-09 capital=200000000 net=0 trades=0 bp=0 pass=no gap=50000000000
row VS 60 SHORT verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
row VS 60 BOTH verdict=MISS trades=1 net=-35039 net-bp=-2 dd-bp=1 dd=35039 dd-peak=start dd-low=2018-09-04 months=1 months-pass=0 months-positive=0 worst=-2 median=-2 best=-2 floor=-2 win-rate=0 avg-win=n/a avg-loss=-35039 pf=0 charges=35039
counts signals=2 while-open=1 cut=0 not-filled=0 zero-qty=0 no-ratio=0 late=0 stop=0 flip=0 time=0 square-off=0 unclosable=1 slots-full=0 fallback-days=0 no-ratio-trail=0 cold=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0 taint=ST:0,BB:0,RSI:0,WR:0,CCI:0,EMA:0
admitted 0
month 2018-09 capital=200000000 net=-35039 trades=1 bp=-2 pass=no gap=50350390000
rank 1 VO 1 LONG
rank 2 VO 3 LONG
rank 3 VO 5 LONG
rank 4 VO 15 LONG
rank 5 VO 30 LONG
rank 6 VO 60 LONG
rank 7 AV2 1 LONG
rank 8 AV2 3 LONG
rank 9 AV2 5 LONG
rank 10 AV2 15 LONG
rank 11 AV2 30 LONG
rank 12 AV2 60 LONG
rank 13 VS 1 LONG
rank 14 VS 3 LONG
rank 15 VS 5 LONG
rank 16 VS 15 LONG
rank 17 VS 30 LONG
rank 18 VS 60 LONG
rank 19 VO 30 SHORT
rank 20 VO 30 BOTH
rank 21 VO 60 SHORT
rank 22 VO 60 BOTH
rank 23 AV2 30 SHORT
rank 24 AV2 30 BOTH
rank 25 AV2 60 SHORT
rank 26 AV2 60 BOTH
rank 27 VS 30 SHORT
rank 28 VS 30 BOTH
rank 29 VS 60 SHORT
rank 30 VS 60 BOTH
rank 31 VO 1 SHORT
rank 32 VO 1 BOTH
rank 33 VO 3 SHORT
rank 34 VO 3 BOTH
rank 35 VO 5 SHORT
rank 36 VO 5 BOTH
rank 37 VO 15 SHORT
rank 38 VO 15 BOTH
rank 39 AV2 1 SHORT
rank 40 AV2 1 BOTH
rank 41 AV2 3 SHORT
rank 42 AV2 3 BOTH
rank 43 AV2 5 SHORT
rank 44 AV2 5 BOTH
rank 45 AV2 15 SHORT
rank 46 AV2 15 BOTH
rank 47 VS 1 SHORT
rank 48 VS 1 BOTH
rank 49 VS 3 SHORT
rank 50 VS 3 BOTH
rank 51 VS 5 SHORT
rank 52 VS 5 BOTH
rank 53 VS 15 SHORT
rank 54 VS 15 BOTH
head 1 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 1 SHORT AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 SHORT VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 SHORT VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 1 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 3 SHORT AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 SHORT VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 SHORT VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 3 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 5 SHORT AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 SHORT VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 SHORT VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 5 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 15 SHORT AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 SHORT VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 SHORT VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 BOTH AV2/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 BOTH VS/VO trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 15 BOTH VS/AV2 trades=1000 net=base:-35041 dd-bp=1000 dd=1000
head 30 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 30 SHORT AV2/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 30 SHORT VS/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 30 SHORT VS/AV2 trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 30 BOTH AV2/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 30 BOTH VS/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 30 BOTH VS/AV2 trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 60 LONG AV2/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG VS/VO trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 LONG VS/AV2 trades=base:0 net=base:0 dd-bp=base:0 dd=base:0
head 60 SHORT AV2/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 60 SHORT VS/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 60 SHORT VS/AV2 trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 60 BOTH AV2/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 60 BOTH VS/VO trades=1000 net=base:-35039 dd-bp=1000 dd=1000
head 60 BOTH VS/AV2 trades=1000 net=base:-35039 dd-bp=1000 dd=1000
nearest-met-floor 0 VO 1 LONG
```
