//! The instrument universes, as transcribed data.
//!
//! # Why these are Rust literals
//!
//! `CLAUDE.md` §2 permits no `.csv` in this repository, so a constituent list
//! cannot be tracked as a data file. The *data* is fine; the *format* is not.
//! These are transcriptions, and each names where it came from.
//!
//! # Why membership is not a field of [`crate::instrument::InstrumentKey`]
//!
//! `InstrumentKey` derives `Hash` and `Eq` over every field and is used as a
//! `HashMap` key — that is the deduplication mechanism. If membership were a
//! field, the *same contract* carrying different memberships would become two
//! different keys and dedup would break silently. Membership is therefore a
//! property looked up **from** the key, never part of it.
//!
//! # Cost
//!
//! Lookup is one hash, one mask and a bounded probe of [`MemberIndex`] — a
//! table this file builds at compile time. The worst probe is asserted at
//! `<= 8` by `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1`,
//! which walks each table the way `contains` does and counts the steps, and
//! measured, on the six tables this file now builds, at:
//!
//! | table | members | slots | worst probe |
//! |---|---|---|---|
//! | `NTM_INDEX` | 750 | 2048 | 6 |
//! | `FNO_INDEX` | 213 | 1024 | 7 |
//! | `NIFTY_500_INDEX` | 500 | 2048 | 5 |
//! | `NIFTY_200_INDEX` | 200 | 1024 | 6 |
//! | `NIFTY_100_INDEX` | 100 | 512 | 5 |
//! | `NIFTY_50_INDEX` | 50 | 256 | 3 |
//!
//! [`of_equity`] probes all six, so a membership question costs at most 32
//! probe steps rather than 13. That is three times what it was and it is still
//! a CONSTANT — none of those numbers moves when a list grows within its
//! table, which is the property `CLAUDE.md` §3 rule 4 asks for. The four
//! appended tables came in at D-0089; sizing them the way [`MemberIndex`]
//! merely *permits* rather than the way its bound was *measured* put the NIFTY
//! 500 at 13 steps, and [`NIFTY_500_INDEX`] records why they are a quarter
//! full instead of half.
//!
//! **This paragraph said "a binary search over a sorted array — O(log n) on 750
//! entries, so at most 10 comparisons ... a perfect hash would make it O(1) and
//! is not worth the machinery at this size."** The machinery was built, in this
//! file, and neither this sentence nor `docs/06-limits.md` §11 was walked back
//! for it — so the crate documented a departure from `CLAUDE.md` golden rule 4
//! that no longer existed. Corrected by D-0065, which removed the workspace's
//! last `binary_search` next door in [`crate::vendor`] and found this on the
//! way. Neither list is on the sweep's per-bar path in any case — membership is
//! asked once per instrument at merge time — but that is now a fact about
//! where it is called, not an excuse for what it costs.
//!
//! # What consults this
//!
//! `api::server::universe` stamps every merged cash listing with its
//! membership, and the page and the report both show it. That is not
//! decoration: [`crate::vendor::Skip::SmeBoard`] declines 1,117 real shares on
//! the ground that neither list contains an SME listing, and a claim that
//! load-bearing is checked here rather than asserted in a comment — see
//! [`of_equity`] and `no_measured_sme_ticker_belongs_to_either_universe`.
//!
//! # What the four NIFTY tiers replace
//!
//! Nothing, in this crate — they had never existed here. The web pages offered
//! NIFTY 50 / 100 / 200 / 500 and produced them by SLICING
//! [`NIFTY_TOTAL_MARKET`], which is stored alphabetically, so the first fifty
//! rows were `360ONE, 3MINDIA, AADHARHFC, …` presented as the NIFTY 50. That
//! is a `CLAUDE.md` §3 rule 1 violation shipped as a feature: a claim about
//! index membership with no source behind it. D-0089 replaces the slice with
//! the exchange's own published constituent files. Wiring the API and the page
//! to these constants is deliberately NOT part of that change — the data and
//! the source note land first, and `api::server::universe_label` still emits
//! only `index|fno|ntm|other` until a later one.

use crate::instrument::{InstrumentKey, Kind};

