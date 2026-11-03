# universe.json (copied as Markdown: the repository tracks .json only under .claude/)

```json
{
 "source": "Current list = every underlying with an option file in GDFL's 29 Oct 2026 F&O day zip (gfdl-download-20261029-d29/FNO/OCT_2026/GFDLNFO_TICK_29102026.zip, sha256 28599e78..., in the source folder of the 29 Oct store import per rehearsal-d29.md and attack-d29-data.md), less the 6 index underlyings of /aliases.json; it equals, name for name, the 6 Oct 2026 Zerodha instrument master's NFO futures underlyings (runtime/masters/zerodha_instruments.csv, sha256 cd24bb5e...) after the dated changes of 6-29 Oct (BAJAJHLDNG left after the 27 Oct expiry; ICICIPRULI renamed ICICILIFE 23 Oct; COROMANDEL, CUB, DATAPATTNS, KARURVYSYA, NAVINFLUOR, SYRMA first traded 28 Oct). ISIN and listing date: the stored NSE session master runtime/store/session-masters/NSE_CM_security_29102026.csv.gz (sha256 4853f9c0..., EQ row; ListgDt is seconds from 1980-01-01). Earlier names: the 27 applied rows of /aliases.json symbol_changes, plus the two held-back rows of the charter D-7311 table (NSE symbolchange.csv read 24 Oct 2026): TATAGLOBAL->TATACONSUM 2020-02-27 and ICICIPRULI->ICICILIFE 2026-10-23 (the latter also measured in GDFL file names and NSE session masters). Starts: Zerodha max(2018-01-01, ListgDt), GDFL max(2018-09-03, ListgDt). Report: universe.md; detail: universe-evidence/universe-detail.json.",
 "count": 218,
 "report_path": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fno-data-audit-20261030/universe.md",
 "stocks": [
  {
   "symbol": "360ONE",
   "isin": "INE466L01038",
   "start_zerodha": "2019-09-19",
   "start_gdfl": "2019-09-19",
   "history_names": [
    "IIFLWAM (until 2023-01-22; 360ONE from 2023-01-23)"
   ],
   "note": "listed 2019-09-19, inside the window | store: GDFL 1s key 360ONE starts 2019-10, after the start 2019-09-19"
  },
  {
   "symbol": "ABB",
   "isin": "INE117A01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ABCAPITAL",
   "isin": "INE674K01013",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ADANIENSOL",
   "isin": "INE931S01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "ADANITRANS (until 2023-08-23; ADANIENSOL from 2023-08-24)"
   ],
   "note": ""
  },
  {
   "symbol": "ADANIENT",
   "isin": "INE423A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ADANIGREEN",
   "isin": "INE364U01010",
   "start_zerodha": "2018-06-18",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "listed 2018-06-18, inside the window"
  },
  {
   "symbol": "ADANIPORTS",
   "isin": "INE742F01042",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ADANIPOWER",
   "isin": "INE814H01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ALKEM",
   "isin": "INE540L01014",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "AMBER",
   "isin": "INE371P01015",
   "start_zerodha": "2018-01-30",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "listed 2018-01-30, inside the window"
  },
  {
   "symbol": "AMBUJACEM",
   "isin": "INE079A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ANANDRATHI",
   "isin": "INE463V01026",
   "start_zerodha": "2021-12-14",
   "start_gdfl": "2021-12-14",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | listed 2021-12-14, inside the window | store: no Zerodha cash key ANANDRATHI | store: GDFL 1s key ANANDRATHI starts 2026-10, after the start 2021-12-14"
  },
  {
   "symbol": "ANGELONE",
   "isin": "INE732I01021",
   "start_zerodha": "2020-10-05",
   "start_gdfl": "2020-10-05",
   "history_names": [
    "ANGELBRKG (until 2021-11-10; ANGELONE from 2021-11-11)"
   ],
   "note": "listed 2020-10-05, inside the window | GDFL key ANGELONE starts 2021-11, not before the ANGELBRKG -> ANGELONE rename (2021-11-11), and no ANGELBRKG GDFL key exists: GDFL history from 2020-10-05 until the 2021-11 month file appears to sit under NO key"
  },
  {
   "symbol": "APLAPOLLO",
   "isin": "INE702C01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "APOLLOHOSP",
   "isin": "INE437A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ASHOKLEY",
   "isin": "INE208A01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ASIANPAINT",
   "isin": "INE021A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ASTRAL",
   "isin": "INE006I01046",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ATHERENERG",
   "isin": "INE0LEZ01016",
   "start_zerodha": "2025-05-06",
   "start_gdfl": "2025-05-06",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | listed 2025-05-06, inside the window | store: no Zerodha cash key ATHERENERG"
  },
  {
   "symbol": "AUBANK",
   "isin": "INE949L01017",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "AUROPHARMA",
   "isin": "INE406A01037",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "AXISBANK",
   "isin": "INE238A01034",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BAJAJ-AUTO",
   "isin": "INE917I01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BAJAJFINSV",
   "isin": "INE918I01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BAJFINANCE",
   "isin": "INE296A01032",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BANDHANBNK",
   "isin": "INE545U01014",
   "start_zerodha": "2018-03-27",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "listed 2018-03-27, inside the window"
  },
  {
   "symbol": "BANKBARODA",
   "isin": "INE028A01039",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BANKINDIA",
   "isin": "INE084A01016",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BDL",
   "isin": "INE171Z01026",
   "start_zerodha": "2018-03-23",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "listed 2018-03-23, inside the window"
  },
  {
   "symbol": "BEL",
   "isin": "INE263A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BHARATFORG",
   "isin": "INE465A01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BHARTIARTL",
   "isin": "INE397D01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BHEL",
   "isin": "INE257A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BIOCON",
   "isin": "INE376G01013",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BLUESTARCO",
   "isin": "INE472A01039",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BOSCHLTD",
   "isin": "INE323A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BPCL",
   "isin": "INE029A01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BRITANNIA",
   "isin": "INE216A01030",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "BSE",
   "isin": "INE118H01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CAMS",
   "isin": "INE596I01020",
   "start_zerodha": "2020-10-05",
   "start_gdfl": "2020-10-05",
   "history_names": [],
   "note": "listed 2020-10-05, inside the window"
  },
  {
   "symbol": "CANBK",
   "isin": "INE476A01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CDSL",
   "isin": "INE736A01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CGPOWER",
   "isin": "INE067A01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CHOLAFIN",
   "isin": "INE121A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CIPLA",
   "isin": "INE059A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "COALINDIA",
   "isin": "INE522F01014",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "COCHINSHIP",
   "isin": "INE704P01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "COFORGE",
   "isin": "INE591G01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "NIITTECH (until 2020-08-19; COFORGE from 2020-08-20)"
   ],
   "note": ""
  },
  {
   "symbol": "COLPAL",
   "isin": "INE259A01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CONCOR",
   "isin": "INE111A01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "COROMANDEL",
   "isin": "INE169A01031",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-28 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | store: no Zerodha cash key COROMANDEL"
  },
  {
   "symbol": "CROMPTON",
   "isin": "INE299U01018",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "CUB",
   "isin": "INE491A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-28 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | store: no Zerodha cash key CUB"
  },
  {
   "symbol": "CUMMINSIND",
   "isin": "INE298A01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "DABUR",
   "isin": "INE016A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "DATAPATTNS",
   "isin": "INE0IX101010",
   "start_zerodha": "2021-12-24",
   "start_gdfl": "2021-12-24",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-28 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | listed 2021-12-24, inside the window | store: no Zerodha cash key DATAPATTNS | store: GDFL 1s key DATAPATTNS starts 2026-10, after the start 2021-12-24"
  },
  {
   "symbol": "DELHIVERY",
   "isin": "INE148O01028",
   "start_zerodha": "2022-05-24",
   "start_gdfl": "2022-05-24",
   "history_names": [],
   "note": "listed 2022-05-24, inside the window"
  },
  {
   "symbol": "DIVISLAB",
   "isin": "INE361B01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "DIXON",
   "isin": "INE935N01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "DLF",
   "isin": "INE271C01023",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "DMART",
   "isin": "INE192R01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "DRREDDY",
   "isin": "INE089A01031",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "EICHERMOT",
   "isin": "INE066A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ENRIN",
   "isin": "INE1NPP01017",
   "start_zerodha": "2025-06-19",
   "start_gdfl": "2025-06-19",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | Siemens Energy India, demerged from SIEMENS (background); history starts at its own listing. | listed 2025-06-19, inside the window | store: no Zerodha cash key ENRIN | store: GDFL 1s key ENRIN starts 2026-10, after the start 2025-06-19"
  },
  {
   "symbol": "ETERNAL",
   "isin": "INE758T01015",
   "start_zerodha": "2021-07-23",
   "start_gdfl": "2021-07-23",
   "history_names": [
    "ZOMATO (until 2025-04-08; ETERNAL from 2025-04-09)"
   ],
   "note": "listed 2021-07-23, inside the window"
  },
  {
   "symbol": "FEDERALBNK",
   "isin": "INE171A01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "FORCEMOT",
   "isin": "INE451A01017",
   "start_zerodha": "2019-08-19",
   "start_gdfl": "2019-08-19",
   "history_names": [
    "BAJAJTEMPO (until 2019-08-18; FORCEMOT from 2019-08-19)"
   ],
   "note": "NSE symbolchange row BAJAJTEMPO -> FORCEMOT is dated 2019-08-19, the SAME day as FORCEMOT's NSE ListgDt; both feeds' FORCEMOT keys start 2019-08 and neither holds a BAJAJTEMPO key: no NSE trading as BAJAJTEMPO is expected inside the window (UNVERIFIED beyond these two facts). | listed 2019-08-19, inside the window | store: Zerodha 1min key FORCEMOT ends 2026-09"
  },
  {
   "symbol": "FORTIS",
   "isin": "INE061F01013",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GAIL",
   "isin": "INE129A01019",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GLENMARK",
   "isin": "INE935A01035",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GMRAIRPORT",
   "isin": "INE776C01039",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "GMRINFRA (until 2024-12-10; GMRAIRPORT from 2024-12-11)"
   ],
   "note": "GMRINFRA -> GMRAIRPORT (2024-12-11); GMR Infra demerged its power/urban business in 2022 (price basis change, same key; background, UNVERIFIED in the store)."
  },
  {
   "symbol": "GODFRYPHLP",
   "isin": "INE260B01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GODREJCP",
   "isin": "INE102D01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GODREJPROP",
   "isin": "INE484J01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GRASIM",
   "isin": "INE047A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "GVT&D",
   "isin": "INE200A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "GET&D (until 2024-11-04; GVT&D from 2024-11-05)"
   ],
   "note": "GDFL key GVT&D starts 2025-01, not before the GET&D -> GVT&D rename (2024-11-05), and no GET&D GDFL key exists: GDFL history from 2018-09-03 until the 2025-01 month file appears to sit under NO key"
  },
  {
   "symbol": "HAL",
   "isin": "INE066F01020",
   "start_zerodha": "2018-03-28",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "listed 2018-03-28, inside the window"
  },
  {
   "symbol": "HAVELLS",
   "isin": "INE176B01034",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HCLTECH",
   "isin": "INE860A01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HDFCAMC",
   "isin": "INE127D01025",
   "start_zerodha": "2018-08-06",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "listed 2018-08-06, inside the window"
  },
  {
   "symbol": "HDFCBANK",
   "isin": "INE040A01034",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "HDFC Ltd merged into HDFC Bank in July 2023; HDFC keeps its own key (not a rename, /aliases.json not_aliased; merger itself is background)."
  },
  {
   "symbol": "HDFCLIFE",
   "isin": "INE795G01014",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HEROMOTOCO",
   "isin": "INE158A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HINDALCO",
   "isin": "INE038A01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HINDPETRO",
   "isin": "INE094A01015",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HINDUNILVR",
   "isin": "INE030A01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HINDZINC",
   "isin": "INE267A01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "HYUNDAI",
   "isin": "INE0V6F01027",
   "start_zerodha": "2024-10-22",
   "start_gdfl": "2024-10-22",
   "history_names": [],
   "note": "listed 2024-10-22, inside the window"
  },
  {
   "symbol": "ICICIBANK",
   "isin": "INE090A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ICICIGI",
   "isin": "INE765G01017",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ICICILIFE",
   "isin": "INE726G01019",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "ICICIPRULI (until 2026-10-22; ICICILIFE from 2026-10-23) [rename held back: the store keeps ICICIPRULI as its own key]"
   ],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-23 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | ICICIPRULI -> ICICILIFE 2026-10-23 is HELD BACK (charter D-7311 table, Applied: no; L-R422-06): the store keys the whole cash history as ICICIPRULI in both feeds and has NO ICICILIFE cash key (verify-d29.md: unknown_series NSE-CASH-ICICILIFE); GDFL ICICIPRULI cash holds 2026-10 (rename day 22 Oct refused, ICICILIFE 23 Oct refused / 26-28 Oct held back per gfdl-oct-import live.md). | store: no Zerodha cash key ICICILIFE | store: no GDFL cash key ICICILIFE | store: Zerodha 1min key ICICIPRULI ends 2026-09"
  },
  {
   "symbol": "IDEA",
   "isin": "INE669E01016",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "IDFCFIRSTB",
   "isin": "INE092T01019",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "IDFCBANK (until 2019-01-15; IDFCFIRSTB from 2019-01-16)"
   ],
   "note": "IDFCBANK -> IDFCFIRSTB (2019-01-16) after the Capital First merger; IDFC Ltd merged into it in 2024 (corporate action, same key; background, UNVERIFIED in the store)."
  },
  {
   "symbol": "IEX",
   "isin": "INE022Q01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "6 Oct Zerodha master lists futures only to 2026-11-23 (no Dec 2026 contract): likely leaving F&O after that expiry"
  },
  {
   "symbol": "INDHOTEL",
   "isin": "INE053A01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "INDIANB",
   "isin": "INE562A01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "INDIGO",
   "isin": "INE646L01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "INDUSINDBK",
   "isin": "INE095A01012",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "INDUSTOWER",
   "isin": "INE121J01017",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "INFRATEL (until 2020-12-17; INDUSTOWER from 2020-12-18)"
   ],
   "note": "INFRATEL -> INDUSTOWER (2020-12-18): Bharti Infratel absorbed Indus Towers and took its name (background, UNVERIFIED in the store)."
  },
  {
   "symbol": "INFY",
   "isin": "INE009A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "INOXWIND",
   "isin": "INE066P01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "IOC",
   "isin": "INE242A01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "IREDA",
   "isin": "INE202E01016",
   "start_zerodha": "2023-11-29",
   "start_gdfl": "2023-11-29",
   "history_names": [],
   "note": "6 Oct Zerodha master lists futures only to 2026-11-23 (no Dec 2026 contract): likely leaving F&O after that expiry | listed 2023-11-29, inside the window"
  },
  {
   "symbol": "IRFC",
   "isin": "INE053F01010",
   "start_zerodha": "2021-01-29",
   "start_gdfl": "2021-01-29",
   "history_names": [],
   "note": "listed 2021-01-29, inside the window"
  },
  {
   "symbol": "ITC",
   "isin": "INE154A01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "ITC Hotels demerger in 2025 (price basis change, same key; background)."
  },
  {
   "symbol": "JINDALSTEL",
   "isin": "INE749A01030",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "JIOFIN",
   "isin": "INE758E01017",
   "start_zerodha": "2023-08-21",
   "start_gdfl": "2023-08-21",
   "history_names": [],
   "note": "Demerged from RELIANCE (background); listed 2023-08-21 per ListgDt. | listed 2023-08-21, inside the window | store: GDFL 1s key JIOFIN starts 2023-09, after the start 2023-08-21"
  },
  {
   "symbol": "JSWENERGY",
   "isin": "INE121E01018",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "JSWSTEEL",
   "isin": "INE019A01038",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "JUBLFOOD",
   "isin": "INE797F01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "KALYANKJIL",
   "isin": "INE303R01014",
   "start_zerodha": "2021-03-26",
   "start_gdfl": "2021-03-26",
   "history_names": [],
   "note": "listed 2021-03-26, inside the window"
  },
  {
   "symbol": "KARURVYSYA",
   "isin": "INE036D01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-28 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | store: no Zerodha cash key KARURVYSYA | store: GDFL 1s key KARURVYSYA starts 2026-10, after the start 2018-09-03"
  },
  {
   "symbol": "KAYNES",
   "isin": "INE918Z01012",
   "start_zerodha": "2022-11-22",
   "start_gdfl": "2022-11-22",
   "history_names": [],
   "note": "listed 2022-11-22, inside the window"
  },
  {
   "symbol": "KEI",
   "isin": "INE878B01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "KFINTECH",
   "isin": "INE138Y01010",
   "start_zerodha": "2022-12-29",
   "start_gdfl": "2022-12-29",
   "history_names": [],
   "note": "listed 2022-12-29, inside the window"
  },
  {
   "symbol": "KOTAKBANK",
   "isin": "INE237A01036",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "KPITTECH",
   "isin": "INE04I401011",
   "start_zerodha": "2019-04-22",
   "start_gdfl": "2019-04-22",
   "history_names": [],
   "note": "New KPIT Technologies listing after the 2019 Birlasoft demerger; the old KPIT symbol became BSOFT and is a different series (D-3354 item 3; GDFL holds a separate KPIT cash key). | listed 2019-04-22, inside the window | store: GDFL 1s key KPITTECH starts 2019-05, after the start 2019-04-22"
  },
  {
   "symbol": "LAURUSLABS",
   "isin": "INE947Q01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "LICHSGFIN",
   "isin": "INE115A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "LICI",
   "isin": "INE0J1Y01017",
   "start_zerodha": "2022-05-17",
   "start_gdfl": "2022-05-17",
   "history_names": [],
   "note": "listed 2022-05-17, inside the window"
  },
  {
   "symbol": "LODHA",
   "isin": "INE670K01029",
   "start_zerodha": "2021-04-19",
   "start_gdfl": "2021-04-19",
   "history_names": [],
   "note": "listed 2021-04-19, inside the window"
  },
  {
   "symbol": "LT",
   "isin": "INE018A01030",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "LTF",
   "isin": "INE498L01015",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "L&TFH (until 2024-04-22; LTF from 2024-04-23)"
   ],
   "note": ""
  },
  {
   "symbol": "LTM",
   "isin": "INE214T01019",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "LTI (until 2022-12-04; LTIM from 2022-12-05)",
    "LTIM (until 2026-02-26; LTM from 2026-02-27)"
   ],
   "note": "Two renames: LTI -> LTIM (2022-12-05, the LTI-Mindtree merger) -> LTM (2026-02-27). MINDTREE's own pre-merger history is a different company and keeps its own key (/aliases.json not_aliased)."
  },
  {
   "symbol": "LUPIN",
   "isin": "INE326A01037",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "M&M",
   "isin": "INE101A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MAHABANK",
   "isin": "INE457A01014",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | store: no Zerodha cash key MAHABANK"
  },
  {
   "symbol": "MANAPPURAM",
   "isin": "INE522D01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MANKIND",
   "isin": "INE634S01028",
   "start_zerodha": "2023-05-09",
   "start_gdfl": "2023-05-09",
   "history_names": [],
   "note": "listed 2023-05-09, inside the window"
  },
  {
   "symbol": "MARICO",
   "isin": "INE196A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MARUTI",
   "isin": "INE585B01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MAXHEALTH",
   "isin": "INE027H01010",
   "start_zerodha": "2020-08-21",
   "start_gdfl": "2020-08-21",
   "history_names": [],
   "note": "Max Healthcare listed after the Max India / Radiant scheme (background); date per ListgDt. | listed 2020-08-21, inside the window | store: GDFL 1s key MAXHEALTH starts 2020-09, after the start 2020-08-21"
  },
  {
   "symbol": "MAZDOCK",
   "isin": "INE249Z01020",
   "start_zerodha": "2020-10-12",
   "start_gdfl": "2020-10-12",
   "history_names": [],
   "note": "listed 2020-10-12, inside the window | store: Zerodha 1min key MAZDOCK ends 2026-09"
  },
  {
   "symbol": "MCX",
   "isin": "INE745G01043",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MFSL",
   "isin": "INE180A01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MOTHERSON",
   "isin": "INE775A01035",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "MOTHERSUMI (until 2022-06-08; MOTHERSON from 2022-06-09)"
   ],
   "note": "MOTHERSUMI -> MOTHERSON (2022-06-09), with the 2022 demerger/merger reorganisation (price basis change, same key; background, UNVERIFIED in the store)."
  },
  {
   "symbol": "MOTILALOFS",
   "isin": "INE338I01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MPHASIS",
   "isin": "INE356A01018",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "MUTHOOTFIN",
   "isin": "INE414G01012",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NAM-INDIA",
   "isin": "INE298J01013",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "RNAM (until 2020-01-22; NAM-INDIA from 2020-01-23)"
   ],
   "note": "GDFL key NAM-INDIA starts 2020-01, not before the RNAM -> NAM-INDIA rename (2020-01-23), and no RNAM GDFL key exists: GDFL history from 2018-09-03 until the 2020-01 month file appears to sit under NO key"
  },
  {
   "symbol": "NATIONALUM",
   "isin": "INE139A01034",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NAUKRI",
   "isin": "INE663F01032",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NAVINFLUOR",
   "isin": "INE048G01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-28 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | store: no Zerodha cash key NAVINFLUOR"
  },
  {
   "symbol": "NBCC",
   "isin": "INE095N01031",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NESTLEIND",
   "isin": "INE239A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NHPC",
   "isin": "INE848E01016",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NMDC",
   "isin": "INE584A01023",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NTPC",
   "isin": "INE733E01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "NYKAA",
   "isin": "INE388Y01029",
   "start_zerodha": "2021-11-10",
   "start_gdfl": "2021-11-10",
   "history_names": [],
   "note": "listed 2021-11-10, inside the window"
  },
  {
   "symbol": "OBEROIRLTY",
   "isin": "INE093I01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "OFSS",
   "isin": "INE881D01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "OIL",
   "isin": "INE274J01014",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ONGC",
   "isin": "INE213A01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PAGEIND",
   "isin": "INE761H01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PATANJALI",
   "isin": "INE619A01035",
   "start_zerodha": "2020-01-24",
   "start_gdfl": "2020-01-24",
   "history_names": [
    "RUCHI (until 2022-07-12; PATANJALI from 2022-07-13)"
   ],
   "note": "RUCHI -> PATANJALI 2022-07-13 (NSE circular NSE/CML/52890, charter). ListgDt of ISIN INE619A01035 is 2020-01-24 (the post-insolvency relisting, background): Ruchi Soya trading before that date is not this ISIN's history and is outside its start date; Zerodha PATANJALI starts 2020-01, GDFL PATANJALI only 2023-11, no RUCHI key in either feed. | listed 2020-01-24, inside the window | GDFL key PATANJALI starts 2023-11, not before the RUCHI -> PATANJALI rename (2022-07-13), and no RUCHI GDFL key exists: GDFL history from 2020-01-24 until the 2023-11 month file appears to sit under NO key"
  },
  {
   "symbol": "PAYTM",
   "isin": "INE982J01020",
   "start_zerodha": "2021-11-18",
   "start_gdfl": "2021-11-18",
   "history_names": [],
   "note": "listed 2021-11-18, inside the window"
  },
  {
   "symbol": "PERSISTENT",
   "isin": "INE262H01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PETRONET",
   "isin": "INE347G01014",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PFC",
   "isin": "INE134E01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PGEL",
   "isin": "INE457L01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PHOENIXLTD",
   "isin": "INE211B01039",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PIDILITIND",
   "isin": "INE318A01026",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PIIND",
   "isin": "INE603J01030",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PNB",
   "isin": "INE160A01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "PNBHOUSING",
   "isin": "INE572E01012",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "POLICYBZR",
   "isin": "INE417T01026",
   "start_zerodha": "2021-11-15",
   "start_gdfl": "2021-11-15",
   "history_names": [],
   "note": "listed 2021-11-15, inside the window"
  },
  {
   "symbol": "POLYCAB",
   "isin": "INE455K01017",
   "start_zerodha": "2019-04-16",
   "start_gdfl": "2019-04-16",
   "history_names": [],
   "note": "listed 2019-04-16, inside the window"
  },
  {
   "symbol": "POWERGRID",
   "isin": "INE752E01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "POWERINDIA",
   "isin": "INE07Y701011",
   "start_zerodha": "2020-03-30",
   "start_gdfl": "2020-03-30",
   "history_names": [],
   "note": "listed 2020-03-30, inside the window | store: GDFL 1s key POWERINDIA starts 2020-04, after the start 2020-03-30"
  },
  {
   "symbol": "PREMIERENE",
   "isin": "INE0BS701011",
   "start_zerodha": "2024-09-03",
   "start_gdfl": "2024-09-03",
   "history_names": [],
   "note": "listed 2024-09-03, inside the window"
  },
  {
   "symbol": "PRESTIGE",
   "isin": "INE811K01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "RADICO",
   "isin": "INE944F01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "RBLBANK",
   "isin": "INE976G01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "RECLTD",
   "isin": "INE020B01018",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "RELIANCE",
   "isin": "INE002A01018",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "Jio Financial demerger in 2023 (price basis change, same key; background)."
  },
  {
   "symbol": "RVNL",
   "isin": "INE415G01027",
   "start_zerodha": "2019-04-11",
   "start_gdfl": "2019-04-11",
   "history_names": [],
   "note": "listed 2019-04-11, inside the window"
  },
  {
   "symbol": "SAGILITY",
   "isin": "INE0W2G01015",
   "start_zerodha": "2024-11-12",
   "start_gdfl": "2024-11-12",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | listed 2024-11-12, inside the window | store: no Zerodha cash key SAGILITY"
  },
  {
   "symbol": "SAIL",
   "isin": "INE114A01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "GDFL options first traded under this name on 2026-10-09 in October"
  },
  {
   "symbol": "SBICARD",
   "isin": "INE018E01016",
   "start_zerodha": "2020-03-16",
   "start_gdfl": "2020-03-16",
   "history_names": [],
   "note": "listed 2020-03-16, inside the window"
  },
  {
   "symbol": "SBILIFE",
   "isin": "INE123W01016",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SBIN",
   "isin": "INE062A01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SHREECEM",
   "isin": "INE070A01015",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SHRIRAMFIN",
   "isin": "INE721A01047",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "SRTRANSFIN (until 2022-12-19; SHRIRAMFIN from 2022-12-20)"
   ],
   "note": "SRTRANSFIN -> SHRIRAMFIN (2022-12-20) came with the Shriram group merger (corporate action; background, UNVERIFIED in the store)."
  },
  {
   "symbol": "SIEMENS",
   "isin": "INE003A01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": "Siemens Energy India (ENRIN) demerged in 2025 (price basis change, same key; background)."
  },
  {
   "symbol": "SOLARINDS",
   "isin": "INE343H01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SONACOMS",
   "isin": "INE073K01018",
   "start_zerodha": "2021-06-24",
   "start_gdfl": "2021-06-24",
   "history_names": [],
   "note": "listed 2021-06-24, inside the window"
  },
  {
   "symbol": "SRF",
   "isin": "INE647A01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SUNPHARMA",
   "isin": "INE044A01036",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SUPREMEIND",
   "isin": "INE195A01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SUZLON",
   "isin": "INE040H01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "SWIGGY",
   "isin": "INE00H001014",
   "start_zerodha": "2024-11-13",
   "start_gdfl": "2024-11-13",
   "history_names": [],
   "note": "listed 2024-11-13, inside the window"
  },
  {
   "symbol": "SYRMA",
   "isin": "INE0DYJ01015",
   "start_zerodha": "2022-08-26",
   "start_gdfl": "2022-08-26",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | GDFL options first traded under this name on 2026-10-28 in October | not an NFO futures underlying in the 6 Oct Zerodha master (joined F&O or renamed after 6 Oct) | listed 2022-08-26, inside the window | store: no Zerodha cash key SYRMA | store: GDFL 1s key SYRMA starts 2026-10, after the start 2022-08-26"
  },
  {
   "symbol": "TATACONSUM",
   "isin": "INE192A01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "TATAGLOBAL (until 2020-02-26; TATACONSUM from 2020-02-27) [rename held back: the store keeps TATAGLOBAL as its own key]"
   ],
   "note": "TATAGLOBAL -> TATACONSUM 2020-02-27 is HELD BACK (charter D-7311 table, Applied: no): GDFL keeps TATAGLOBAL as its own cash key (2018-09..2020-02) and TATACONSUM from 2020-02; Zerodha keys the whole span as TATACONSUM. | store: GDFL 1s key TATACONSUM starts 2020-02, after the start 2018-09-03 (earlier months under the held-back old key)"
  },
  {
   "symbol": "TATAELXSI",
   "isin": "INE670A01012",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TATAPOWER",
   "isin": "INE245A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TATASTEEL",
   "isin": "INE081A01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TCS",
   "isin": "INE467B01029",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TECHM",
   "isin": "INE669C01036",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TIINDIA",
   "isin": "INE974X01010",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TITAN",
   "isin": "INE280A01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TMPV",
   "isin": "INE155A01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "TATAMOTORS (until 2025-10-23; TMPV from 2025-10-24)"
   ],
   "note": "Tata Motors demerger: TATAMOTORS renamed TMPV (passenger vehicles) eff. 2025-10-24 per /aliases.json; the commercial-vehicle business is a separate listing, TMCV (ISIN INE1TAE01010 in the 29 Oct NSE master), NOT an F&O underlying on 29 Oct (absent from GDFL 29 Oct options and from the 6 Oct Zerodha futures). TMPV prices after the demerger ex-date are on a new basis (corporate action, not a rename)."
  },
  {
   "symbol": "TORNTPHARM",
   "isin": "INE685A01028",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TRENT",
   "isin": "INE849A01020",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "TVSMOTOR",
   "isin": "INE494B01023",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "UJJIVANSFB",
   "isin": "INE551W01018",
   "start_zerodha": "2019-12-12",
   "start_gdfl": "2019-12-12",
   "history_names": [],
   "note": "NOT in the live build's F&O list (core::universe::FNO_UNDERLYINGS, 213 names measured 2026-08-01): /universes.json target fno does not cover it | listed 2019-12-12, inside the window | store: no Zerodha cash key UJJIVANSFB | store: GDFL 1s key UJJIVANSFB starts 2026-10, after the start 2019-12-12"
  },
  {
   "symbol": "ULTRACEMCO",
   "isin": "INE481G01011",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "UNIONBANK",
   "isin": "INE692A01016",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "UNITDSPR",
   "isin": "INE854D01024",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "MCDOWELL-N (until 2024-06-06; UNITDSPR from 2024-06-07)"
   ],
   "note": ""
  },
  {
   "symbol": "UNOMINDA",
   "isin": "INE405E01023",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "MINDAIND (until 2022-08-04; UNOMINDA from 2022-08-05)"
   ],
   "note": "GDFL key UNOMINDA starts 2022-08, not before the MINDAIND -> UNOMINDA rename (2022-08-05), and no MINDAIND GDFL key exists: GDFL history from 2018-09-03 until the 2022-08 month file appears to sit under NO key"
  },
  {
   "symbol": "UPL",
   "isin": "INE628A01036",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "VBL",
   "isin": "INE200M01039",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "VEDL",
   "isin": "INE205A01025",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "VMM",
   "isin": "INE01EA01019",
   "start_zerodha": "2024-12-18",
   "start_gdfl": "2024-12-18",
   "history_names": [],
   "note": "listed 2024-12-18, inside the window"
  },
  {
   "symbol": "VOLTAS",
   "isin": "INE226A01021",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "WAAREEENER",
   "isin": "INE377N01017",
   "start_zerodha": "2024-10-28",
   "start_gdfl": "2024-10-28",
   "history_names": [],
   "note": "listed 2024-10-28, inside the window"
  },
  {
   "symbol": "WIPRO",
   "isin": "INE075A01022",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "YESBANK",
   "isin": "INE528G01035",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [],
   "note": ""
  },
  {
   "symbol": "ZYDUSLIFE",
   "isin": "INE010B01027",
   "start_zerodha": "2018-01-01",
   "start_gdfl": "2018-09-03",
   "history_names": [
    "CADILAHC (until 2022-03-06; ZYDUSLIFE from 2022-03-07)"
   ],
   "note": ""
  }
 ]
}

```