/// The 750 NIFTY Total Market constituents.
///
/// niftyindices.com: "750 stocks ... all stocks that are part of Nifty 500
/// and Nifty Microcap 250". Transcribed from the local lake's NSE/CASH
/// directories, every one of which matched the primary broker's equity list
/// 750/750 with zero misses. The source and its evidence lane are recorded in
/// `docs/00-charter.md` §4a, as golden rule 1 requires of any claim about an
/// instrument.
///
/// Re-measured against both real masters on 2026-08-01 under the D-0025 gate:
/// **750 of 750 resolve as a kept equity in BOTH vendors**, with agreeing
/// ISINs and zero misses.
///
/// UNVERIFIED against an NSE constituent circular: this is a SNAPSHOT, and
/// index membership is rebalanced. See `docs/06-limits.md` §11.
pub const NIFTY_TOTAL_MARKET: [&str; 750] = [
    "360ONE",
    "3MINDIA",
    "AADHARHFC",
    "AARTIDRUGS",
    "AARTIIND",
    "AARTIPHARM",
    "AAVAS",
    "ABB",
    "ABBOTINDIA",
    "ABCAPITAL",
    "ABDL",
    "ABFRL",
    "ABLBL",
    "ABREL",
    "ABSLAMC",
    "ACC",
    "ACE",
    "ACI",
    "ACMESOLAR",
    "ACUTAAS",
    "ADANIENSOL",
    "ADANIENT",
    "ADANIGREEN",
    "ADANIPORTS",
    "ADANIPOWER",
    "ADVENZYMES",
    "AEGISLOG",
    "AEGISVOPAK",
    "AEQUS",
    "AETHER",
    "AFCONS",
    "AFFLE",
    "AGARWALEYE",
    "AGL",
    "AHLUCONT",
    "AIAENG",
    "AIIL",
    "AJANTPHARM",
    "AKUMS",
    "ALIVUS",
    "ALKEM",
    "ALKYLAMINE",
    "ALOKINDS",
    "AMBER",
    "AMBUJACEM",
    "ANANDRATHI",
    "ANANTRAJ",
    "ANGELONE",
    "ANTHEM",
    "ANUP",
    "ANURAS",
    "APARINDS",
    "APLAPOLLO",
    "APLLTD",
    "APOLLO",
    "APOLLOHOSP",
    "APOLLOTYRE",
    "APTUS",
    "ARE&M",
    "ARVIND",
    "ARVINDFASN",
    "ASAHIINDIA",
    "ASHAPURMIN",
    "ASHOKA",
    "ASHOKLEY",
    "ASIANPAINT",
    "ASKAUTOLTD",
    "ASTERDM",
    "ASTRAL",
    "ASTRAMICRO",
    "ATGL",
    "ATHERENERG",
    "ATLANTAELE",
    "ATUL",
    "AUBANK",
    "AURIONPRO",
    "AUROPHARMA",
    "AVALON",
    "AVANTIFEED",
    "AVL",
    "AWFIS",
    "AWL",
    "AXISBANK",
    "AXISCADES",
    "AZAD",
    "BAJAJ-AUTO",
    "BAJAJELEC",
    "BAJAJFINSV",
    "BAJAJHFL",
    "BAJAJHLDNG",
    "BAJFINANCE",
    "BALAMINES",
    "BALKRISIND",
    "BALRAMCHIN",
    "BALUFORGE",
    "BANCOINDIA",
    "BANDHANBNK",
    "BANKBARODA",
    "BANKINDIA",
    "BATAINDIA",
    "BAYERCROP",
    "BBOX",
    "BBTC",
    "BDL",
    "BECTORFOOD",
    "BEL",
    "BELRISE",
    "BEML",
    "BERGEPAINT",
    "BHARATFORG",
    "BHARTIARTL",
    "BHARTIHEXA",
    "BHEL",
    "BIKAJI",
    "BIOCON",
    "BIRLACORPN",
    "BLACKBUCK",
    "BLS",
    "BLUEDART",
    "BLUEJET",
    "BLUESTARCO",
    "BLUESTONE",
    "BORORENEW",
    "BOSCHLTD",
    "BPCL",
    "BRIGADE",
    "BRITANNIA",
    "BSE",
    "BSOFT",
    "CAMPUS",
    "CAMS",
    "CANBK",
    "CANFINHOME",
    "CANHLIFE",
    "CAPILLARY",
    "CAPLIPOINT",
    "CARBORUNIV",
    "CARTRADE",
    "CASTROLIND",
    "CCAVENUE",
    "CCL",
    "CDSL",
    "CEATLTD",
    "CELLO",
    "CEMPRO",
    "CENTRALBK",
    "CENTURYPLY",
    "CERA",
    "CESC",
    "CGCL",
    "CGPOWER",
    "CHALET",
    "CHAMBLFERT",
    "CHENNPETRO",
    "CHOICEIN",
    "CHOLAFIN",
    "CHOLAHLDNG",
    "CIEINDIA",
    "CIPLA",
    "CLEAN",
    "CMSINFO",
    "COALINDIA",
    "COCHINSHIP",
    "COFORGE",
    "COHANCE",
    "COLPAL",
    "CONCOR",
    "CONCORDBIO",
    "COROMANDEL",
    "CORONA",
    "CPPLUS",
    "CRAFTSMAN",
    "CRAMC",
    "CREDITACC",
    "CRISIL",
    "CRIZAC",
    "CROMPTON",
    "CSBBANK",
    "CUB",
    "CUMMINSIND",
    "CUPID",
    "CYIENT",
    "DABUR",
    "DALBHARAT",
    "DATAMATICS",
    "DATAPATTNS",
    "DBL",
    "DBREALTY",
    "DCBBANK",
    "DCMSHRIRAM",
    "DEEPAKFERT",
    "DEEPAKNTR",
    "DELHIVERY",
    "DEVYANI",
    "DIACABS",
    "DIVISLAB",
    "DIXON",
    "DLF",
    "DMART",
    "DOMS",
    "DRREDDY",
    "DYNAMATECH",
    "ECLERX",
    "EDELWEISS",
    "EICHERMOT",
    "EIDPARRY",
    "EIEL",
    "EIHOTEL",
    "ELECON",
    "ELECTCAST",
    "ELGIEQUIP",
    "ELLEN",
    "EMAMILTD",
    "EMBDL",
    "EMCURE",
    "EMIL",
    "EMMVEE",
    "ENDURANCE",
    "ENGINERSIN",
    "ENRIN",
    "ENTERO",
    "EPL",
    "EQUITASBNK",
    "ERIS",
    "ESCORTS",
    "ETERNAL",
    "ETHOSLTD",
    "EUREKAFORB",
    "EXIDEIND",
    "FACT",
    "FEDERALBNK",
    "FEDFINA",
    "FIEMIND",
    "FINCABLES",
    "FINPIPE",
    "FIRSTCRY",
    "FIVESTAR",
    "FLUOROCHEM",
    "FORCEMOT",
    "FORTIS",
    "FSL",
    "GABRIEL",
    "GAEL",
    "GAIL",
    "GALLANTT",
    "GESHIP",
    "GHCL",
    "GICRE",
    "GILLETTE",
    "GLAND",
    "GLAXO",
    "GLENMARK",
    "GMDCLTD",
    "GMMPFAUDLR",
    "GMRAIRPORT",
    "GMRP&UI",
    "GNFC",
    "GODFRYPHLP",
    "GODIGIT",
    "GODREJAGRO",
    "GODREJCP",
    "GODREJIND",
    "GODREJPROP",
    "GOKEX",
    "GOKULAGRO",
    "GPIL",
    "GPPL",
    "GRANULES",
    "GRAPHITE",
    "GRASIM",
    "GRAVITA",
    "GREAVESCOT",
    "GROWW",
    "GRSE",
    "GRWRHITECH",
    "GSFC",
    "GVT&D",
    "HAL",
    "HAPPSTMNDS",
    "HAVELLS",
    "HBLENGINE",
    "HCC",
    "HCG",
    "HCLTECH",
    "HDBFS",
    "HDFCAMC",
    "HDFCBANK",
    "HDFCLIFE",
    "HEG",
    "HEMIPROP",
    "HERITGFOOD",
    "HEROMOTOCO",
    "HEXT",
    "HFCL",
    "HGINFRA",
    "HINDALCO",
    "HINDCOPPER",
    "HINDPETRO",
    "HINDUNILVR",
    "HINDZINC",
    "HOMEFIRST",
    "HONASA",
    "HONAUT",
    "HSCL",
    "HUDCO",
    "HYUNDAI",
    "ICICIAMC",
    "ICICIBANK",
    "ICICIGI",
    "ICICIPRULI",
    "ICIL",
    "IDBI",
    "IDEA",
    "IDFCFIRSTB",
    "IEX",
    "IFBIND",
    "IFCI",
    "IGIL",
    "IGL",
    "IIFL",
    "IIFLCAPS",
    "IKS",
    "IMFA",
    "INDGN",
    "INDHOTEL",
    "INDIACEM",
    "INDIAGLYCO",
    "INDIAMART",
    "INDIANB",
    "INDIASHLTR",
    "INDIGO",
    "INDIGOPNTS",
    "INDUSINDBK",
    "INDUSTOWER",
    "INFY",
    "INOXGREEN",
    "INOXINDIA",
    "INOXWIND",
    "INTELLECT",
    "IOB",
    "IOC",
    "IONEXCHANG",
    "IPCALAB",
    "IRB",
    "IRCON",
    "IRCTC",
    "IREDA",
    "IRFC",
    "ITC",
    "ITCHOTELS",
    "ITI",
    "IXIGO",
    "J&KBANK",
    "JAIBALAJI",
    "JAINREC",
    "JAMNAAUTO",
    "JAYNECOIND",
    "JBMA",
    "JINDALSAW",
    "JINDALSTEL",
    "JIOFIN",
    "JKCEMENT",
    "JKLAKSHMI",
    "JKPAPER",
    "JKTYRE",
    "JLHL",
    "JMFINANCIL",
    "JPPOWER",
    "JSFB",
    "JSL",
    "JSLL",
    "JSWCEMENT",
    "JSWDULUX",
    "JSWENERGY",
    "JSWINFRA",
    "JSWSTEEL",
    "JUBLFOOD",
    "JUBLINGREA",
    "JUBLPHARMA",
    "JUSTDIAL",
    "JWL",
    "JYOTHYLAB",
    "JYOTICNC",
    "KAJARIACER",
    "KALYANKJIL",
    "KANSAINER",
    "KARURVYSYA",
    "KAYNES",
    "KEC",
    "KEI",
    "KFINTECH",
    "KIMS",
    "KIRLOSBROS",
    "KIRLOSENG",
    "KIRLPNU",
    "KITEX",
    "KNRCON",
    "KOTAKBANK",
    "KPIGREEN",
    "KPIL",
    "KPITTECH",
    "KPRMILL",
    "KRBL",
    "KRN",
    "KSB",
    "KSCL",
    "KTKBANK",
    "LALPATHLAB",
    "LATENTVIEW",
    "LAURUSLABS",
    "LEMONTREE",
    "LENSKART",
    "LGEINDIA",
    "LICHSGFIN",
    "LICI",
    "LINDEINDIA",
    "LLOYDSENGG",
    "LLOYDSENT",
    "LLOYDSME",
    "LODHA",
    "LOTUSDEV",
    "LT",
    "LTF",
    "LTFOODS",
    "LTM",
    "LTTS",
    "LUMAXTECH",
    "LUPIN",
    "LXCHEM",
    "M&M",
    "M&MFIN",
    "MAHABANK",
    "MAHSCOOTER",
    "MAHSEAMLES",
    "MANAPPURAM",
    "MANKIND",
    "MANORAMA",
    "MANYAVAR",
    "MAPMYINDIA",
    "MARICO",
    "MARKSANS",
    "MARUTI",
    "MASTEK",
    "MAXHEALTH",
    "MAZDOCK",
    "MCX",
    "MEDANTA",
    "MEDPLUS",
    "MEESHO",
    "METROPOLIS",
    "MFSL",
    "MGL",
    "MIDHANI",
    "MINDACORP",
    "MMTC",
    "MOIL",
    "MOTHERSON",
    "MOTILALOFS",
    "MPHASIS",
    "MRF",
    "MRPL",
    "MSTCLTD",
    "MSUMI",
    "MTARTECH",
    "MUTHOOTFIN",
    "NAM-INDIA",
    "NATCOPHARM",
    "NATIONALUM",
    "NAUKRI",
    "NAVA",
    "NAVINFLUOR",
    "NAZARA",
    "NBCC",
    "NCC",
    "NEOGEN",
    "NESCO",
    "NESTLEIND",
    "NETWEB",
    "NETWORK18",
    "NEULANDLAB",
    "NEWGEN",
    "NFL",
    "NH",
    "NHPC",
    "NIACL",
    "NIVABUPA",
    "NLCINDIA",
    "NMDC",
    "NSLNISP",
    "NTPC",
    "NTPCGREEN",
    "NUVAMA",
    "NUVOCO",
    "NYKAA",
    "OBEROIRLTY",
    "OFSS",
    "OIL",
    "OLAELEC",
    "OLECTRA",
    "ONESOURCE",
    "ONGC",
    "OPTIEMUS",
    "ORIENTCEM",
    "ORKLAINDIA",
    "OSWALPUMPS",
    "PAGEIND",
    "PARADEEP",
    "PARAS",
    "PARKHOSPS",
    "PATANJALI",
    "PAYTM",
    "PCBL",
    "PCJEWELLER",
    "PERSISTENT",
    "PETRONET",
    "PFC",
    "PFIZER",
    "PFOCUS",
    "PGEL",
    "PGIL",
    "PHOENIXLTD",
    "PICCADIL",
    "PIDILITIND",
    "PIIND",
    "PINELABS",
    "PIRAMALFIN",
    "PNB",
    "PNBHOUSING",
    "PNCINFRA",
    "PNGJL",
    "POLICYBZR",
    "POLYCAB",
    "POLYMED",
    "POONAWALLA",
    "POWERGRID",
    "POWERINDIA",
    "POWERMECH",
    "PPLPHARMA",
    "PRAJIND",
    "PREMIERENE",
    "PRESTIGE",
    "PRICOLLTD",
    "PRIVISCL",
    "PRSMJOHNSN",
    "PRUDENT",
    "PTC",
    "PTCIL",
    "PURVA",
    "PVRINOX",
    "PWL",
    "QPOWER",
    "QUESS",
    "RADICO",
    "RAILTEL",
    "RAIN",
    "RAINBOW",
    "RALLIS",
    "RAMCOCEM",
    "RATEGAIN",
    "RATNAMANI",
    "RAYMONDLSL",
    "RBA",
    "RBLBANK",
    "RCF",
    "RECLTD",
    "REDINGTON",
    "REDTAPE",
    "REFEX",
    "RELAXO",
    "RELIANCE",
    "RELIGARE",
    "RENUKA",
    "RHIM",
    "RITES",
    "RKFORGE",
    "ROUTE",
    "RPOWER",
    "RRKABEL",
    "RTNINDIA",
    "RTNPOWER",
    "RUBICON",
    "RVNL",
    "SAATVIKGL",
    "SAFARI",
    "SAGILITY",
    "SAIL",
    "SAILIFE",
    "SAMHI",
    "SAMMAANCAP",
    "SANDUMA",
    "SANOFICONR",
    "SANSERA",
    "SAPPHIRE",
    "SARDAEN",
    "SAREGAMA",
    "SBFC",
    "SBICARD",
    "SBILIFE",
    "SBIN",
    "SCHAEFFLER",
    "SCHNEIDER",
    "SCI",
    "SENCO",
    "SFL",
    "SHAILY",
    "SHAKTIPUMP",
    "SHARDACROP",
    "SHAREINDIA",
    "SHILPAMED",
    "SHREECEM",
    "SHRIPISTON",
    "SHRIRAMFIN",
    "SHYAMMETL",
    "SIEMENS",
    "SIGNATURE",
    "SJVN",
    "SKFINDIA",
    "SKFINDUS",
    "SKIPPER",
    "SKYGOLD",
    "SMARTWORKS",
    "SMLMAH",
    "SOBHA",
    "SOLARINDS",
    "SONACOMS",
    "SONATSOFTW",
    "SOUTHBANK",
    "SPARC",
    "SPLPETRO",
    "SRF",
    "STAR",
    "STARCEMENT",
    "STARHEALTH",
    "STLTECH",
    "STYL",
    "STYRENIX",
    "SUBROS",
    "SUDARSCHEM",
    "SUDEEPPHRM",
    "SUMICHEM",
    "SUNDARMFIN",
    "SUNPHARMA",
    "SUNTECK",
    "SUNTV",
    "SUPREMEIND",
    "SUPRIYA",
    "SURYAROSNI",
    "SUZLON",
    "SWANCORP",
    "SWIGGY",
    "SWSOLAR",
    "SYNGENE",
    "SYRMA",
    "TANLA",
    "TARC",
    "TARIL",
    "TATACAP",
    "TATACHEM",
    "TATACOMM",
    "TATACONSUM",
    "TATAELXSI",
    "TATAINVEST",
    "TATAPOWER",
    "TATASTEEL",
    "TATATECH",
    "TBOTEK",
    "TCS",
    "TDPOWERSYS",
    "TECHM",
    "TECHNOE",
    "TEGA",
    "TEJASNET",
    "TENNIND",
    "TEXRAIL",
    "THANGAMAYL",
    "THELEELA",
    "THERMAX",
    "THOMASCOOK",
    "THYROCARE",
    "TI",
    "TIINDIA",
    "TIMETECHNO",
    "TIMKEN",
    "TIPSMUSIC",
    "TITAGARH",
    "TITAN",
    "TMB",
    "TMCV",
    "TMPV",
    "TORNTPHARM",
    "TORNTPOWER",
    "TRANSRAILL",
    "TRAVELFOOD",
    "TRENT",
    "TRIDENT",
    "TRITURBINE",
    "TRIVENI",
    "TSFINV",
    "TTML",
    "TVSMOTOR",
    "TVSSCS",
    "UBL",
    "UCOBANK",
    "UJJIVANSFB",
    "ULTRACEMCO",
    "UNIONBANK",
    "UNITDSPR",
    "UNOMINDA",
    "UPL",
    "URBANCO",
    "USHAMART",
    "UTIAMC",
    "UTLSOLAR",
    "V2RETAIL",
    "VAIBHAVGBL",
    "VARROC",
    "VBL",
    "VEDL",
    "VGUARD",
    "VIJAYA",
    "VIKRAMSOLR",
    "VIPIND",
    "VIYASH",
    "VMART",
    "VMM",
    "VOLTAMP",
    "VOLTAS",
    "VTL",
    "WAAREEENER",
    "WAAREERTL",
    "WABAG",
    "WAKEFIT",
    "WEBELSOLAR",
    "WELCORP",
    "WELENT",
    "WELSPUNLIV",
    "WESTLIFE",
    "WEWORK",
    "WHIRLPOOL",
    "WIPRO",
    "WOCKPHARMA",
    "YATHARTH",
    "YESBANK",
    "ZAGGLE",
    "ZEEL",
    "ZENSARTECH",
    "ZENTEC",
    "ZFCVINDIA",
    "ZYDUSLIFE",
    "ZYDUSWELL",
];

/// Underlyings with listed futures or options on NSE.
///
/// DERIVED from both vendor masters and cross-checked: the two brokers agree
/// exactly — 213 each, zero on either side only. Exchange test instruments
/// (`*NSETEST`) are excluded. Recorded in `docs/00-charter.md` §4a.
///
/// Re-measured 2026-08-01 under the D-0025 gate: 208 of the 213 resolve as a
/// kept equity in **both** vendors. The other five are indices, which carry no
/// ISIN and therefore no cross-vendor check at all — `NIFTY`, `BANKNIFTY` and
/// `FINNIFTY` are named identically by both masters, while `MIDCPNIFTY` and
/// `NIFTYNXT50` are Dhan's spellings for the indices Groww calls
/// `NIFTYMIDSELECT` and `NIFTYJR`. **211 of 213 are therefore confirmed by two
/// vendors and 2 rest on one**, which `docs/06-limits.md` §12 records and
/// `api::server::report` states on every run rather than leaving implied.
pub const FNO_UNDERLYINGS: [&str; 213] = [
    "360ONE",
    "ABB",
    "ABCAPITAL",
    "ADANIENSOL",
    "ADANIENT",
    "ADANIGREEN",
    "ADANIPORTS",
    "ADANIPOWER",
    "ALKEM",
    "AMBER",
    "AMBUJACEM",
    "ANGELONE",
    "APLAPOLLO",
    "APOLLOHOSP",
    "ASHOKLEY",
    "ASIANPAINT",
    "ASTRAL",
    "AUBANK",
    "AUROPHARMA",
    "AXISBANK",
    "BAJAJ-AUTO",
    "BAJAJFINSV",
    "BAJAJHLDNG",
    "BAJFINANCE",
    "BANDHANBNK",
    "BANKBARODA",
    "BANKINDIA",
    "BANKNIFTY",
    "BDL",
    "BEL",
    "BHARATFORG",
    "BHARTIARTL",
    "BHEL",
    "BIOCON",
    "BLUESTARCO",
    "BOSCHLTD",
    "BPCL",
    "BRITANNIA",
    "BSE",
    "CAMS",
    "CANBK",
    "CDSL",
    "CGPOWER",
    "CHOLAFIN",
    "CIPLA",
    "COALINDIA",
    "COCHINSHIP",
    "COFORGE",
    "COLPAL",
    "CONCOR",
    "CROMPTON",
    "CUMMINSIND",
    "DABUR",
    "DALBHARAT",
    "DELHIVERY",
    "DIVISLAB",
    "DIXON",
    "DLF",
    "DMART",
    "DRREDDY",
    "EICHERMOT",
    "ETERNAL",
    "FEDERALBNK",
    "FINNIFTY",
    "FORCEMOT",
    "FORTIS",
    "GAIL",
    "GLENMARK",
    "GMRAIRPORT",
    "GODFRYPHLP",
    "GODREJCP",
    "GODREJPROP",
    "GRASIM",
    "GVT&D",
    "HAL",
    "HAVELLS",
    "HCLTECH",
    "HDFCAMC",
    "HDFCBANK",
    "HDFCLIFE",
    "HEROMOTOCO",
    "HINDALCO",
    "HINDPETRO",
    "HINDUNILVR",
    "HINDZINC",
    "HYUNDAI",
    "ICICIBANK",
    "ICICIGI",
    "ICICIPRULI",
    "IDEA",
    "IDFCFIRSTB",
    "IEX",
    "INDHOTEL",
    "INDIANB",
    "INDIGO",
    "INDUSINDBK",
    "INDUSTOWER",
    "INFY",
    "INOXWIND",
    "IOC",
    "IREDA",
    "IRFC",
    "ITC",
    "JINDALSTEL",
    "JIOFIN",
    "JSWENERGY",
    "JSWSTEEL",
    "JUBLFOOD",
    "KALYANKJIL",
    "KAYNES",
    "KEI",
    "KFINTECH",
    "KOTAKBANK",
    "KPITTECH",
    "LAURUSLABS",
    "LICHSGFIN",
    "LICI",
    "LODHA",
    "LT",
    "LTF",
    "LTM",
    "LUPIN",
    "M&M",
    "MANAPPURAM",
    "MANKIND",
    "MARICO",
    "MARUTI",
    "MAXHEALTH",
    "MAZDOCK",
    "MCX",
    "MFSL",
    "MIDCPNIFTY",
    "MOTHERSON",
    "MOTILALOFS",
    "MPHASIS",
    "MUTHOOTFIN",
    "NAM-INDIA",
    "NATIONALUM",
    "NAUKRI",
    "NBCC",
    "NESTLEIND",
    "NHPC",
    "NIFTY",
    "NIFTYNXT50",
    "NMDC",
    "NTPC",
    "NYKAA",
    "OBEROIRLTY",
    "OFSS",
    "OIL",
    "ONGC",
    "PAGEIND",
    "PATANJALI",
    "PAYTM",
    "PERSISTENT",
    "PETRONET",
    "PFC",
    "PGEL",
    "PHOENIXLTD",
    "PIDILITIND",
    "PIIND",
    "PNB",
    "PNBHOUSING",
    "POLICYBZR",
    "POLYCAB",
    "POWERGRID",
    "POWERINDIA",
    "PREMIERENE",
    "PRESTIGE",
    "RADICO",
    "RBLBANK",
    "RECLTD",
    "RELIANCE",
    "RVNL",
    "SAIL",
    "SBICARD",
    "SBILIFE",
    "SBIN",
    "SHREECEM",
    "SHRIRAMFIN",
    "SIEMENS",
    "SOLARINDS",
    "SONACOMS",
    "SRF",
    "SUNPHARMA",
    "SUPREMEIND",
    "SUZLON",
    "SWIGGY",
    "TATACONSUM",
    "TATAELXSI",
    "TATAPOWER",
    "TATASTEEL",
    "TCS",
    "TECHM",
    "TIINDIA",
    "TITAN",
    "TMPV",
    "TORNTPHARM",
    "TRENT",
    "TVSMOTOR",
    "ULTRACEMCO",
    "UNIONBANK",
    "UNITDSPR",
    "UNOMINDA",
    "UPL",
    "VBL",
    "VEDL",
    "VMM",
    "VOLTAS",
    "WAAREEENER",
    "WIPRO",
    "YESBANK",
    "ZYDUSLIFE",
];

/// The 50 NIFTY 50 constituents.
///
/// Transcribed from
/// <https://nsearchives.nseindia.com/content/indices/ind_nifty50list.csv>,
/// fetched 2026-08-11, and recorded in `docs/00-charter.md` §4c as golden
/// rule 1 requires of any claim about an instrument. The file is authored by
/// the exchange that computes the index, so this is a PUBLISHED membership
/// rather than a derived one — which is the whole difference between this
/// constant and the four ways the web pages used to fake it.
///
/// Symbols only. Every row of the source also carries a company name, an
/// industry, a series and an ISIN; the ISIN is what makes the row checkable
/// against a vendor master, and it is recorded in the charter rather than
/// duplicated here because nothing in this crate reads it.
///
/// UNVERIFIED against an NSE constituent circular: this is a SNAPSHOT and the
/// index is rebalanced semi-annually. See `docs/06-limits.md` §11.
pub const NIFTY_50: [&str; 50] = [
    "ADANIENT",
    "ADANIPORTS",
    "APOLLOHOSP",
    "ASIANPAINT",
    "AXISBANK",
    "BAJAJ-AUTO",
    "BAJAJFINSV",
    "BAJFINANCE",
    "BEL",
    "BHARTIARTL",
    "CIPLA",
    "COALINDIA",
    "DRREDDY",
    "EICHERMOT",
    "ETERNAL",
    "GRASIM",
    "HCLTECH",
    "HDFCBANK",
    "HDFCLIFE",
    "HINDALCO",
    "HINDUNILVR",
    "ICICIBANK",
    "INDIGO",
    "INFY",
    "ITC",
    "JIOFIN",
    "JSWSTEEL",
    "KOTAKBANK",
    "LT",
    "M&M",
    "MARUTI",
    "MAXHEALTH",
    "NESTLEIND",
    "NTPC",
    "ONGC",
    "POWERGRID",
    "RELIANCE",
    "SBILIFE",
    "SBIN",
    "SHRIRAMFIN",
    "SUNPHARMA",
    "TATACONSUM",
    "TATASTEEL",
    "TCS",
    "TECHM",
    "TITAN",
    "TMPV",
    "TRENT",
    "ULTRACEMCO",
    "WIPRO",
];

/// The 100 NIFTY 100 constituents.
///
/// Transcribed from
/// <https://nsearchives.nseindia.com/content/indices/ind_nifty100list.csv>,
/// fetched 2026-08-11, and recorded in `docs/00-charter.md` §4c.
///
/// Contains [`NIFTY_50`] entire — 50 of 50, zero outside — which is not
/// assumed from the names but asserted for every symbol by
/// `core::universe::the_published_tiers_nest_one_inside_the_next`.
///
/// UNVERIFIED against an NSE constituent circular: this is a SNAPSHOT and the
/// index is rebalanced semi-annually. See `docs/06-limits.md` §11.
pub const NIFTY_100: [&str; 100] = [
    "ABB",
    "ADANIENSOL",
    "ADANIENT",
    "ADANIGREEN",
    "ADANIPORTS",
    "ADANIPOWER",
    "AMBUJACEM",
    "APOLLOHOSP",
    "ASIANPAINT",
    "AXISBANK",
    "BAJAJ-AUTO",
    "BAJAJFINSV",
    "BAJAJHLDNG",
    "BAJFINANCE",
    "BANKBARODA",
    "BEL",
    "BHARTIARTL",
    "BOSCHLTD",
    "BPCL",
    "BRITANNIA",
    "CANBK",
    "CGPOWER",
    "CHOLAFIN",
    "CIPLA",
    "COALINDIA",
    "CUMMINSIND",
    "DIVISLAB",
    "DLF",
    "DMART",
    "DRREDDY",
    "EICHERMOT",
    "ENRIN",
    "ETERNAL",
    "GAIL",
    "GODREJCP",
    "GRASIM",
    "HAL",
    "HCLTECH",
    "HDFCAMC",
    "HDFCBANK",
    "HDFCLIFE",
    "HINDALCO",
    "HINDUNILVR",
    "HINDZINC",
    "HYUNDAI",
    "ICICIBANK",
    "INDHOTEL",
    "INDIGO",
    "INFY",
    "IOC",
    "IRFC",
    "ITC",
    "JINDALSTEL",
    "JIOFIN",
    "JSWSTEEL",
    "KOTAKBANK",
    "LODHA",
    "LT",
    "LTM",
    "M&M",
    "MARUTI",
    "MAXHEALTH",
    "MAZDOCK",
    "MOTHERSON",
    "MUTHOOTFIN",
    "NESTLEIND",
    "NTPC",
    "ONGC",
    "PFC",
    "PIDILITIND",
    "PNB",
    "POWERGRID",
    "RECLTD",
    "RELIANCE",
    "SBILIFE",
    "SBIN",
    "SHREECEM",
    "SHRIRAMFIN",
    "SIEMENS",
    "SOLARINDS",
    "SUNPHARMA",
    "TATACAP",
    "TATACONSUM",
    "TATAPOWER",
    "TATASTEEL",
    "TCS",
    "TECHM",
    "TITAN",
    "TMCV",
    "TMPV",
    "TORNTPHARM",
    "TRENT",
    "TVSMOTOR",
    "ULTRACEMCO",
    "UNIONBANK",
    "UNITDSPR",
    "VBL",
    "VEDL",
    "WIPRO",
    "ZYDUSLIFE",
];

/// The 200 NIFTY 200 constituents.
///
/// Transcribed from
/// <https://nsearchives.nseindia.com/content/indices/ind_nifty200list.csv>,
/// fetched 2026-08-11, and recorded in `docs/00-charter.md` §4c.
///
/// Contains [`NIFTY_100`] entire — 100 of 100, zero outside — asserted by
/// `core::universe::the_published_tiers_nest_one_inside_the_next`.
///
/// UNVERIFIED against an NSE constituent circular: this is a SNAPSHOT and the
/// index is rebalanced semi-annually. See `docs/06-limits.md` §11.
pub const NIFTY_200: [&str; 200] = [
    "360ONE",
    "ABB",
    "ABCAPITAL",
    "ADANIENSOL",
    "ADANIENT",
    "ADANIGREEN",
    "ADANIPORTS",
    "ADANIPOWER",
    "ALKEM",
    "AMBUJACEM",
    "APLAPOLLO",
    "APOLLOHOSP",
    "ASHOKLEY",
    "ASIANPAINT",
    "ASTRAL",
    "ATGL",
    "AUBANK",
    "AUROPHARMA",
    "AXISBANK",
    "BAJAJ-AUTO",
    "BAJAJFINSV",
    "BAJAJHLDNG",
    "BAJFINANCE",
    "BANKBARODA",
    "BANKINDIA",
    "BDL",
    "BEL",
    "BHARATFORG",
    "BHARTIARTL",
    "BHEL",
    "BIOCON",
    "BLUESTARCO",
    "BOSCHLTD",
    "BPCL",
    "BRITANNIA",
    "BSE",
    "CANBK",
    "CGPOWER",
    "CHOLAFIN",
    "CIPLA",
    "COALINDIA",
    "COCHINSHIP",
    "COFORGE",
    "COLPAL",
    "CONCOR",
    "COROMANDEL",
    "CUMMINSIND",
    "DABUR",
    "DIVISLAB",
    "DIXON",
    "DLF",
    "DMART",
    "DRREDDY",
    "EICHERMOT",
    "ENRIN",
    "ETERNAL",
    "EXIDEIND",
    "FEDERALBNK",
    "FORTIS",
    "GAIL",
    "GLENMARK",
    "GMRAIRPORT",
    "GODFRYPHLP",
    "GODREJCP",
    "GODREJPROP",
    "GRASIM",
    "GROWW",
    "GVT&D",
    "HAL",
    "HAVELLS",
    "HCLTECH",
    "HDFCAMC",
    "HDFCBANK",
    "HDFCLIFE",
    "HEROMOTOCO",
    "HINDALCO",
    "HINDPETRO",
    "HINDUNILVR",
    "HINDZINC",
    "HUDCO",
    "HYUNDAI",
    "ICICIAMC",
    "ICICIBANK",
    "ICICIGI",
    "IDEA",
    "IDFCFIRSTB",
    "INDHOTEL",
    "INDIANB",
    "INDIGO",
    "INDUSINDBK",
    "INDUSTOWER",
    "INFY",
    "IOC",
    "IRCTC",
    "IREDA",
    "IRFC",
    "ITC",
    "JINDALSTEL",
    "JIOFIN",
    "JSWENERGY",
    "JSWSTEEL",
    "JUBLFOOD",
    "KALYANKJIL",
    "KEI",
    "KOTAKBANK",
    "KPITTECH",
    "LAURUSLABS",
    "LENSKART",
    "LGEINDIA",
    "LICHSGFIN",
    "LODHA",
    "LT",
    "LTF",
    "LTM",
    "LUPIN",
    "M&M",
    "M&MFIN",
    "MANKIND",
    "MARICO",
    "MARUTI",
    "MAXHEALTH",
    "MAZDOCK",
    "MCX",
    "MFSL",
    "MOTHERSON",
    "MOTILALOFS",
    "MPHASIS",
    "MRF",
    "MUTHOOTFIN",
    "NATIONALUM",
    "NAUKRI",
    "NESTLEIND",
    "NHPC",
    "NMDC",
    "NTPC",
    "NYKAA",
    "OBEROIRLTY",
    "OFSS",
    "OIL",
    "ONGC",
    "PAGEIND",
    "PATANJALI",
    "PAYTM",
    "PERSISTENT",
    "PFC",
    "PHOENIXLTD",
    "PIDILITIND",
    "PIIND",
    "PNB",
    "POLICYBZR",
    "POLYCAB",
    "POWERGRID",
    "POWERINDIA",
    "PREMIERENE",
    "PRESTIGE",
    "RADICO",
    "RECLTD",
    "RELIANCE",
    "RVNL",
    "SAIL",
    "SBICARD",
    "SBILIFE",
    "SBIN",
    "SHREECEM",
    "SHRIRAMFIN",
    "SIEMENS",
    "SOLARINDS",
    "SRF",
    "SUNPHARMA",
    "SUPREMEIND",
    "SUZLON",
    "SWIGGY",
    "TATACAP",
    "TATACOMM",
    "TATACONSUM",
    "TATAELXSI",
    "TATAINVEST",
    "TATAPOWER",
    "TATASTEEL",
    "TCS",
    "TECHM",
    "TIINDIA",
    "TITAN",
    "TMCV",
    "TMPV",
    "TORNTPHARM",
    "TRENT",
    "TVSMOTOR",
    "ULTRACEMCO",
    "UNIONBANK",
    "UNITDSPR",
    "UPL",
    "VBL",
    "VEDL",
    "VMM",
    "VOLTAS",
    "WAAREEENER",
    "WIPRO",
    "YESBANK",
    "ZYDUSLIFE",
];

/// The 500 NIFTY 500 constituents.
///
/// Transcribed from
/// <https://nsearchives.nseindia.com/content/indices/ind_nifty500list.csv>,
/// fetched 2026-08-11, and recorded in `docs/00-charter.md` §4c.
///
/// Contains [`NIFTY_200`] entire — 200 of 200, zero outside — and is itself
/// contained entire by [`NIFTY_TOTAL_MARKET`]: all 500 resolve in the 750, with
/// zero outside. Both are asserted for every symbol by
/// `core::universe::the_published_tiers_nest_one_inside_the_next`, which lets
/// [`of_equity`] set the containing bits without inventing a membership no
/// source states.
///
/// That second containment is the load-bearing one. niftyindices.com defines
/// the Total Market as "all stocks that are part of Nifty 500 and Nifty
/// Microcap 250", and the published files agree with the definition exactly:
/// the union of the NIFTY 500 file and the NIFTY Microcap 250 file is the
/// NIFTY Total Market file, row for row, with nothing on either side alone.
/// The 750-versus-752 discrepancy D-0089 records lives entirely in the
/// Microcap 250 half, which this repository does not carry as a named tier —
/// so it cannot reach any of the four tiers here. See `docs/06-limits.md` §11.
///
/// UNVERIFIED against an NSE constituent circular: this is a SNAPSHOT and the
/// index is rebalanced semi-annually. See `docs/06-limits.md` §11.
pub const NIFTY_500: [&str; 500] = [
    "360ONE",
    "3MINDIA",
    "AADHARHFC",
    "AARTIIND",
    "AAVAS",
    "ABB",
    "ABBOTINDIA",
    "ABCAPITAL",
    "ABDL",
    "ABFRL",
    "ABLBL",
    "ABREL",
    "ABSLAMC",
    "ACC",
    "ACE",
    "ACMESOLAR",
    "ACUTAAS",
    "ADANIENSOL",
    "ADANIENT",
    "ADANIGREEN",
    "ADANIPORTS",
    "ADANIPOWER",
    "AEGISLOG",
    "AEGISVOPAK",
    "AFCONS",
    "AFFLE",
    "AIAENG",
    "AIIL",
    "AJANTPHARM",
    "ALKEM",
    "AMBER",
    "AMBUJACEM",
    "ANANDRATHI",
    "ANANTRAJ",
    "ANGELONE",
    "ANTHEM",
    "ANURAS",
    "APARINDS",
    "APLAPOLLO",
    "APOLLOHOSP",
    "APOLLOTYRE",
    "APTUS",
    "ARE&M",
    "ASAHIINDIA",
    "ASHOKLEY",
    "ASIANPAINT",
    "ASTERDM",
    "ASTRAL",
    "ATGL",
    "ATHERENERG",
    "ATUL",
    "AUBANK",
    "AUROPHARMA",
    "AWL",
    "AXISBANK",
    "BAJAJ-AUTO",
    "BAJAJFINSV",
    "BAJAJHFL",
    "BAJAJHLDNG",
    "BAJFINANCE",
    "BALKRISIND",
    "BALRAMCHIN",
    "BANDHANBNK",
    "BANKBARODA",
    "BANKINDIA",
    "BATAINDIA",
    "BAYERCROP",
    "BBTC",
    "BDL",
    "BEL",
    "BELRISE",
    "BEML",
    "BERGEPAINT",
    "BHARATFORG",
    "BHARTIARTL",
    "BHARTIHEXA",
    "BHEL",
    "BIKAJI",
    "BIOCON",
    "BLS",
    "BLUEDART",
    "BLUEJET",
    "BLUESTARCO",
    "BOSCHLTD",
    "BPCL",
    "BRIGADE",
    "BRITANNIA",
    "BSE",
    "BSOFT",
    "CAMS",
    "CANBK",
    "CANFINHOME",
    "CANHLIFE",
    "CAPLIPOINT",
    "CARBORUNIV",
    "CARTRADE",
    "CASTROLIND",
    "CCL",
    "CDSL",
    "CEATLTD",
    "CEMPRO",
    "CENTRALBK",
    "CESC",
    "CGCL",
    "CGPOWER",
    "CHALET",
    "CHAMBLFERT",
    "CHENNPETRO",
    "CHOICEIN",
    "CHOLAFIN",
    "CHOLAHLDNG",
    "CIEINDIA",
    "CIPLA",
    "CLEAN",
    "COALINDIA",
    "COCHINSHIP",
    "COFORGE",
    "COHANCE",
    "COLPAL",
    "CONCOR",
    "CONCORDBIO",
    "COROMANDEL",
    "CPPLUS",
    "CRAFTSMAN",
    "CREDITACC",
    "CRISIL",
    "CROMPTON",
    "CUB",
    "CUMMINSIND",
    "CYIENT",
    "DABUR",
    "DALBHARAT",
    "DATAPATTNS",
    "DCMSHRIRAM",
    "DEEPAKFERT",
    "DEEPAKNTR",
    "DELHIVERY",
    "DEVYANI",
    "DIVISLAB",
    "DIXON",
    "DLF",
    "DMART",
    "DOMS",
    "DRREDDY",
    "ECLERX",
    "EICHERMOT",
    "EIDPARRY",
    "EIHOTEL",
    "ELECON",
    "ELGIEQUIP",
    "EMAMILTD",
    "EMCURE",
    "EMMVEE",
    "ENDURANCE",
    "ENGINERSIN",
    "ENRIN",
    "ERIS",
    "ESCORTS",
    "ETERNAL",
    "EXIDEIND",
    "FACT",
    "FEDERALBNK",
    "FINCABLES",
    "FIRSTCRY",
    "FIVESTAR",
    "FLUOROCHEM",
    "FORCEMOT",
    "FORTIS",
    "FSL",
    "GABRIEL",
    "GAIL",
    "GALLANTT",
    "GESHIP",
    "GICRE",
    "GILLETTE",
    "GLAND",
    "GLAXO",
    "GLENMARK",
    "GMDCLTD",
    "GMRAIRPORT",
    "GODFRYPHLP",
    "GODIGIT",
    "GODREJCP",
    "GODREJIND",
    "GODREJPROP",
    "GPIL",
    "GRANULES",
    "GRAPHITE",
    "GRASIM",
    "GRAVITA",
    "GROWW",
    "GRSE",
    "GVT&D",
    "HAL",
    "HAVELLS",
    "HBLENGINE",
    "HCLTECH",
    "HDBFS",
    "HDFCAMC",
    "HDFCBANK",
    "HDFCLIFE",
    "HEG",
    "HEROMOTOCO",
    "HEXT",
    "HFCL",
    "HINDALCO",
    "HINDCOPPER",
    "HINDPETRO",
    "HINDUNILVR",
    "HINDZINC",
    "HOMEFIRST",
    "HONASA",
    "HONAUT",
    "HSCL",
    "HUDCO",
    "HYUNDAI",
    "ICICIAMC",
    "ICICIBANK",
    "ICICIGI",
    "ICICIPRULI",
    "IDBI",
    "IDEA",
    "IDFCFIRSTB",
    "IEX",
    "IFCI",
    "IGIL",
    "IGL",
    "IIFL",
    "IKS",
    "INDGN",
    "INDHOTEL",
    "INDIACEM",
    "INDIAMART",
    "INDIANB",
    "INDIGO",
    "INDUSINDBK",
    "INDUSTOWER",
    "INFY",
    "INOXWIND",
    "INTELLECT",
    "IOB",
    "IOC",
    "IPCALAB",
    "IRB",
    "IRCON",
    "IRCTC",
    "IREDA",
    "IRFC",
    "ITC",
    "ITCHOTELS",
    "ITI",
    "J&KBANK",
    "JAINREC",
    "JBMA",
    "JINDALSAW",
    "JINDALSTEL",
    "JIOFIN",
    "JKCEMENT",
    "JKTYRE",
    "JMFINANCIL",
    "JPPOWER",
    "JSL",
    "JSWCEMENT",
    "JSWDULUX",
    "JSWENERGY",
    "JSWINFRA",
    "JSWSTEEL",
    "JUBLFOOD",
    "JUBLINGREA",
    "JUBLPHARMA",
    "JWL",
    "JYOTICNC",
    "KAJARIACER",
    "KALYANKJIL",
    "KARURVYSYA",
    "KAYNES",
    "KEC",
    "KEI",
    "KFINTECH",
    "KIMS",
    "KIRLOSENG",
    "KOTAKBANK",
    "KPIL",
    "KPITTECH",
    "KPRMILL",
    "LALPATHLAB",
    "LATENTVIEW",
    "LAURUSLABS",
    "LEMONTREE",
    "LENSKART",
    "LGEINDIA",
    "LICHSGFIN",
    "LICI",
    "LINDEINDIA",
    "LLOYDSME",
    "LODHA",
    "LT",
    "LTF",
    "LTFOODS",
    "LTM",
    "LTTS",
    "LUPIN",
    "M&M",
    "M&MFIN",
    "MAHABANK",
    "MANAPPURAM",
    "MANKIND",
    "MAPMYINDIA",
    "MARICO",
    "MARUTI",
    "MAXHEALTH",
    "MAZDOCK",
    "MCX",
    "MEDANTA",
    "MEESHO",
    "MFSL",
    "MGL",
    "MINDACORP",
    "MMTC",
    "MOTHERSON",
    "MOTILALOFS",
    "MPHASIS",
    "MRF",
    "MRPL",
    "MSUMI",
    "MUTHOOTFIN",
    "NAM-INDIA",
    "NATCOPHARM",
    "NATIONALUM",
    "NAUKRI",
    "NAVA",
    "NAVINFLUOR",
    "NBCC",
    "NCC",
    "NESTLEIND",
    "NETWEB",
    "NEULANDLAB",
    "NEWGEN",
    "NH",
    "NHPC",
    "NIACL",
    "NIVABUPA",
    "NLCINDIA",
    "NMDC",
    "NSLNISP",
    "NTPC",
    "NTPCGREEN",
    "NUVAMA",
    "NUVOCO",
    "NYKAA",
    "OBEROIRLTY",
    "OFSS",
    "OIL",
    "OLAELEC",
    "OLECTRA",
    "ONESOURCE",
    "ONGC",
    "PAGEIND",
    "PARADEEP",
    "PATANJALI",
    "PAYTM",
    "PCBL",
    "PERSISTENT",
    "PETRONET",
    "PFC",
    "PFIZER",
    "PFOCUS",
    "PGEL",
    "PHOENIXLTD",
    "PIDILITIND",
    "PIIND",
    "PINELABS",
    "PIRAMALFIN",
    "PNB",
    "PNBHOUSING",
    "POLICYBZR",
    "POLYCAB",
    "POLYMED",
    "POONAWALLA",
    "POWERGRID",
    "POWERINDIA",
    "PPLPHARMA",
    "PREMIERENE",
    "PRESTIGE",
    "PTCIL",
    "PVRINOX",
    "PWL",
    "RADICO",
    "RAILTEL",
    "RAINBOW",
    "RAMCOCEM",
    "RBLBANK",
    "RECLTD",
    "REDINGTON",
    "RELIANCE",
    "RHIM",
    "RITES",
    "RKFORGE",
    "RPOWER",
    "RRKABEL",
    "RVNL",
    "SAGILITY",
    "SAIL",
    "SAILIFE",
    "SAMMAANCAP",
    "SAPPHIRE",
    "SARDAEN",
    "SAREGAMA",
    "SBFC",
    "SBICARD",
    "SBILIFE",
    "SBIN",
    "SCHAEFFLER",
    "SCHNEIDER",
    "SCI",
    "SHREECEM",
    "SHRIRAMFIN",
    "SHYAMMETL",
    "SIEMENS",
    "SIGNATURE",
    "SJVN",
    "SOBHA",
    "SOLARINDS",
    "SONACOMS",
    "SONATSOFTW",
    "SPLPETRO",
    "SRF",
    "STARHEALTH",
    "SUMICHEM",
    "SUNDARMFIN",
    "SUNPHARMA",
    "SUNTV",
    "SUPREMEIND",
    "SUZLON",
    "SWANCORP",
    "SWIGGY",
    "SYNGENE",
    "SYRMA",
    "TARIL",
    "TATACAP",
    "TATACHEM",
    "TATACOMM",
    "TATACONSUM",
    "TATAELXSI",
    "TATAINVEST",
    "TATAPOWER",
    "TATASTEEL",
    "TATATECH",
    "TBOTEK",
    "TCS",
    "TECHM",
    "TECHNOE",
    "TEGA",
    "TEJASNET",
    "TENNIND",
    "THELEELA",
    "THERMAX",
    "TIINDIA",
    "TIMKEN",
    "TITAGARH",
    "TITAN",
    "TMCV",
    "TMPV",
    "TORNTPHARM",
    "TORNTPOWER",
    "TRAVELFOOD",
    "TRENT",
    "TRIDENT",
    "TRITURBINE",
    "TTML",
    "TVSMOTOR",
    "UBL",
    "UCOBANK",
    "ULTRACEMCO",
    "UNIONBANK",
    "UNITDSPR",
    "UNOMINDA",
    "UPL",
    "URBANCO",
    "USHAMART",
    "UTIAMC",
    "VBL",
    "VEDL",
    "VIJAYA",
    "VMM",
    "VOLTAS",
    "VTL",
    "WAAREEENER",
    "WELCORP",
    "WELSPUNLIV",
    "WHIRLPOOL",
    "WIPRO",
    "WOCKPHARMA",
    "YESBANK",
    "ZEEL",
    "ZENSARTECH",
    "ZENTEC",
    "ZFCVINDIA",
    "ZYDUSLIFE",
    "ZYDUSWELL",
];

/// Which universes an instrument belongs to.
///
/// A bitset rather than an enum: an instrument belongs to **many** universes
/// at once — a stock can be an F&O underlying *and* a Total Market
/// constituent. Bits are append-only, exactly like the condition table: a new
/// universe takes the next free bit and invalidates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Universe(u32);

impl Universe {
    /// Belongs to no universe. Stored, never listed.
    pub const NONE: Self = Self(0);
    /// A spot index.
    pub const INDEX: Self = Self(1 << 0);
    /// Has listed futures or options.
    pub const FNO: Self = Self(1 << 1);
    /// A NIFTY Total Market constituent.
    pub const TOTAL_MARKET: Self = Self(1 << 2);
    /// A NIFTY 500 constituent.
    ///
    /// Bits 3 through 6 were appended by D-0089 and are ordered so that a
    /// HIGHER bit is a NARROWER tier: 500, 200, 100, 50. That is the reading
    /// order of the containment ladder, and it is only a mnemonic — nothing
    /// compares two universe values numerically, and nothing may start,
    /// because bit order is a convenience and membership is a set.
    ///
    /// Bits 0, 1 and 2 keep the positions they have always had. `CLAUDE.md`
    /// §3 rule 8 forbids renumbering, and a stored `bits()` from before this
    /// change still decodes to the same three universes it always did.
    pub const NIFTY_500: Self = Self(1 << 3);
    /// A NIFTY 200 constituent.
    pub const NIFTY_200: Self = Self(1 << 4);
    /// A NIFTY 100 constituent.
    pub const NIFTY_100: Self = Self(1 << 5);
    /// A NIFTY 50 constituent.
    pub const NIFTY_50: Self = Self(1 << 6);

    /// Whether this set contains every bit of `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether this set is empty.
    #[must_use]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// The union of two sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The raw bits.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// The universes a cash-segment symbol belongs to.
///
/// **This line said "binary search over sorted arrays: at most ten comparisons
/// on 750 entries".** There has been no binary search here since D-0065
/// replaced it with [`MemberIndex`]; the sentence outlived the code by one
/// decision and is corrected by D-0089. Six hashed probes now, each bounded by
/// the table's fill factor and none of them growing with the list — the cost
/// [`MemberIndex`] documents, paid six times instead of twice.
///
/// # Why six probes and not one probe plus arithmetic
///
/// The tiers nest — every NIFTY 50 name is in the 100, the 100 in the 200, the
/// 200 in the 500, the 500 in the Total Market — so a narrowest-hit-wins
/// lookup could set the containing bits from the ladder alone and stop after
/// one hit. It does not, because that would make this function state a
/// membership no file states: it would DERIVE "RELIANCE is in the NIFTY 200"
/// from "RELIANCE is in the NIFTY 50" rather than from
/// `ind_nifty200list.csv`, and a rebalance that broke the nesting would be
/// answered confidently and wrongly.
///
/// Each bit is therefore read from its own published file, and the nesting is
/// a CHECKED property rather than an assumed one —
/// `core::universe::the_published_tiers_nest_one_inside_the_next` asserts it
/// symbol by symbol across all 750. If a snapshot stops nesting it fails and
/// says so; this function keeps answering what the files say either way.
/// `CLAUDE.md` §3 rule 1 and §4's "degrade loudly" row, in that order.
#[must_use]
pub fn of_equity(symbol: &str) -> Universe {
    let mut u = Universe::NONE;
    if FNO_INDEX.contains(symbol) {
        u = u.union(Universe::FNO);
    }
    if NTM_INDEX.contains(symbol) {
        u = u.union(Universe::TOTAL_MARKET);
    }
    if NIFTY_500_INDEX.contains(symbol) {
        u = u.union(Universe::NIFTY_500);
    }
    if NIFTY_200_INDEX.contains(symbol) {
        u = u.union(Universe::NIFTY_200);
    }
    if NIFTY_100_INDEX.contains(symbol) {
        u = u.union(Universe::NIFTY_100);
    }
    if NIFTY_50_INDEX.contains(symbol) {
        u = u.union(Universe::NIFTY_50);
    }
    u
}

/// An open-addressed membership table, built at compile time.
///
/// # Why not `binary_search`
///
/// `binary_search` over 750 entries is ~10 comparisons — O(log n) wearing an
/// O(1) label. It is fast, and it is not constant: doubling the list adds a
/// comparison. `docs/06-limits.md` recorded it as the one lookup in the engine
/// that grows with its input.
///
/// # Why not a `HashSet`
///
/// `core` declares no dependency at all (CI gate 9), so there is no hashing
/// crate available, and `std::collections::HashSet` cannot be built in a
/// `const` — it would need lazy initialisation, a lock on first use, and a
/// runtime allocation for a set whose contents are known when the file is
/// compiled.
///
/// # What this is
///
/// A power-of-two table of `Option<&str>` filled by linear probing at compile
/// time. Lookup hashes once, masks, and probes. The table is sized so it is at
/// most half full, which bounds the probe length: with 750 entries in 2048
/// slots the expected probe is under 1.5, and the worst observed is asserted by
/// `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1` —
/// which walks the table the way `contains` does and pins a NUMBER — so the
/// bound is measured rather than assumed.
///
/// Costs one pointer per slot — 32 KiB for the larger table. That is the space
/// traded for the time, and it is constant rather than growing with the data.
pub struct MemberIndex<const N: usize> {
    /// `pub(crate)` so the probe-length tests can walk the table the way
    /// [`Self::contains`] does and COUNT the steps. Layer 4's bound is the
    /// probe length, and a test that cannot see the slots can only assert the
    /// answer, never the cost. Crate-visible and no wider: nothing outside
    /// `core` has a reason to reach past `contains`.
    pub(crate) slots: [Option<&'static str>; N],
}

impl<const N: usize> MemberIndex<N> {
    /// Builds the table at compile time from a list of members.
    ///
    /// # Panics
    ///
    /// At COMPILE time if the table cannot hold the list — a `const` panic is
    /// a build error, not a runtime one, so an over-full table can never ship.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "both indices are provably in range: `i` is bounded by the \
                  loop condition `i < members.len()`, and `at` comes from \
                  `mask`, which masks to N-1 and so cannot reach N. A `const \
                  fn` cannot use `.get()` on a slice of references anyway, and \
                  an out-of-range index here would be a COMPILE error, not a \
                  runtime panic."
    )]
    pub const fn build(members: &[&'static str]) -> Self {
        assert!(
            members.len() * 2 <= N,
            "the table must stay at most half full so probing stays bounded"
        );
        let mut slots = [None; N];
        let mut i = 0;
        while i < members.len() {
            let mut at = mask(fnv1a(members[i]), N);
            // Linear probing. Terminates because the table is at most half
            // full, which the assert above guarantees.
            while slots[at].is_some() {
                at = (at + 1) & (N - 1);
            }
            slots[at] = Some(members[i]);
            i += 1;
        }
        Self { slots }
    }

    /// Whether the table holds this symbol.
    ///
    /// Hash, mask, probe. The probe stops at the first empty slot, which exists
    /// because the table is at most half full.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "`at` comes from `mask`, which masks to N-1, so it cannot \
                  reach N. The probe advances by the same mask, so it stays in \
                  range for every iteration."
    )]
    pub fn contains(&self, symbol: &str) -> bool {
        // A STRING TOO LONG TO BE A SYMBOL CANNOT BE A MEMBER, SO IT IS NEVER
        // HASHED.
        //
        // `fnv1a` walks the whole argument with no bound of its own, and
        // `of_equity` is public, takes an unguarded `&str`, and calls this
        // once per table — TWICE when this guard was written, SIX times since
        // D-0089 added the four published NIFTY tiers. So the cost of a
        // membership probe was the caller's string length, not the table's
        // size, and it was about to become three times that.
        //
        // Measured before this line existed, against gate 8's 3.0x ceiling:
        //      8 B          8,256 ps    baseline
        //     24 B         14,333 ps    1.736x
        //      1 KiB    1,806,225 ps    218.777x   BREACH
        //      4 MiB  7,663,937,500 ps  928,287x   BREACH  (7.66 ms in ONE call)
        // The per-byte slope agreed across those last two — 882 ps/byte at
        // 1 KiB, 914 ps/byte at 4 MiB — a clean linear fit over a 4,096x span
        // of input. That is not a constant-time operation wearing a bad
        // constant; it is O(n) in the argument.
        //
        // The guard is a CORRECTNESS statement before it is a cost one: every
        // member of every table is a `Symbol`, and `Symbol::new` refuses
        // anything past `SYMBOL_CAPACITY`. So a longer string has no member it
        // could equal, and answering `false` without hashing is the same
        // answer arrived at sooner. It is checked here rather than in
        // `of_equity` because `contains` is the function that hashes, and a
        // guard on the caller leaves the next caller unprotected.
        if symbol.len() > crate::symbol::SYMBOL_CAPACITY {
            return false;
        }
        let mut at = mask(fnv1a(symbol), N);
        while let Some(held) = self.slots[at] {
            if held == symbol {
                return true;
            }
            at = (at + 1) & (N - 1);
        }
        false
    }

    /// How many members the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// Whether the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Folds a hash into a slot index for a table of `n` slots.
///
/// Masks in `u64` and casts afterwards, never the reverse: casting first would
/// truncate on a 32-bit pointer target before the mask could narrow it. After
/// the mask the value is at most `n - 1`, which fits any pointer width this
/// engine builds for.
///
/// `pub(crate)` for the same reason [`MemberIndex::slots`] is: a probe-length
/// test in another module of this crate must start where `contains` starts,
/// and a second copy of this arithmetic in a test is a copy free to disagree
/// with the one under test.
pub(crate) const fn mask(hash: u64, n: usize) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the mask has already reduced the value to at most n-1, and n \
                  is a compile-time table size far below u32::MAX, so the cast \
                  is exact on every target."
    )]
    {
        (hash & (n as u64 - 1)) as usize
    }
}

/// FNV-1a over the bytes of a symbol.
///
/// Chosen because it is four lines, has no dependency, and is `const` — the
/// tables below are built by the compiler, not on first use. It is not
/// collision-resistant and does not need to be: the table stores the full
/// string and compares it, so a collision costs one extra probe rather than a
/// wrong answer.
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "`i` is bounded by the loop condition `i < bytes.len()`. A `const \
              fn` cannot use an iterator over a slice, and an out-of-range \
              index in a const context is a COMPILE error rather than a \
              runtime panic."
)]
pub const fn fnv1a(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    hash
}

/// The F&O underlyings, indexed. 213 members in 1024 slots.
pub static FNO_INDEX: MemberIndex<1024> = MemberIndex::build(&FNO_UNDERLYINGS);

/// The NIFTY Total Market constituents, indexed. 750 members in 2048 slots.
pub static NTM_INDEX: MemberIndex<2048> = MemberIndex::build(&NIFTY_TOTAL_MARKET);

/// The NIFTY 500 constituents, indexed. 500 members in 2048 slots.
///
/// # Why a QUARTER full and not the half [`MemberIndex::build`] allows
///
/// `build` asserts `members * 2 <= N`, and 500 members in 1024 slots satisfies
/// it. It was written that way first, and the probe test refused it:
///
/// | table | slots | fill | worst probe |
/// |---|---|---|---|
/// | NIFTY 500 | 1024 | 48.8% | **13** — over the bound of 8 |
/// | NIFTY 500 | 2048 | 24.4% | 5 |
///
/// The `build` assertion is a TERMINATION guarantee — at most half full means
/// an empty slot always exists, so the probe loop ends. It is not a COST
/// guarantee, and the two were never the same bound. Linear probing degrades
/// sharply near half: the clusters merge. The existing two tables happen to
/// sit at 36.6% and 20.8%, so nothing had ever exercised the difference, and
/// the `<= 8` in
/// `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1` was
/// a measurement taken only at those densities.
///
/// So every tier below is sized to keep the fill at or under a quarter, which
/// is the density the measured bound actually came from. 3,840 slots, 30 KiB
/// of pointers on a 64-bit target — constant, known at link time, and the
/// price of the probe bound `CLAUDE.md` §3 rule 4 asks for. Recorded in
/// `docs/06-limits.md` §11 and D-0089.
pub static NIFTY_500_INDEX: MemberIndex<2048> = MemberIndex::build(&NIFTY_500);

/// The NIFTY 200 constituents, indexed. 200 members in 1024 slots.
pub static NIFTY_200_INDEX: MemberIndex<1024> = MemberIndex::build(&NIFTY_200);

/// The NIFTY 100 constituents, indexed. 100 members in 512 slots.
pub static NIFTY_100_INDEX: MemberIndex<512> = MemberIndex::build(&NIFTY_100);

/// The NIFTY 50 constituents, indexed. 50 members in 256 slots.
pub static NIFTY_50_INDEX: MemberIndex<256> = MemberIndex::build(&NIFTY_50);

/// The universes a merged instrument belongs to.
///
/// The entry point every caller outside this module uses, so that "which list
/// is this in" is answered in one place from the whole key rather than from a
/// bare string a caller had to remember to derive correctly. An index is
/// [`Universe::INDEX`] and is never looked up in the equity lists — a spot
/// index is not a constituent of itself, and `NIFTY` appears in
/// [`FNO_UNDERLYINGS`] as the *underlying of its options*, which is a different
/// fact from being a share.
#[must_use]
pub fn of_instrument(key: &InstrumentKey) -> Universe {
    match key.kind {
        Kind::Index => Universe::INDEX.union(of_equity(key.underlying.as_str())),
        Kind::Equity => of_equity(key.underlying.as_str()),
        // A live derivative is never stored, so it never reaches a universe.
        // Saying `NONE` is accurate; inventing membership for it would not be.
        Kind::Future { .. } | Kind::Option { .. } => Universe::NONE,
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;

    #[test]
    fn a_collision_probes_forward_instead_of_overwriting() {
        // The probe loop inside `build` only runs when two members land on the
        // same slot. With a tiny table that is easy to force, and it is the one
        // branch that decides whether a collision costs a step or silently
        // loses a member.
        //
        // "A" and "Q" both hash to slot 12 of 16 — computed against this exact
        // FNV-1a, not hoped for. A test that merely fills a small table does
        // not necessarily collide, and then this branch stays unentered while
        // the test passes.
        assert_eq!(
            mask(fnv1a("A"), 16),
            mask(fnv1a("Q"), 16),
            "the pair collides"
        );
        let members = ["A", "Q", "B", "C", "D", "E", "F", "G"];
        let idx: MemberIndex<16> = MemberIndex::build(&members);
        assert_eq!(idx.len(), members.len(), "no member was overwritten");
        for m in members {
            assert!(idx.contains(m), "{m} survived the collision");
        }
        // And a member that was never inserted is still absent, so probing
        // forward did not turn the table into a set of everything.
        assert!(!idx.contains("I"));
        assert!(!idx.contains("AA"));
    }

    #[test]
    fn the_table_builder_is_exercised_at_runtime_not_only_at_compile_time() {
        // `MemberIndex::build` is a `const fn` and the two real tables are
        // `static`, so the compiler evaluates it and the runtime coverage
        // instrumentation never sees a single line of it. Code that runs at
        // build time is code no test can enter — which is the same hole as an
        // unreachable branch, arriving by a different route.
        //
        // Calling it here, in a non-const context, instruments it.
        let idx: MemberIndex<8> = MemberIndex::build(&["A", "BB", "CCC"]);
        assert_eq!(idx.len(), 3);
        assert!(!idx.is_empty());
        assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
        assert!(!idx.contains("D"));

        // An empty table: every probe hits the first empty slot immediately.
        let empty: MemberIndex<4> = MemberIndex::build(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(!empty.contains("anything"));

        // A FULL-to-the-limit table still probes correctly. Two slots of four
        // is the most `build` permits, and it is where clustering is worst.
        let tight: MemberIndex<4> = MemberIndex::build(&["X", "Y"]);
        assert!(tight.contains("X") && tight.contains("Y"));
        assert!(!tight.contains("Z"));
    }

    #[test]
    fn the_index_holds_every_member_and_nothing_else() {
        // Exhaustive: every one of the 1,813 members must be found, and the
        // tables must hold exactly as many as the lists do -- a collision that
        // silently dropped a member would leave an instrument permanently
        // outside its own universe.
        for m in NIFTY_TOTAL_MARKET {
            assert!(NTM_INDEX.contains(m), "{m} is a Total Market constituent");
        }
        for m in FNO_UNDERLYINGS {
            assert!(FNO_INDEX.contains(m), "{m} is an F&O underlying");
        }
        for m in NIFTY_500 {
            assert!(NIFTY_500_INDEX.contains(m), "{m} is a NIFTY 500 member");
        }
        for m in NIFTY_200 {
            assert!(NIFTY_200_INDEX.contains(m), "{m} is a NIFTY 200 member");
        }
        for m in NIFTY_100 {
            assert!(NIFTY_100_INDEX.contains(m), "{m} is a NIFTY 100 member");
        }
        for m in NIFTY_50 {
            assert!(NIFTY_50_INDEX.contains(m), "{m} is a NIFTY 50 member");
        }
        assert_eq!(NTM_INDEX.len(), NIFTY_TOTAL_MARKET.len());
        assert_eq!(FNO_INDEX.len(), FNO_UNDERLYINGS.len());
        assert_eq!(NIFTY_500_INDEX.len(), NIFTY_500.len());
        assert_eq!(NIFTY_200_INDEX.len(), NIFTY_200.len());
        assert_eq!(NIFTY_100_INDEX.len(), NIFTY_100.len());
        assert_eq!(NIFTY_50_INDEX.len(), NIFTY_50.len());
        assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
        assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
        assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());

        for absent in ["", "ZZZZNOTREAL", "NIFT", "RELIANCEX", "  ", "nifty"] {
            assert!(!NTM_INDEX.contains(absent), "{absent:?} is not a member");
            assert!(!FNO_INDEX.contains(absent), "{absent:?} is not a member");
            assert!(!NIFTY_500_INDEX.contains(absent), "{absent:?} is not one");
            assert!(!NIFTY_200_INDEX.contains(absent), "{absent:?} is not one");
            assert!(!NIFTY_100_INDEX.contains(absent), "{absent:?} is not one");
            assert!(!NIFTY_50_INDEX.contains(absent), "{absent:?} is not one");
        }
    }

    #[test]
    fn the_probe_length_is_bounded_which_is_what_makes_it_o1() {
        // THE CLAIM UNDER TEST. `binary_search` cost ~10 comparisons and grew
        // with the list; this must not grow at all. Measured by walking the
        // table the same way `contains` does and counting the steps.
        //
        // The bound is asserted as a NUMBER, not as "small": a probe length
        // that crept up with a future member would otherwise pass silently.
        fn worst_probe<const N: usize>(idx: &MemberIndex<N>, members: &[&str]) -> usize {
            let mut worst = 0;
            for m in members {
                let start = mask(fnv1a(m), N);
                let mut at = start;
                let mut steps = 1;
                while let Some(held) = idx.slots[at] {
                    if held == *m {
                        break;
                    }
                    at = (at + 1) & (N - 1);
                    steps += 1;
                }
                worst = worst.max(steps);
            }
            worst
        }

        let ntm = worst_probe(&NTM_INDEX, &NIFTY_TOTAL_MARKET);
        let fno = worst_probe(&FNO_INDEX, &FNO_UNDERLYINGS);
        assert!(
            ntm <= 8,
            "750 in 2048 slots must probe at most 8 times, got {ntm}"
        );
        assert!(
            fno <= 8,
            "213 in 1024 slots must probe at most 8 times, got {fno}"
        );

        // The four published tiers, held to the SAME number rather than to a
        // looser one for being smaller. `of_equity` probes all six tables on
        // every call, so the cost of a membership question is now the SUM of
        // these, and a tier that quietly needed twelve steps would raise that
        // sum without changing any answer -- which is exactly the kind of
        // drift a bound stated as a number catches and a bound stated as
        // "small" does not.
        let n500 = worst_probe(&NIFTY_500_INDEX, &NIFTY_500);
        let n200 = worst_probe(&NIFTY_200_INDEX, &NIFTY_200);
        let n100 = worst_probe(&NIFTY_100_INDEX, &NIFTY_100);
        let n50 = worst_probe(&NIFTY_50_INDEX, &NIFTY_50);
        for (name, slots, worst) in [
            ("NIFTY 500", 2048, n500),
            ("NIFTY 200", 1024, n200),
            ("NIFTY 100", 512, n100),
            ("NIFTY 50", 256, n50),
        ] {
            assert!(
                worst <= 8,
                "{name} in {slots} slots must probe at most 8 times, got {worst}"
            );
        }
        println!(
            "worst probe: NTM {ntm}, FNO {fno}, 500 {n500}, 200 {n200}, \
             100 {n100}, 50 {n50}"
        );
    }

    #[test]
    fn the_hash_is_deterministic_and_the_tables_stay_half_empty() {
        // Determinism: the same symbol must hash the same way in every process,
        // or a stored result and a fresh one would disagree (§3 rule 5).
        assert_eq!(fnv1a("RELIANCE"), fnv1a("RELIANCE"));
        assert_ne!(fnv1a("RELIANCE"), fnv1a("RELIANCF"));
        assert_ne!(fnv1a(""), fnv1a("A"));
        // The empty string still hashes -- the FNV offset basis, not zero.
        assert_ne!(fnv1a(""), 0);

        // Half-empty is what bounds the probe. `build` asserts this at COMPILE
        // time; asserted again here so the reason is visible where the property
        // is used rather than only where it is enforced.
        assert!(NIFTY_TOTAL_MARKET.len() * 2 <= 2048);
        assert!(FNO_UNDERLYINGS.len() * 4 <= 1024);
        // The four appended tiers are held to a QUARTER rather than the half
        // `build` asserts. Half is what makes the probe TERMINATE; a quarter
        // is what the measured bound of 8 was taken at, and the NIFTY 500 at
        // 1024 slots probed 13 times to prove the two are different bounds.
        assert!(NIFTY_500.len() * 4 <= 2048);
        assert!(NIFTY_200.len() * 4 <= 1024);
        assert!(NIFTY_100.len() * 4 <= 512);
        assert!(NIFTY_50.len() * 4 <= 256);

        // mask never exceeds the table.
        for h in [0, 1, u64::MAX, 0xcbf2_9ce4_8422_2325] {
            assert!(mask(h, 2048) < 2048);
            assert!(mask(h, 1024) < 1024);
            assert!(mask(h, 512) < 512);
            assert!(mask(h, 256) < 256);
        }
    }

    #[test]
    fn both_lists_are_sorted_and_unique_so_binary_search_is_valid() {
        // binary_search on an unsorted array returns garbage silently.
        for (name, list) in [
            ("NIFTY_TOTAL_MARKET", NIFTY_TOTAL_MARKET.as_slice()),
            ("FNO_UNDERLYINGS", FNO_UNDERLYINGS.as_slice()),
            ("NIFTY_500", NIFTY_500.as_slice()),
            ("NIFTY_200", NIFTY_200.as_slice()),
            ("NIFTY_100", NIFTY_100.as_slice()),
            ("NIFTY_50", NIFTY_50.as_slice()),
        ] {
            for w in list.windows(2) {
                assert!(w[0] < w[1], "{name} is not sorted or not unique at {w:?}");
            }
        }
    }

    #[test]
    fn the_counts_are_the_measured_ones() {
        assert_eq!(NIFTY_TOTAL_MARKET.len(), 750, "niftyindices.com states 750");
        assert_eq!(FNO_UNDERLYINGS.len(), 213, "both masters name 213, exactly");
        // A tier whose count does not match its name is the whole reason this
        // change exists: the pages sliced an alphabetical list and called the
        // first fifty rows the NIFTY 50. The count is the cheapest check that
        // the list came from the file it claims.
        assert_eq!(NIFTY_500.len(), 500, "NSE publishes 500 rows");
        assert_eq!(NIFTY_200.len(), 200, "NSE publishes 200 rows");
        assert_eq!(NIFTY_100.len(), 100, "NSE publishes 100 rows");
        assert_eq!(NIFTY_50.len(), 50, "NSE publishes 50 rows");
        for (name, list) in [
            ("FNO_UNDERLYINGS", FNO_UNDERLYINGS.as_slice()),
            ("NIFTY_TOTAL_MARKET", NIFTY_TOTAL_MARKET.as_slice()),
            ("NIFTY_500", NIFTY_500.as_slice()),
            ("NIFTY_200", NIFTY_200.as_slice()),
            ("NIFTY_100", NIFTY_100.as_slice()),
            ("NIFTY_50", NIFTY_50.as_slice()),
        ] {
            assert!(
                list.iter().all(|s| !s.contains("NSETEST")),
                "{name}: exchange test instruments must never be a universe member"
            );
            // NSE's own files carry placeholder scrips during a corporate
            // action -- `DUMMYINXGN` and `DUMMYTRVN` sit in the published
            // Total Market and Microcap 250 files today, wearing pseudo-ISINs
            // that begin `DUM` instead of `INE`. They are not constituents,
            // and D-0089 records why they were dropped rather than
            // transcribed. This asserts the drop instead of trusting it.
            assert!(
                list.iter().all(|s| !s.starts_with("DUMMY")),
                "{name}: an NSE placeholder scrip is not a constituent"
            );
        }
    }

    #[test]
    fn the_published_tiers_nest_one_inside_the_next() {
        // THE INVARIANT `of_equity` RESTS ON, and the reason it is allowed to
        // set several bits for one symbol without any of them being invented.
        //
        // Asserted symbol by symbol rather than by counting: 50 <= 100 <= 200
        // <= 500 <= 750 holds for four disjoint lists too, and a count proves
        // nothing about membership. Every containment is checked in the
        // direction that can fail -- narrow inside wide -- and the failure
        // message names the symbol, because a rebalance breaks nesting for one
        // name at a time.
        for (inner_name, inner, outer_name, outer) in [
            (
                "NIFTY 50",
                NIFTY_50.as_slice(),
                "NIFTY 100",
                &NIFTY_100_INDEX as &dyn Probe,
            ),
            (
                "NIFTY 100",
                NIFTY_100.as_slice(),
                "NIFTY 200",
                &NIFTY_200_INDEX as &dyn Probe,
            ),
            (
                "NIFTY 200",
                NIFTY_200.as_slice(),
                "NIFTY 500",
                &NIFTY_500_INDEX as &dyn Probe,
            ),
            (
                "NIFTY 500",
                NIFTY_500.as_slice(),
                "Total Market",
                &NTM_INDEX as &dyn Probe,
            ),
        ] {
            for m in inner {
                assert!(
                    outer.holds(m),
                    "{m} is in the {inner_name} and not in the {outer_name} — \
                     the tiers no longer nest, so `of_equity` would be setting \
                     a bit no published file states"
                );
            }
        }

        // And the same fact stated through the public surface, because that is
        // where a caller reads it: the bits a NIFTY 50 name carries must be a
        // superset of the bits every wider tier carries.
        for m in NIFTY_50 {
            let u = of_equity(m);
            assert!(
                u.contains(Universe::NIFTY_50)
                    && u.contains(Universe::NIFTY_100)
                    && u.contains(Universe::NIFTY_200)
                    && u.contains(Universe::NIFTY_500)
                    && u.contains(Universe::TOTAL_MARKET),
                "{m} is a NIFTY 50 name and must carry every containing bit, got {:#b}",
                u.bits()
            );
        }
        for m in NIFTY_500 {
            let u = of_equity(m);
            assert!(
                u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
                "{m} is a NIFTY 500 name and must also be a Total Market one"
            );
        }

        // The converse must NOT hold, or the bits carry no information: a
        // Total Market name outside the 500 exists, and it must say so.
        let outside = NIFTY_TOTAL_MARKET
            .iter()
            .find(|s| !NIFTY_500_INDEX.contains(s))
            .expect("the Total Market is wider than the NIFTY 500");
        let u = of_equity(outside);
        assert!(u.contains(Universe::TOTAL_MARKET));
        assert!(
            !u.contains(Universe::NIFTY_500),
            "{outside} is outside the NIFTY 500 and must not claim the bit"
        );
    }

    /// One method, so the nesting table above can hold tables of four
    /// different sizes in one array.
    ///
    /// `MemberIndex<N>` is generic over the slot count, so `MemberIndex<256>`
    /// and `MemberIndex<2048>` are different types and cannot sit in the same
    /// slice without one. The alternative is four copies of the same loop,
    /// which is four places for a future tier to be forgotten.
    trait Probe {
        fn holds(&self, symbol: &str) -> bool;
    }

    impl<const N: usize> Probe for MemberIndex<N> {
        fn holds(&self, symbol: &str) -> bool {
            self.contains(symbol)
        }
    }

    #[test]
    fn the_three_original_bits_never_moved() {
        // `CLAUDE.md` §3 rule 8 as an assertion rather than a promise. These
        // three values are stamped onto merged rows and rendered on the page;
        // if D-0089 had inserted a tier at bit 2 instead of appending at bit
        // 3, every one of those would silently mean something else.
        assert_eq!(Universe::INDEX.bits(), 1);
        assert_eq!(Universe::FNO.bits(), 2);
        assert_eq!(Universe::TOTAL_MARKET.bits(), 4);
        // And the appended four, pinned so a later insertion cannot renumber
        // them either.
        assert_eq!(Universe::NIFTY_500.bits(), 8);
        assert_eq!(Universe::NIFTY_200.bits(), 16);
        assert_eq!(Universe::NIFTY_100.bits(), 32);
        assert_eq!(Universe::NIFTY_50.bits(), 64);
        assert_eq!(Universe::NONE.bits(), 0);
    }

    #[test]
    fn no_measured_sme_ticker_belongs_to_either_universe() {
        // THE CLAIM `Skip::SmeBoard` DECLINES 1,117 REAL SHARES ON. It was a
        // sentence in a comment about a module nothing consulted; here it is a
        // check. Every ticker below is verbatim from an `SM` or `ST` row of a
        // real master -- they are the 22 rows whose series the two vendors
        // disagree about, so they are also the SME rows most likely to be
        // mis-boarded, plus two ordinary ones.
        for sme in [
            "DRONE",
            "SATKARTAR",
            "SOCL",
            "WHITEFORCE",
            "VLINFRA",
            "SUNLITE",
            "ARVINDPORT",
            "YAAP",
            "MOXSH",
            "INVICTA",
            "ARCIIL",
            "RITEZONE",
            "AILIMITED",
            "ACCORD",
        ] {
            assert!(
                of_equity(sme).is_none(),
                "{sme} is an SME listing and is in neither universe"
            );
        }
    }

    #[test]
    fn an_index_is_its_own_universe_and_a_live_derivative_is_in_none() {
        use crate::instrument::{Exchange, Expiry, OptionSide, Segment};
        use crate::price::Paisa;
        use crate::symbol::Symbol;

        let nifty = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("valid");
        let u = of_instrument(&nifty);
        assert!(u.contains(Universe::INDEX));
        assert!(
            u.contains(Universe::FNO),
            "NIFTY is also the underlying of its own options"
        );
        assert!(
            !u.contains(Universe::TOTAL_MARKET),
            "an index is not a share"
        );

        let sensex = InstrumentKey::index(Exchange::Nse, "NOTANINDEXWEKNOW").expect("valid");
        assert_eq!(of_instrument(&sensex), Universe::INDEX);

        let share = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Cash,
            underlying: Symbol::new("RELIANCE").expect("valid"),
            kind: Kind::Equity,
        };
        let r = of_instrument(&share);
        assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
        assert!(!r.contains(Universe::INDEX));

        // A derivative is never stored, so it is never a member of anything --
        // not even of the universe its own underlying belongs to.
        let fut = InstrumentKey {
            segment: Segment::Fno,
            kind: Kind::Future {
                expiry: Expiry::new(2026, 8, 27).expect("valid"),
            },
            ..share
        };
        assert_eq!(of_instrument(&fut), Universe::NONE);
        let opt = InstrumentKey {
            kind: Kind::Option {
                expiry: Expiry::new(2026, 8, 27).expect("valid"),
                strike: Paisa::from_raw(1_945_000),
                side: OptionSide::Call,
            },
            ..fut
        };
        assert_eq!(of_instrument(&opt), Universe::NONE);
    }

    #[test]
    fn membership_is_a_set_not_a_category() {
        // RELIANCE is both. The whole point of a bitset.
        let r = of_equity("RELIANCE");
        assert!(r.contains(Universe::FNO));
        assert!(r.contains(Universe::TOTAL_MARKET));
        assert!(!r.is_none());
    }

    #[test]
    fn a_symbol_in_neither_belongs_to_nothing() {
        let n = of_equity("NOT_A_REAL_SYMBOL");
        assert!(n.is_none());
        assert_eq!(n, Universe::NONE);
        assert_eq!(n.bits(), 0);
    }

    #[test]
    fn contains_is_a_superset_test_not_equality() {
        let both = Universe::FNO.union(Universe::TOTAL_MARKET);
        assert!(both.contains(Universe::FNO));
        assert!(both.contains(Universe::TOTAL_MARKET));
        assert!(both.contains(both));
        assert!(
            !Universe::FNO.contains(both),
            "one bit does not contain two"
        );
        assert!(Universe::NONE.contains(Universe::NONE));
        assert!(!Universe::NONE.contains(Universe::INDEX));
    }

    #[test]
    fn bits_are_distinct_powers_of_two() {
        // An append-only bitset is only safe if no two universes share a bit.
        let all = [
            Universe::INDEX,
            Universe::FNO,
            Universe::TOTAL_MARKET,
            Universe::NIFTY_500,
            Universe::NIFTY_200,
            Universe::NIFTY_100,
            Universe::NIFTY_50,
        ];
        for (i, a) in all.iter().enumerate() {
            assert_eq!(a.bits().count_ones(), 1, "universe {i} is not a single bit");
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert_eq!(a.bits() & b.bits(), 0, "universes {i} and {j} share a bit");
                }
            }
        }
    }
}
