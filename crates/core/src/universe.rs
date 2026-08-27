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
//! table this file builds at compile time. **Two bounds, not one.** A HIT
//! stops at the slot holding the symbol; a MISS cannot, and runs on to the
//! first EMPTY slot. The miss is therefore never the shorter walk, and it is
//! measured separately rather than assumed to fit under the hit's number:
//!
//! | table | members | slots | worst hit | worst miss |
//! |---|---|---|---|---|
//! | `NTM_INDEX` | 750 | 2048 | 6 | 11 |
//! | `FNO_INDEX` | 213 | 1024 | 7 | 10 |
//! | `NIFTY_500_INDEX` | 500 | 2048 | 5 | 8 |
//! | `NIFTY_200_INDEX` | 200 | 1024 | 6 | 9 |
//! | `NIFTY_100_INDEX` | 100 | 512 | 5 | 7 |
//! | `NIFTY_50_INDEX` | 50 | 256 | 3 | 9 |
//!
//! The hit column is asserted at `<= 8` by
//! `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1` and
//! the miss column at `<= 12` by
//! `core::universe::a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`,
//! each walking the table the way `contains` does and counting the steps. The
//! miss column is not a sample of unlucky strings: it is the longest run of
//! occupied slots in each table plus one, taken over every slot, which is the
//! worst case over EVERY string that can miss whatever it hashes to.
//!
//! **This section carried the hit column alone and called it "the worst
//! probe".** [`of_equity`] is the caller, and the question it is asked most is
//! about a name outside every tier — an SME listing, a BSE-only share — which
//! is a MISS in all six tables and so the case the stated bound did not cover.
//! Six misses is at most **54** steps measured (72 by the asserted per-table
//! bound), not the 32 the hit column sums to; back when there were two tables
//! it was 21 rather than the 13 this paragraph used to quote. None of those
//! numbers moves when a list grows within its table, so it is still the
//! CONSTANT `CLAUDE.md` §3 rule 4 asks for — 54 is just the honest constant
//! and 32 was the flattering one. The four appended tables came in at D-0089;
//! sizing them the way [`MemberIndex`] merely *permits* rather than the way its
//! bound was *measured* put the NIFTY 500 at 13 hit steps, and
//! [`NIFTY_500_INDEX`] records why they are a quarter full instead of half.
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
//!
//! # The exchange's ISIN column, and the hop it exists to remove
//!
//! D-0089 took the `Symbol` column of those files and discarded the `ISIN
//! Code` column beside it, so nothing in this crate held an NSE-issued ISIN and
//! `api::constituents` — which keys correctly on `(exchange, ISIN)` — had to
//! resolve a constituent's identity through the merged VENDOR universe by
//! symbol first. `docs/06-limits.md` §62 records that hop.
//!
//! D-0122 carries the column. Six `*_ISIN` arrays sit beside the six name
//! arrays, positionally aligned, each naming the published file it came from by
//! byte size and SHA-256; [`nse_isin`] reads them in constant time. **1,807 of
//! 1,813 positions carry one.** The six that do not are five indices, which are
//! issued no ISIN, and `AGL`, which the exchange's own file has no row for —
//! named, not counted, by `the_absent_isins_are_exactly_these_six_names`.
//!
//! No broker master was opened for any value here. Both carry an ISIN column
//! and neither was consulted: taking a missing identifier from a vendor would
//! have reintroduced, inside the data, the exact vendor hop this removes from
//! the join.
//!
//! **The last sentence of this paragraph used to say the wiring was not done.**
//! It read *"Wiring `api::constituents` to `nse_isin` is deliberately NOT part
//! of that change — `crates/api` was held by another session — so the join
//! still hops until it lands."* That was true when written and stopped being
//! true when D-0125 landed: `api::constituents::nse_identity_of` calls
//! [`nse_isin`] directly and that module's own header now opens **"THE KEY IS
//! NSE'S OWN ISIN, AT BOTH ENDS. THERE IS NO SYMBOL STEP."** A note saying the
//! join still hops, sitting above the table the join reads, is the worst
//! possible place for a stale claim — it tells a reader auditing the identity
//! chain that the weak link is still there.
//!
//! Verified against the operator's own masters, 2026-08-27: `RELIANCE` resolves
//! to `INE002A01018` here, and that is the ISIN in Dhan's master beside
//! security id `2885` and in Groww's beside `RELIANCE`. **Zerodha publishes no
//! ISIN column at all**, which is why it cannot be ISIN-joined and why
//! `pull::universe::Verdict::VendorHasNoIsin` exists as a bucket separate from
//! `Lacks` — reporting it as a lack would blame a vendor for a cell the
//! exchange never asked it to fill.

use crate::instrument::{InstrumentKey, Kind};
use crate::isin::Isin;

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

/// What an ISIN array carries where the exchange's own file has no row for
/// that name.
///
/// Empty, and deliberately **not** a plausible-looking placeholder:
/// [`crate::isin::Isin::new`] refuses it on length before the check digit is
/// ever reached, so an absent entry that leaks into a join is a refusal at the
/// boundary rather than an identifier that points at nothing. Inventing a
/// stand-in here is the `CLAUDE.md` §4 fallback that hides a failure, and
/// deriving one from a broker master is the `CLAUDE.md` §3 rule 1 invention
/// this whole transcription exists to remove.
///
/// Six of the 1,813 positions carry it. They are named in
/// `the_absent_isins_are_exactly_these_six_names`, which pins the set rather
/// than counting it.
pub const ISIN_ABSENT: &str = "";

/// NSE's own ISIN for each name of [`NIFTY_50`], at the same index.
///
/// Transcribed from `ind_nifty50list.csv` — 3,352 bytes, sha256
/// `9fb8832853c279448d2bc05f0e7dd5f460ed2ff35332fea8c40fc1250362ad28`,
/// 50 data rows — taking the file's `ISIN Code` column and joining it to this
/// array on the file's own `Symbol` column, which is the column [`NIFTY_50`]
/// itself was transcribed from at D-0089. **50 of 50 carry one.**
///
/// The trailing comment on each row is the symbol it belongs to. It is not
/// decoration: positional alignment is the entire correctness claim of this
/// array, and a reviewer diffing it against the exchange's file needs both
/// columns on the same line to check it by eye. The machine check is
/// `the_isin_arrays_are_positionally_aligned_with_the_names`.
pub const NIFTY_50_ISIN: [&str; 50] = [
    "INE423A01024", // ADANIENT
    "INE742F01042", // ADANIPORTS
    "INE437A01024", // APOLLOHOSP
    "INE021A01026", // ASIANPAINT
    "INE238A01034", // AXISBANK
    "INE917I01010", // BAJAJ-AUTO
    "INE918I01026", // BAJAJFINSV
    "INE296A01032", // BAJFINANCE
    "INE263A01024", // BEL
    "INE397D01024", // BHARTIARTL
    "INE059A01026", // CIPLA
    "INE522F01014", // COALINDIA
    "INE089A01031", // DRREDDY
    "INE066A01021", // EICHERMOT
    "INE758T01015", // ETERNAL
    "INE047A01021", // GRASIM
    "INE860A01027", // HCLTECH
    "INE040A01034", // HDFCBANK
    "INE795G01014", // HDFCLIFE
    "INE038A01020", // HINDALCO
    "INE030A01027", // HINDUNILVR
    "INE090A01021", // ICICIBANK
    "INE646L01027", // INDIGO
    "INE009A01021", // INFY
    "INE154A01025", // ITC
    "INE758E01017", // JIOFIN
    "INE019A01038", // JSWSTEEL
    "INE237A01036", // KOTAKBANK
    "INE018A01030", // LT
    "INE101A01026", // M&M
    "INE585B01010", // MARUTI
    "INE027H01010", // MAXHEALTH
    "INE239A01024", // NESTLEIND
    "INE733E01010", // NTPC
    "INE213A01029", // ONGC
    "INE752E01010", // POWERGRID
    "INE002A01018", // RELIANCE
    "INE123W01016", // SBILIFE
    "INE062A01020", // SBIN
    "INE721A01047", // SHRIRAMFIN
    "INE044A01036", // SUNPHARMA
    "INE192A01025", // TATACONSUM
    "INE081A01020", // TATASTEEL
    "INE467B01029", // TCS
    "INE669C01036", // TECHM
    "INE280A01028", // TITAN
    "INE155A01022", // TMPV
    "INE849A01020", // TRENT
    "INE481G01011", // ULTRACEMCO
    "INE075A01022", // WIPRO
];

/// NSE's own ISIN for each name of [`NIFTY_100`], at the same index.
///
/// Transcribed from `ind_nifty100list.csv` — 6,611 bytes, sha256
/// `1a40e33a0febf458986a178bc76f7b0051f163718f2a8bc11a726ba70a39c0a9`,
/// 100 data rows — taking the file's `ISIN Code` column and joining it to this
/// array on the file's own `Symbol` column, which is the column [`NIFTY_100`]
/// itself was transcribed from at D-0089. **100 of 100 carry one.**
///
/// The trailing comment on each row is the symbol it belongs to. It is not
/// decoration: positional alignment is the entire correctness claim of this
/// array, and a reviewer diffing it against the exchange's file needs both
/// columns on the same line to check it by eye. The machine check is
/// `the_isin_arrays_are_positionally_aligned_with_the_names`.
pub const NIFTY_100_ISIN: [&str; 100] = [
    "INE117A01022", // ABB
    "INE931S01010", // ADANIENSOL
    "INE423A01024", // ADANIENT
    "INE364U01010", // ADANIGREEN
    "INE742F01042", // ADANIPORTS
    "INE814H01029", // ADANIPOWER
    "INE079A01024", // AMBUJACEM
    "INE437A01024", // APOLLOHOSP
    "INE021A01026", // ASIANPAINT
    "INE238A01034", // AXISBANK
    "INE917I01010", // BAJAJ-AUTO
    "INE918I01026", // BAJAJFINSV
    "INE118A01012", // BAJAJHLDNG
    "INE296A01032", // BAJFINANCE
    "INE028A01039", // BANKBARODA
    "INE263A01024", // BEL
    "INE397D01024", // BHARTIARTL
    "INE323A01026", // BOSCHLTD
    "INE029A01011", // BPCL
    "INE216A01030", // BRITANNIA
    "INE476A01022", // CANBK
    "INE067A01029", // CGPOWER
    "INE121A01024", // CHOLAFIN
    "INE059A01026", // CIPLA
    "INE522F01014", // COALINDIA
    "INE298A01020", // CUMMINSIND
    "INE361B01024", // DIVISLAB
    "INE271C01023", // DLF
    "INE192R01011", // DMART
    "INE089A01031", // DRREDDY
    "INE066A01021", // EICHERMOT
    "INE1NPP01017", // ENRIN
    "INE758T01015", // ETERNAL
    "INE129A01019", // GAIL
    "INE102D01028", // GODREJCP
    "INE047A01021", // GRASIM
    "INE066F01020", // HAL
    "INE860A01027", // HCLTECH
    "INE127D01025", // HDFCAMC
    "INE040A01034", // HDFCBANK
    "INE795G01014", // HDFCLIFE
    "INE038A01020", // HINDALCO
    "INE030A01027", // HINDUNILVR
    "INE267A01025", // HINDZINC
    "INE0V6F01027", // HYUNDAI
    "INE090A01021", // ICICIBANK
    "INE053A01029", // INDHOTEL
    "INE646L01027", // INDIGO
    "INE009A01021", // INFY
    "INE242A01010", // IOC
    "INE053F01010", // IRFC
    "INE154A01025", // ITC
    "INE749A01030", // JINDALSTEL
    "INE758E01017", // JIOFIN
    "INE019A01038", // JSWSTEEL
    "INE237A01036", // KOTAKBANK
    "INE670K01029", // LODHA
    "INE018A01030", // LT
    "INE214T01019", // LTM
    "INE101A01026", // M&M
    "INE585B01010", // MARUTI
    "INE027H01010", // MAXHEALTH
    "INE249Z01020", // MAZDOCK
    "INE775A01035", // MOTHERSON
    "INE414G01012", // MUTHOOTFIN
    "INE239A01024", // NESTLEIND
    "INE733E01010", // NTPC
    "INE213A01029", // ONGC
    "INE134E01011", // PFC
    "INE318A01026", // PIDILITIND
    "INE160A01022", // PNB
    "INE752E01010", // POWERGRID
    "INE020B01018", // RECLTD
    "INE002A01018", // RELIANCE
    "INE123W01016", // SBILIFE
    "INE062A01020", // SBIN
    "INE070A01015", // SHREECEM
    "INE721A01047", // SHRIRAMFIN
    "INE003A01024", // SIEMENS
    "INE343H01029", // SOLARINDS
    "INE044A01036", // SUNPHARMA
    "INE976I01016", // TATACAP
    "INE192A01025", // TATACONSUM
    "INE245A01021", // TATAPOWER
    "INE081A01020", // TATASTEEL
    "INE467B01029", // TCS
    "INE669C01036", // TECHM
    "INE280A01028", // TITAN
    "INE1TAE01010", // TMCV
    "INE155A01022", // TMPV
    "INE685A01028", // TORNTPHARM
    "INE849A01020", // TRENT
    "INE494B01023", // TVSMOTOR
    "INE481G01011", // ULTRACEMCO
    "INE692A01016", // UNIONBANK
    "INE854D01024", // UNITDSPR
    "INE200M01039", // VBL
    "INE205A01025", // VEDL
    "INE075A01022", // WIPRO
    "INE010B01027", // ZYDUSLIFE
];

/// NSE's own ISIN for each name of [`NIFTY_200`], at the same index.
///
/// Transcribed from `ind_nifty200list.csv` — 13,081 bytes, sha256
/// `76b8b127931953ce7e5e5511c99c3b73775140eeb83b6b293085b4a9483dce1a`,
/// 200 data rows — taking the file's `ISIN Code` column and joining it to this
/// array on the file's own `Symbol` column, which is the column [`NIFTY_200`]
/// itself was transcribed from at D-0089. **200 of 200 carry one.**
///
/// The trailing comment on each row is the symbol it belongs to. It is not
/// decoration: positional alignment is the entire correctness claim of this
/// array, and a reviewer diffing it against the exchange's file needs both
/// columns on the same line to check it by eye. The machine check is
/// `the_isin_arrays_are_positionally_aligned_with_the_names`.
pub const NIFTY_200_ISIN: [&str; 200] = [
    "INE466L01038", // 360ONE
    "INE117A01022", // ABB
    "INE674K01013", // ABCAPITAL
    "INE931S01010", // ADANIENSOL
    "INE423A01024", // ADANIENT
    "INE364U01010", // ADANIGREEN
    "INE742F01042", // ADANIPORTS
    "INE814H01029", // ADANIPOWER
    "INE540L01014", // ALKEM
    "INE079A01024", // AMBUJACEM
    "INE702C01027", // APLAPOLLO
    "INE437A01024", // APOLLOHOSP
    "INE208A01029", // ASHOKLEY
    "INE021A01026", // ASIANPAINT
    "INE006I01046", // ASTRAL
    "INE399L01023", // ATGL
    "INE949L01017", // AUBANK
    "INE406A01037", // AUROPHARMA
    "INE238A01034", // AXISBANK
    "INE917I01010", // BAJAJ-AUTO
    "INE918I01026", // BAJAJFINSV
    "INE118A01012", // BAJAJHLDNG
    "INE296A01032", // BAJFINANCE
    "INE028A01039", // BANKBARODA
    "INE084A01016", // BANKINDIA
    "INE171Z01026", // BDL
    "INE263A01024", // BEL
    "INE465A01025", // BHARATFORG
    "INE397D01024", // BHARTIARTL
    "INE257A01026", // BHEL
    "INE376G01013", // BIOCON
    "INE472A01039", // BLUESTARCO
    "INE323A01026", // BOSCHLTD
    "INE029A01011", // BPCL
    "INE216A01030", // BRITANNIA
    "INE118H01025", // BSE
    "INE476A01022", // CANBK
    "INE067A01029", // CGPOWER
    "INE121A01024", // CHOLAFIN
    "INE059A01026", // CIPLA
    "INE522F01014", // COALINDIA
    "INE704P01025", // COCHINSHIP
    "INE591G01025", // COFORGE
    "INE259A01022", // COLPAL
    "INE111A01025", // CONCOR
    "INE169A01031", // COROMANDEL
    "INE298A01020", // CUMMINSIND
    "INE016A01026", // DABUR
    "INE361B01024", // DIVISLAB
    "INE935N01020", // DIXON
    "INE271C01023", // DLF
    "INE192R01011", // DMART
    "INE089A01031", // DRREDDY
    "INE066A01021", // EICHERMOT
    "INE1NPP01017", // ENRIN
    "INE758T01015", // ETERNAL
    "INE302A01020", // EXIDEIND
    "INE171A01029", // FEDERALBNK
    "INE061F01013", // FORTIS
    "INE129A01019", // GAIL
    "INE935A01035", // GLENMARK
    "INE776C01039", // GMRAIRPORT
    "INE260B01028", // GODFRYPHLP
    "INE102D01028", // GODREJCP
    "INE484J01027", // GODREJPROP
    "INE047A01021", // GRASIM
    "INE0HOQ01053", // GROWW
    "INE200A01026", // GVT&D
    "INE066F01020", // HAL
    "INE176B01034", // HAVELLS
    "INE860A01027", // HCLTECH
    "INE127D01025", // HDFCAMC
    "INE040A01034", // HDFCBANK
    "INE795G01014", // HDFCLIFE
    "INE158A01026", // HEROMOTOCO
    "INE038A01020", // HINDALCO
    "INE094A01015", // HINDPETRO
    "INE030A01027", // HINDUNILVR
    "INE267A01025", // HINDZINC
    "INE031A01017", // HUDCO
    "INE0V6F01027", // HYUNDAI
    "INE346A01027", // ICICIAMC
    "INE090A01021", // ICICIBANK
    "INE765G01017", // ICICIGI
    "INE669E01016", // IDEA
    "INE092T01019", // IDFCFIRSTB
    "INE053A01029", // INDHOTEL
    "INE562A01011", // INDIANB
    "INE646L01027", // INDIGO
    "INE095A01012", // INDUSINDBK
    "INE121J01017", // INDUSTOWER
    "INE009A01021", // INFY
    "INE242A01010", // IOC
    "INE335Y01020", // IRCTC
    "INE202E01016", // IREDA
    "INE053F01010", // IRFC
    "INE154A01025", // ITC
    "INE749A01030", // JINDALSTEL
    "INE758E01017", // JIOFIN
    "INE121E01018", // JSWENERGY
    "INE019A01038", // JSWSTEEL
    "INE797F01020", // JUBLFOOD
    "INE303R01014", // KALYANKJIL
    "INE878B01027", // KEI
    "INE237A01036", // KOTAKBANK
    "INE04I401011", // KPITTECH
    "INE947Q01028", // LAURUSLABS
    "INE956O01016", // LENSKART
    "INE324D01010", // LGEINDIA
    "INE115A01026", // LICHSGFIN
    "INE670K01029", // LODHA
    "INE018A01030", // LT
    "INE498L01015", // LTF
    "INE214T01019", // LTM
    "INE326A01037", // LUPIN
    "INE101A01026", // M&M
    "INE774D01024", // M&MFIN
    "INE634S01028", // MANKIND
    "INE196A01026", // MARICO
    "INE585B01010", // MARUTI
    "INE027H01010", // MAXHEALTH
    "INE249Z01020", // MAZDOCK
    "INE745G01043", // MCX
    "INE180A01020", // MFSL
    "INE775A01035", // MOTHERSON
    "INE338I01027", // MOTILALOFS
    "INE356A01018", // MPHASIS
    "INE883A01011", // MRF
    "INE414G01012", // MUTHOOTFIN
    "INE139A01034", // NATIONALUM
    "INE663F01032", // NAUKRI
    "INE239A01024", // NESTLEIND
    "INE848E01016", // NHPC
    "INE584A01023", // NMDC
    "INE733E01010", // NTPC
    "INE388Y01029", // NYKAA
    "INE093I01010", // OBEROIRLTY
    "INE881D01027", // OFSS
    "INE274J01014", // OIL
    "INE213A01029", // ONGC
    "INE761H01022", // PAGEIND
    "INE619A01035", // PATANJALI
    "INE982J01020", // PAYTM
    "INE262H01021", // PERSISTENT
    "INE134E01011", // PFC
    "INE211B01039", // PHOENIXLTD
    "INE318A01026", // PIDILITIND
    "INE603J01030", // PIIND
    "INE160A01022", // PNB
    "INE417T01026", // POLICYBZR
    "INE455K01017", // POLYCAB
    "INE752E01010", // POWERGRID
    "INE07Y701011", // POWERINDIA
    "INE0BS701011", // PREMIERENE
    "INE811K01011", // PRESTIGE
    "INE944F01028", // RADICO
    "INE020B01018", // RECLTD
    "INE002A01018", // RELIANCE
    "INE415G01027", // RVNL
    "INE114A01011", // SAIL
    "INE018E01016", // SBICARD
    "INE123W01016", // SBILIFE
    "INE062A01020", // SBIN
    "INE070A01015", // SHREECEM
    "INE721A01047", // SHRIRAMFIN
    "INE003A01024", // SIEMENS
    "INE343H01029", // SOLARINDS
    "INE647A01010", // SRF
    "INE044A01036", // SUNPHARMA
    "INE195A01028", // SUPREMEIND
    "INE040H01021", // SUZLON
    "INE00H001014", // SWIGGY
    "INE976I01016", // TATACAP
    "INE151A01013", // TATACOMM
    "INE192A01025", // TATACONSUM
    "INE670A01012", // TATAELXSI
    "INE672A01026", // TATAINVEST
    "INE245A01021", // TATAPOWER
    "INE081A01020", // TATASTEEL
    "INE467B01029", // TCS
    "INE669C01036", // TECHM
    "INE974X01010", // TIINDIA
    "INE280A01028", // TITAN
    "INE1TAE01010", // TMCV
    "INE155A01022", // TMPV
    "INE685A01028", // TORNTPHARM
    "INE849A01020", // TRENT
    "INE494B01023", // TVSMOTOR
    "INE481G01011", // ULTRACEMCO
    "INE692A01016", // UNIONBANK
    "INE854D01024", // UNITDSPR
    "INE628A01036", // UPL
    "INE200M01039", // VBL
    "INE205A01025", // VEDL
    "INE01EA01019", // VMM
    "INE226A01021", // VOLTAS
    "INE377N01017", // WAAREEENER
    "INE075A01022", // WIPRO
    "INE528G01035", // YESBANK
    "INE010B01027", // ZYDUSLIFE
];

/// NSE's own ISIN for each name of [`NIFTY_500`], at the same index.
///
/// Transcribed from `ind_nifty500list.csv` — 32,766 bytes, sha256
/// `637b99dc20a36a994b8dd43ae8449781258a9c94fab20ca3b87741fb39bd67db`,
/// 500 data rows — taking the file's `ISIN Code` column and joining it to this
/// array on the file's own `Symbol` column, which is the column [`NIFTY_500`]
/// itself was transcribed from at D-0089. **500 of 500 carry one.**
///
/// The trailing comment on each row is the symbol it belongs to. It is not
/// decoration: positional alignment is the entire correctness claim of this
/// array, and a reviewer diffing it against the exchange's file needs both
/// columns on the same line to check it by eye. The machine check is
/// `the_isin_arrays_are_positionally_aligned_with_the_names`.
pub const NIFTY_500_ISIN: [&str; 500] = [
    "INE466L01038", // 360ONE
    "INE470A01017", // 3MINDIA
    "INE883F01010", // AADHARHFC
    "INE769A01020", // AARTIIND
    "INE216P01012", // AAVAS
    "INE117A01022", // ABB
    "INE358A01014", // ABBOTINDIA
    "INE674K01013", // ABCAPITAL
    "INE552Z01027", // ABDL
    "INE647O01011", // ABFRL
    "INE14LE01019", // ABLBL
    "INE055A01016", // ABREL
    "INE404A01024", // ABSLAMC
    "INE012A01025", // ACC
    "INE731H01025", // ACE
    "INE622W01025", // ACMESOLAR
    "INE00FF01025", // ACUTAAS
    "INE931S01010", // ADANIENSOL
    "INE423A01024", // ADANIENT
    "INE364U01010", // ADANIGREEN
    "INE742F01042", // ADANIPORTS
    "INE814H01029", // ADANIPOWER
    "INE208C01025", // AEGISLOG
    "INE0INX01018", // AEGISVOPAK
    "INE101I01011", // AFCONS
    "INE00WC01027", // AFFLE
    "INE212H01026", // AIAENG
    "INE206F01022", // AIIL
    "INE031B01049", // AJANTPHARM
    "INE540L01014", // ALKEM
    "INE371P01015", // AMBER
    "INE079A01024", // AMBUJACEM
    "INE463V01026", // ANANDRATHI
    "INE242C01024", // ANANTRAJ
    "INE732I01021", // ANGELONE
    "INE0CZ201020", // ANTHEM
    "INE930P01018", // ANURAS
    "INE372A01015", // APARINDS
    "INE702C01027", // APLAPOLLO
    "INE437A01024", // APOLLOHOSP
    "INE438A01022", // APOLLOTYRE
    "INE852O01025", // APTUS
    "INE885A01032", // ARE&M
    "INE439A01020", // ASAHIINDIA
    "INE208A01029", // ASHOKLEY
    "INE021A01026", // ASIANPAINT
    "INE914M01019", // ASTERDM
    "INE006I01046", // ASTRAL
    "INE399L01023", // ATGL
    "INE0LEZ01016", // ATHERENERG
    "INE100A01010", // ATUL
    "INE949L01017", // AUBANK
    "INE406A01037", // AUROPHARMA
    "INE699H01024", // AWL
    "INE238A01034", // AXISBANK
    "INE917I01010", // BAJAJ-AUTO
    "INE918I01026", // BAJAJFINSV
    "INE377Y01014", // BAJAJHFL
    "INE118A01012", // BAJAJHLDNG
    "INE296A01032", // BAJFINANCE
    "INE787D01026", // BALKRISIND
    "INE119A01028", // BALRAMCHIN
    "INE545U01014", // BANDHANBNK
    "INE028A01039", // BANKBARODA
    "INE084A01016", // BANKINDIA
    "INE176A01028", // BATAINDIA
    "INE462A01022", // BAYERCROP
    "INE050A01025", // BBTC
    "INE171Z01026", // BDL
    "INE263A01024", // BEL
    "INE894V01022", // BELRISE
    "INE258A01024", // BEML
    "INE463A01038", // BERGEPAINT
    "INE465A01025", // BHARATFORG
    "INE397D01024", // BHARTIARTL
    "INE343G01021", // BHARTIHEXA
    "INE257A01026", // BHEL
    "INE00E101023", // BIKAJI
    "INE376G01013", // BIOCON
    "INE153T01027", // BLS
    "INE233B01017", // BLUEDART
    "INE0KBH01020", // BLUEJET
    "INE472A01039", // BLUESTARCO
    "INE323A01026", // BOSCHLTD
    "INE029A01011", // BPCL
    "INE791I01019", // BRIGADE
    "INE216A01030", // BRITANNIA
    "INE118H01025", // BSE
    "INE836A01035", // BSOFT
    "INE596I01020", // CAMS
    "INE476A01022", // CANBK
    "INE477A01020", // CANFINHOME
    "INE01TY01017", // CANHLIFE
    "INE475E01026", // CAPLIPOINT
    "INE120A01034", // CARBORUNIV
    "INE290S01011", // CARTRADE
    "INE172A01027", // CASTROLIND
    "INE421D01022", // CCL
    "INE736A01011", // CDSL
    "INE482A01020", // CEATLTD
    "INE686A01026", // CEMPRO
    "INE483A01010", // CENTRALBK
    "INE486A01021", // CESC
    "INE180C01042", // CGCL
    "INE067A01029", // CGPOWER
    "INE427F01016", // CHALET
    "INE085A01013", // CHAMBLFERT
    "INE178A01016", // CHENNPETRO
    "INE102B01014", // CHOICEIN
    "INE121A01024", // CHOLAFIN
    "INE149A01033", // CHOLAHLDNG
    "INE536H01010", // CIEINDIA
    "INE059A01026", // CIPLA
    "INE227W01023", // CLEAN
    "INE522F01014", // COALINDIA
    "INE704P01025", // COCHINSHIP
    "INE591G01025", // COFORGE
    "INE03QK01018", // COHANCE
    "INE259A01022", // COLPAL
    "INE111A01025", // CONCOR
    "INE338H01029", // CONCORDBIO
    "INE169A01031", // COROMANDEL
    "INE819V01029", // CPPLUS
    "INE00LO01017", // CRAFTSMAN
    "INE741K01010", // CREDITACC
    "INE007A01025", // CRISIL
    "INE299U01018", // CROMPTON
    "INE491A01021", // CUB
    "INE298A01020", // CUMMINSIND
    "INE136B01020", // CYIENT
    "INE016A01026", // DABUR
    "INE00R701025", // DALBHARAT
    "INE0IX101010", // DATAPATTNS
    "INE499A01024", // DCMSHRIRAM
    "INE501A01019", // DEEPAKFERT
    "INE288B01029", // DEEPAKNTR
    "INE148O01028", // DELHIVERY
    "INE872J01023", // DEVYANI
    "INE361B01024", // DIVISLAB
    "INE935N01020", // DIXON
    "INE271C01023", // DLF
    "INE192R01011", // DMART
    "INE321T01012", // DOMS
    "INE089A01031", // DRREDDY
    "INE738I01010", // ECLERX
    "INE066A01021", // EICHERMOT
    "INE126A01031", // EIDPARRY
    "INE230A01023", // EIHOTEL
    "INE205B01031", // ELECON
    "INE285A01027", // ELGIEQUIP
    "INE548C01032", // EMAMILTD
    "INE168P01015", // EMCURE
    "INE1C6T01020", // EMMVEE
    "INE913H01037", // ENDURANCE
    "INE510A01028", // ENGINERSIN
    "INE1NPP01017", // ENRIN
    "INE406M01024", // ERIS
    "INE042A01014", // ESCORTS
    "INE758T01015", // ETERNAL
    "INE302A01020", // EXIDEIND
    "INE188A01015", // FACT
    "INE171A01029", // FEDERALBNK
    "INE235A01022", // FINCABLES
    "INE02RE01045", // FIRSTCRY
    "INE128S01021", // FIVESTAR
    "INE09N301011", // FLUOROCHEM
    "INE451A01017", // FORCEMOT
    "INE061F01013", // FORTIS
    "INE684F01012", // FSL
    "INE524A01029", // GABRIEL
    "INE129A01019", // GAIL
    "INE297H01019", // GALLANTT
    "INE017A01032", // GESHIP
    "INE481Y01014", // GICRE
    "INE322A01010", // GILLETTE
    "INE068V01023", // GLAND
    "INE159A01016", // GLAXO
    "INE935A01035", // GLENMARK
    "INE131A01031", // GMDCLTD
    "INE776C01039", // GMRAIRPORT
    "INE260B01028", // GODFRYPHLP
    "INE03JT01014", // GODIGIT
    "INE102D01028", // GODREJCP
    "INE233A01035", // GODREJIND
    "INE484J01027", // GODREJPROP
    "INE177H01039", // GPIL
    "INE101D01020", // GRANULES
    "INE371A01025", // GRAPHITE
    "INE047A01021", // GRASIM
    "INE024L01027", // GRAVITA
    "INE0HOQ01053", // GROWW
    "INE382Z01011", // GRSE
    "INE200A01026", // GVT&D
    "INE066F01020", // HAL
    "INE176B01034", // HAVELLS
    "INE292B01021", // HBLENGINE
    "INE860A01027", // HCLTECH
    "INE756I01012", // HDBFS
    "INE127D01025", // HDFCAMC
    "INE040A01034", // HDFCBANK
    "INE795G01014", // HDFCLIFE
    "INE545A01024", // HEG
    "INE158A01026", // HEROMOTOCO
    "INE093A01041", // HEXT
    "INE548A01028", // HFCL
    "INE038A01020", // HINDALCO
    "INE531E01026", // HINDCOPPER
    "INE094A01015", // HINDPETRO
    "INE030A01027", // HINDUNILVR
    "INE267A01025", // HINDZINC
    "INE481N01025", // HOMEFIRST
    "INE0J5401028", // HONASA
    "INE671A01010", // HONAUT
    "INE019C01026", // HSCL
    "INE031A01017", // HUDCO
    "INE0V6F01027", // HYUNDAI
    "INE346A01027", // ICICIAMC
    "INE090A01021", // ICICIBANK
    "INE765G01017", // ICICIGI
    "INE726G01019", // ICICIPRULI
    "INE008A01015", // IDBI
    "INE669E01016", // IDEA
    "INE092T01019", // IDFCFIRSTB
    "INE022Q01020", // IEX
    "INE039A01010", // IFCI
    "INE0Q9301021", // IGIL
    "INE203G01027", // IGL
    "INE530B01024", // IIFL
    "INE115Q01022", // IKS
    "INE065X01017", // INDGN
    "INE053A01029", // INDHOTEL
    "INE383A01012", // INDIACEM
    "INE933S01016", // INDIAMART
    "INE562A01011", // INDIANB
    "INE646L01027", // INDIGO
    "INE095A01012", // INDUSINDBK
    "INE121J01017", // INDUSTOWER
    "INE009A01021", // INFY
    "INE066P01011", // INOXWIND
    "INE306R01017", // INTELLECT
    "INE565A01014", // IOB
    "INE242A01010", // IOC
    "INE571A01038", // IPCALAB
    "INE821I01022", // IRB
    "INE962Y01021", // IRCON
    "INE335Y01020", // IRCTC
    "INE202E01016", // IREDA
    "INE053F01010", // IRFC
    "INE154A01025", // ITC
    "INE379A01028", // ITCHOTELS
    "INE248A01017", // ITI
    "INE168A01041", // J&KBANK
    "INE0YD401026", // JAINREC
    "INE927D01051", // JBMA
    "INE324A01032", // JINDALSAW
    "INE749A01030", // JINDALSTEL
    "INE758E01017", // JIOFIN
    "INE823G01014", // JKCEMENT
    "INE573A01042", // JKTYRE
    "INE780C01023", // JMFINANCIL
    "INE351F01018", // JPPOWER
    "INE220G01021", // JSL
    "INE718I01012", // JSWCEMENT
    "INE133A01011", // JSWDULUX
    "INE121E01018", // JSWENERGY
    "INE880J01026", // JSWINFRA
    "INE019A01038", // JSWSTEEL
    "INE797F01020", // JUBLFOOD
    "INE0BY001018", // JUBLINGREA
    "INE700A01033", // JUBLPHARMA
    "INE209L01016", // JWL
    "INE980O01024", // JYOTICNC
    "INE217B01036", // KAJARIACER
    "INE303R01014", // KALYANKJIL
    "INE036D01028", // KARURVYSYA
    "INE918Z01012", // KAYNES
    "INE389H01022", // KEC
    "INE878B01027", // KEI
    "INE138Y01010", // KFINTECH
    "INE967H01025", // KIMS
    "INE146L01010", // KIRLOSENG
    "INE237A01036", // KOTAKBANK
    "INE220B01022", // KPIL
    "INE04I401011", // KPITTECH
    "INE930H01031", // KPRMILL
    "INE600L01024", // LALPATHLAB
    "INE0I7C01011", // LATENTVIEW
    "INE947Q01028", // LAURUSLABS
    "INE970X01018", // LEMONTREE
    "INE956O01016", // LENSKART
    "INE324D01010", // LGEINDIA
    "INE115A01026", // LICHSGFIN
    "INE0J1Y01017", // LICI
    "INE473A01011", // LINDEINDIA
    "INE281B01032", // LLOYDSME
    "INE670K01029", // LODHA
    "INE018A01030", // LT
    "INE498L01015", // LTF
    "INE818H01020", // LTFOODS
    "INE214T01019", // LTM
    "INE010V01017", // LTTS
    "INE326A01037", // LUPIN
    "INE101A01026", // M&M
    "INE774D01024", // M&MFIN
    "INE457A01014", // MAHABANK
    "INE522D01027", // MANAPPURAM
    "INE634S01028", // MANKIND
    "INE0BV301023", // MAPMYINDIA
    "INE196A01026", // MARICO
    "INE585B01010", // MARUTI
    "INE027H01010", // MAXHEALTH
    "INE249Z01020", // MAZDOCK
    "INE745G01043", // MCX
    "INE474Q01031", // MEDANTA
    "INE0VDM01015", // MEESHO
    "INE180A01020", // MFSL
    "INE002S01010", // MGL
    "INE842C01021", // MINDACORP
    "INE123F01029", // MMTC
    "INE775A01035", // MOTHERSON
    "INE338I01027", // MOTILALOFS
    "INE356A01018", // MPHASIS
    "INE883A01011", // MRF
    "INE103A01014", // MRPL
    "INE0FS801015", // MSUMI
    "INE414G01012", // MUTHOOTFIN
    "INE298J01013", // NAM-INDIA
    "INE987B01026", // NATCOPHARM
    "INE139A01034", // NATIONALUM
    "INE663F01032", // NAUKRI
    "INE725A01030", // NAVA
    "INE048G01026", // NAVINFLUOR
    "INE095N01031", // NBCC
    "INE868B01028", // NCC
    "INE239A01024", // NESTLEIND
    "INE0NT901020", // NETWEB
    "INE794A01010", // NEULANDLAB
    "INE619B01017", // NEWGEN
    "INE410P01011", // NH
    "INE848E01016", // NHPC
    "INE470Y01017", // NIACL
    "INE995S01015", // NIVABUPA
    "INE589A01014", // NLCINDIA
    "INE584A01023", // NMDC
    "INE0NNS01018", // NSLNISP
    "INE733E01010", // NTPC
    "INE0ONG01011", // NTPCGREEN
    "INE531F01023", // NUVAMA
    "INE118D01016", // NUVOCO
    "INE388Y01029", // NYKAA
    "INE093I01010", // OBEROIRLTY
    "INE881D01027", // OFSS
    "INE274J01014", // OIL
    "INE0LXG01040", // OLAELEC
    "INE260D01016", // OLECTRA
    "INE013P01021", // ONESOURCE
    "INE213A01029", // ONGC
    "INE761H01022", // PAGEIND
    "INE088F01024", // PARADEEP
    "INE619A01035", // PATANJALI
    "INE982J01020", // PAYTM
    "INE602A01031", // PCBL
    "INE262H01021", // PERSISTENT
    "INE347G01014", // PETRONET
    "INE134E01011", // PFC
    "INE182A01018", // PFIZER
    "INE367G01038", // PFOCUS
    "INE457L01029", // PGEL
    "INE211B01039", // PHOENIXLTD
    "INE318A01026", // PIDILITIND
    "INE603J01030", // PIIND
    "INE15B701018", // PINELABS
    "INE202B01038", // PIRAMALFIN
    "INE160A01022", // PNB
    "INE572E01012", // PNBHOUSING
    "INE417T01026", // POLICYBZR
    "INE455K01017", // POLYCAB
    "INE205C01021", // POLYMED
    "INE511C01022", // POONAWALLA
    "INE752E01010", // POWERGRID
    "INE07Y701011", // POWERINDIA
    "INE0DK501011", // PPLPHARMA
    "INE0BS701011", // PREMIERENE
    "INE811K01011", // PRESTIGE
    "INE596F01018", // PTCIL
    "INE191H01014", // PVRINOX
    "INE0LP301011", // PWL
    "INE944F01028", // RADICO
    "INE0DD101019", // RAILTEL
    "INE961O01016", // RAINBOW
    "INE331A01037", // RAMCOCEM
    "INE976G01028", // RBLBANK
    "INE020B01018", // RECLTD
    "INE891D01026", // REDINGTON
    "INE002A01018", // RELIANCE
    "INE743M01012", // RHIM
    "INE320J01015", // RITES
    "INE399G01023", // RKFORGE
    "INE614G01033", // RPOWER
    "INE777K01022", // RRKABEL
    "INE415G01027", // RVNL
    "INE0W2G01015", // SAGILITY
    "INE114A01011", // SAIL
    "INE570L01029", // SAILIFE
    "INE148I01020", // SAMMAANCAP
    "INE806T01020", // SAPPHIRE
    "INE385C01021", // SARDAEN
    "INE979A01025", // SAREGAMA
    "INE423Y01016", // SBFC
    "INE018E01016", // SBICARD
    "INE123W01016", // SBILIFE
    "INE062A01020", // SBIN
    "INE513A01022", // SCHAEFFLER
    "INE839M01018", // SCHNEIDER
    "INE109A01011", // SCI
    "INE070A01015", // SHREECEM
    "INE721A01047", // SHRIRAMFIN
    "INE810G01011", // SHYAMMETL
    "INE003A01024", // SIEMENS
    "INE903U01023", // SIGNATURE
    "INE002L01015", // SJVN
    "INE671H01015", // SOBHA
    "INE343H01029", // SOLARINDS
    "INE073K01018", // SONACOMS
    "INE269A01021", // SONATSOFTW
    "INE663A01033", // SPLPETRO
    "INE647A01010", // SRF
    "INE575P01011", // STARHEALTH
    "INE258G01013", // SUMICHEM
    "INE660A01013", // SUNDARMFIN
    "INE044A01036", // SUNPHARMA
    "INE424H01027", // SUNTV
    "INE195A01028", // SUPREMEIND
    "INE040H01021", // SUZLON
    "INE665A01038", // SWANCORP
    "INE00H001014", // SWIGGY
    "INE398R01022", // SYNGENE
    "INE0DYJ01015", // SYRMA
    "INE763I01026", // TARIL
    "INE976I01016", // TATACAP
    "INE092A01019", // TATACHEM
    "INE151A01013", // TATACOMM
    "INE192A01025", // TATACONSUM
    "INE670A01012", // TATAELXSI
    "INE672A01026", // TATAINVEST
    "INE245A01021", // TATAPOWER
    "INE081A01020", // TATASTEEL
    "INE142M01025", // TATATECH
    "INE673O01025", // TBOTEK
    "INE467B01029", // TCS
    "INE669C01036", // TECHM
    "INE285K01026", // TECHNOE
    "INE011K01018", // TEGA
    "INE010J01012", // TEJASNET
    "INE19RI01016", // TENNIND
    "INE0AQ201015", // THELEELA
    "INE152A01029", // THERMAX
    "INE974X01010", // TIINDIA
    "INE325A01013", // TIMKEN
    "INE615H01020", // TITAGARH
    "INE280A01028", // TITAN
    "INE1TAE01010", // TMCV
    "INE155A01022", // TMPV
    "INE685A01028", // TORNTPHARM
    "INE813H01021", // TORNTPOWER
    "INE103V01028", // TRAVELFOOD
    "INE849A01020", // TRENT
    "INE064C01022", // TRIDENT
    "INE152M01016", // TRITURBINE
    "INE517B01013", // TTML
    "INE494B01023", // TVSMOTOR
    "INE686F01025", // UBL
    "INE691A01018", // UCOBANK
    "INE481G01011", // ULTRACEMCO
    "INE692A01016", // UNIONBANK
    "INE854D01024", // UNITDSPR
    "INE405E01023", // UNOMINDA
    "INE628A01036", // UPL
    "INE0CAZ01013", // URBANCO
    "INE228A01035", // USHAMART
    "INE094J01016", // UTIAMC
    "INE200M01039", // VBL
    "INE205A01025", // VEDL
    "INE043W01024", // VIJAYA
    "INE01EA01019", // VMM
    "INE226A01021", // VOLTAS
    "INE825A01020", // VTL
    "INE377N01017", // WAAREEENER
    "INE191B01025", // WELCORP
    "INE192B01031", // WELSPUNLIV
    "INE716A01013", // WHIRLPOOL
    "INE075A01022", // WIPRO
    "INE049B01025", // WOCKPHARMA
    "INE528G01035", // YESBANK
    "INE256A01028", // ZEEL
    "INE520A01027", // ZENSARTECH
    "INE251B01027", // ZENTEC
    "INE342J01019", // ZFCVINDIA
    "INE010B01027", // ZYDUSLIFE
    "INE768C01028", // ZYDUSWELL
];

/// NSE's own ISIN for each name of [`NIFTY_TOTAL_MARKET`], at the same index.
///
/// Transcribed from `ind_niftytotalmarket_list.csv` — 49,178 bytes, sha256
/// `e67c8d99d10b541c56a9b10fd0b9de15ae9b6eae34347a6531c78104b5924c1e`,
/// 752 data rows — the same way the four tiers above are. **749 of 750 carry
/// one.**
///
/// # The one that does not, and why it is left absent
///
/// `AGL` sits at index 33 and the exchange's file has no row for it. That is
/// not news and it is not this change's to fix: `docs/06-limits.md` §11
/// already records that the published file holds `GRINDWELL` where this array
/// holds `AGL`, and that `AGL` is **UNVERIFIED** — this repository does not
/// know what instrument it is. Writing `GRINDWELL`'s `INE536A01023` here would
/// attach a known identity to an unknown name, which is worse than absence: it
/// would make the wrong row *join*. So the position carries [`ISIN_ABSENT`],
/// and `crate::universe::nse_isin` answers `None` for it.
///
/// # What the file holds that this array does not
///
/// The file's 752 rows are 750 real names plus `DUMMYINXGN` and `DUMMYTRVN`,
/// two placeholder scrips carrying `DUM`-prefixed pseudo-ISINs. They are
/// MALFORMED by construction — wrong prefix, and both fail the ISO 6166 check
/// digit, which `the_placeholder_scrips_are_malformed_and_are_not_here` proves
/// rather than asserts. [`NIFTY_TOTAL_MARKET`] never held them and neither
/// does this.
pub const NIFTY_TOTAL_MARKET_ISIN: [&str; 750] = [
    "INE466L01038", // 360ONE
    "INE470A01017", // 3MINDIA
    "INE883F01010", // AADHARHFC
    "INE767A01016", // AARTIDRUGS
    "INE769A01020", // AARTIIND
    "INE0LRU01027", // AARTIPHARM
    "INE216P01012", // AAVAS
    "INE117A01022", // ABB
    "INE358A01014", // ABBOTINDIA
    "INE674K01013", // ABCAPITAL
    "INE552Z01027", // ABDL
    "INE647O01011", // ABFRL
    "INE14LE01019", // ABLBL
    "INE055A01016", // ABREL
    "INE404A01024", // ABSLAMC
    "INE012A01025", // ACC
    "INE731H01025", // ACE
    "INE128X01021", // ACI
    "INE622W01025", // ACMESOLAR
    "INE00FF01025", // ACUTAAS
    "INE931S01010", // ADANIENSOL
    "INE423A01024", // ADANIENT
    "INE364U01010", // ADANIGREEN
    "INE742F01042", // ADANIPORTS
    "INE814H01029", // ADANIPOWER
    "INE837H01020", // ADVENZYMES
    "INE208C01025", // AEGISLOG
    "INE0INX01018", // AEGISVOPAK
    "INE947N01017", // AEQUS
    "INE0BWX01014", // AETHER
    "INE101I01011", // AFCONS
    "INE00WC01027", // AFFLE
    "INE943P01029", // AGARWALEYE
    ISIN_ABSENT,    // AGL
    "INE758C01029", // AHLUCONT
    "INE212H01026", // AIAENG
    "INE206F01022", // AIIL
    "INE031B01049", // AJANTPHARM
    "INE09XN01023", // AKUMS
    "INE03Q201024", // ALIVUS
    "INE540L01014", // ALKEM
    "INE150B01039", // ALKYLAMINE
    "INE270A01029", // ALOKINDS
    "INE371P01015", // AMBER
    "INE079A01024", // AMBUJACEM
    "INE463V01026", // ANANDRATHI
    "INE242C01024", // ANANTRAJ
    "INE732I01021", // ANGELONE
    "INE0CZ201020", // ANTHEM
    "INE294Z01018", // ANUP
    "INE930P01018", // ANURAS
    "INE372A01015", // APARINDS
    "INE702C01027", // APLAPOLLO
    "INE901L01018", // APLLTD
    "INE713T01028", // APOLLO
    "INE437A01024", // APOLLOHOSP
    "INE438A01022", // APOLLOTYRE
    "INE852O01025", // APTUS
    "INE885A01032", // ARE&M
    "INE034A01011", // ARVIND
    "INE955V01021", // ARVINDFASN
    "INE439A01020", // ASAHIINDIA
    "INE348A01023", // ASHAPURMIN
    "INE442H01029", // ASHOKA
    "INE208A01029", // ASHOKLEY
    "INE021A01026", // ASIANPAINT
    "INE491J01022", // ASKAUTOLTD
    "INE914M01019", // ASTERDM
    "INE006I01046", // ASTRAL
    "INE386C01029", // ASTRAMICRO
    "INE399L01023", // ATGL
    "INE0LEZ01016", // ATHERENERG
    "INE0Z4F01028", // ATLANTAELE
    "INE100A01010", // ATUL
    "INE949L01017", // AUBANK
    "INE132H01018", // AURIONPRO
    "INE406A01037", // AUROPHARMA
    "INE0LCL01028", // AVALON
    "INE871C01038", // AVANTIFEED
    "INE679V01027", // AVL
    "INE108V01019", // AWFIS
    "INE699H01024", // AWL
    "INE238A01034", // AXISBANK
    "INE555B01013", // AXISCADES
    "INE02IJ01035", // AZAD
    "INE917I01010", // BAJAJ-AUTO
    "INE193E01025", // BAJAJELEC
    "INE918I01026", // BAJAJFINSV
    "INE377Y01014", // BAJAJHFL
    "INE118A01012", // BAJAJHLDNG
    "INE296A01032", // BAJFINANCE
    "INE050E01027", // BALAMINES
    "INE787D01026", // BALKRISIND
    "INE119A01028", // BALRAMCHIN
    "INE011E01029", // BALUFORGE
    "INE213C01025", // BANCOINDIA
    "INE545U01014", // BANDHANBNK
    "INE028A01039", // BANKBARODA
    "INE084A01016", // BANKINDIA
    "INE176A01028", // BATAINDIA
    "INE462A01022", // BAYERCROP
    "INE676A01027", // BBOX
    "INE050A01025", // BBTC
    "INE171Z01026", // BDL
    "INE495P01020", // BECTORFOOD
    "INE263A01024", // BEL
    "INE894V01022", // BELRISE
    "INE258A01024", // BEML
    "INE463A01038", // BERGEPAINT
    "INE465A01025", // BHARATFORG
    "INE397D01024", // BHARTIARTL
    "INE343G01021", // BHARTIHEXA
    "INE257A01026", // BHEL
    "INE00E101023", // BIKAJI
    "INE376G01013", // BIOCON
    "INE340A01012", // BIRLACORPN
    "INE0UIZ01018", // BLACKBUCK
    "INE153T01027", // BLS
    "INE233B01017", // BLUEDART
    "INE0KBH01020", // BLUEJET
    "INE472A01039", // BLUESTARCO
    "INE304W01038", // BLUESTONE
    "INE666D01022", // BORORENEW
    "INE323A01026", // BOSCHLTD
    "INE029A01011", // BPCL
    "INE791I01019", // BRIGADE
    "INE216A01030", // BRITANNIA
    "INE118H01025", // BSE
    "INE836A01035", // BSOFT
    "INE278Y01022", // CAMPUS
    "INE596I01020", // CAMS
    "INE476A01022", // CANBK
    "INE477A01020", // CANFINHOME
    "INE01TY01017", // CANHLIFE
    "INE0ILV01024", // CAPILLARY
    "INE475E01026", // CAPLIPOINT
    "INE120A01034", // CARBORUNIV
    "INE290S01011", // CARTRADE
    "INE172A01027", // CASTROLIND
    "INE483S01020", // CCAVENUE
    "INE421D01022", // CCL
    "INE736A01011", // CDSL
    "INE482A01020", // CEATLTD
    "INE0LMW01024", // CELLO
    "INE686A01026", // CEMPRO
    "INE483A01010", // CENTRALBK
    "INE348B01021", // CENTURYPLY
    "INE739E01017", // CERA
    "INE486A01021", // CESC
    "INE180C01042", // CGCL
    "INE067A01029", // CGPOWER
    "INE427F01016", // CHALET
    "INE085A01013", // CHAMBLFERT
    "INE178A01016", // CHENNPETRO
    "INE102B01014", // CHOICEIN
    "INE121A01024", // CHOLAFIN
    "INE149A01033", // CHOLAHLDNG
    "INE536H01010", // CIEINDIA
    "INE059A01026", // CIPLA
    "INE227W01023", // CLEAN
    "INE925R01014", // CMSINFO
    "INE522F01014", // COALINDIA
    "INE704P01025", // COCHINSHIP
    "INE591G01025", // COFORGE
    "INE03QK01018", // COHANCE
    "INE259A01022", // COLPAL
    "INE111A01025", // CONCOR
    "INE338H01029", // CONCORDBIO
    "INE169A01031", // COROMANDEL
    "INE02ZQ01018", // CORONA
    "INE819V01029", // CPPLUS
    "INE00LO01017", // CRAFTSMAN
    "INE218I01013", // CRAMC
    "INE741K01010", // CREDITACC
    "INE007A01025", // CRISIL
    "INE0S4R01014", // CRIZAC
    "INE299U01018", // CROMPTON
    "INE679A01013", // CSBBANK
    "INE491A01021", // CUB
    "INE298A01020", // CUMMINSIND
    "INE509F01029", // CUPID
    "INE136B01020", // CYIENT
    "INE016A01026", // DABUR
    "INE00R701025", // DALBHARAT
    "INE365B01017", // DATAMATICS
    "INE0IX101010", // DATAPATTNS
    "INE917M01012", // DBL
    "INE879I01012", // DBREALTY
    "INE503A01015", // DCBBANK
    "INE499A01024", // DCMSHRIRAM
    "INE501A01019", // DEEPAKFERT
    "INE288B01029", // DEEPAKNTR
    "INE148O01028", // DELHIVERY
    "INE872J01023", // DEVYANI
    "INE989C01038", // DIACABS
    "INE361B01024", // DIVISLAB
    "INE935N01020", // DIXON
    "INE271C01023", // DLF
    "INE192R01011", // DMART
    "INE321T01012", // DOMS
    "INE089A01031", // DRREDDY
    "INE221B01012", // DYNAMATECH
    "INE738I01010", // ECLERX
    "INE532F01054", // EDELWEISS
    "INE066A01021", // EICHERMOT
    "INE126A01031", // EIDPARRY
    "INE0LLY01014", // EIEL
    "INE230A01023", // EIHOTEL
    "INE205B01031", // ELECON
    "INE086A01029", // ELECTCAST
    "INE285A01027", // ELGIEQUIP
    "INE236E01022", // ELLEN
    "INE548C01032", // EMAMILTD
    "INE069I01010", // EMBDL
    "INE168P01015", // EMCURE
    "INE02YR01019", // EMIL
    "INE1C6T01020", // EMMVEE
    "INE913H01037", // ENDURANCE
    "INE510A01028", // ENGINERSIN
    "INE1NPP01017", // ENRIN
    "INE010601016", // ENTERO
    "INE255A01020", // EPL
    "INE063P01018", // EQUITASBNK
    "INE406M01024", // ERIS
    "INE042A01014", // ESCORTS
    "INE758T01015", // ETERNAL
    "INE04TZ01018", // ETHOSLTD
    "INE0KCE01017", // EUREKAFORB
    "INE302A01020", // EXIDEIND
    "INE188A01015", // FACT
    "INE171A01029", // FEDERALBNK
    "INE007N01010", // FEDFINA
    "INE737H01014", // FIEMIND
    "INE235A01022", // FINCABLES
    "INE183A01024", // FINPIPE
    "INE02RE01045", // FIRSTCRY
    "INE128S01021", // FIVESTAR
    "INE09N301011", // FLUOROCHEM
    "INE451A01017", // FORCEMOT
    "INE061F01013", // FORTIS
    "INE684F01012", // FSL
    "INE524A01029", // GABRIEL
    "INE036B01030", // GAEL
    "INE129A01019", // GAIL
    "INE297H01019", // GALLANTT
    "INE017A01032", // GESHIP
    "INE539A01019", // GHCL
    "INE481Y01014", // GICRE
    "INE322A01010", // GILLETTE
    "INE068V01023", // GLAND
    "INE159A01016", // GLAXO
    "INE935A01035", // GLENMARK
    "INE131A01031", // GMDCLTD
    "INE541A01023", // GMMPFAUDLR
    "INE776C01039", // GMRAIRPORT
    "INE0CU601026", // GMRP&UI
    "INE113A01013", // GNFC
    "INE260B01028", // GODFRYPHLP
    "INE03JT01014", // GODIGIT
    "INE850D01014", // GODREJAGRO
    "INE102D01028", // GODREJCP
    "INE233A01035", // GODREJIND
    "INE484J01027", // GODREJPROP
    "INE887G01027", // GOKEX
    "INE314T01033", // GOKULAGRO
    "INE177H01039", // GPIL
    "INE517F01014", // GPPL
    "INE101D01020", // GRANULES
    "INE371A01025", // GRAPHITE
    "INE047A01021", // GRASIM
    "INE024L01027", // GRAVITA
    "INE224A01026", // GREAVESCOT
    "INE0HOQ01053", // GROWW
    "INE382Z01011", // GRSE
    "INE291A01017", // GRWRHITECH
    "INE026A01025", // GSFC
    "INE200A01026", // GVT&D
    "INE066F01020", // HAL
    "INE419U01012", // HAPPSTMNDS
    "INE176B01034", // HAVELLS
    "INE292B01021", // HBLENGINE
    "INE549A01026", // HCC
    "INE075I01017", // HCG
    "INE860A01027", // HCLTECH
    "INE756I01012", // HDBFS
    "INE127D01025", // HDFCAMC
    "INE040A01034", // HDFCBANK
    "INE795G01014", // HDFCLIFE
    "INE545A01024", // HEG
    "INE0AJG01018", // HEMIPROP
    "INE978A01027", // HERITGFOOD
    "INE158A01026", // HEROMOTOCO
    "INE093A01041", // HEXT
    "INE548A01028", // HFCL
    "INE926X01010", // HGINFRA
    "INE038A01020", // HINDALCO
    "INE531E01026", // HINDCOPPER
    "INE094A01015", // HINDPETRO
    "INE030A01027", // HINDUNILVR
    "INE267A01025", // HINDZINC
    "INE481N01025", // HOMEFIRST
    "INE0J5401028", // HONASA
    "INE671A01010", // HONAUT
    "INE019C01026", // HSCL
    "INE031A01017", // HUDCO
    "INE0V6F01027", // HYUNDAI
    "INE346A01027", // ICICIAMC
    "INE090A01021", // ICICIBANK
    "INE765G01017", // ICICIGI
    "INE726G01019", // ICICIPRULI
    "INE483B01026", // ICIL
    "INE008A01015", // IDBI
    "INE669E01016", // IDEA
    "INE092T01019", // IDFCFIRSTB
    "INE022Q01020", // IEX
    "INE559A01017", // IFBIND
    "INE039A01010", // IFCI
    "INE0Q9301021", // IGIL
    "INE203G01027", // IGL
    "INE530B01024", // IIFL
    "INE489L01022", // IIFLCAPS
    "INE115Q01022", // IKS
    "INE919H01018", // IMFA
    "INE065X01017", // INDGN
    "INE053A01029", // INDHOTEL
    "INE383A01012", // INDIACEM
    "INE560A01023", // INDIAGLYCO
    "INE933S01016", // INDIAMART
    "INE562A01011", // INDIANB
    "INE922K01024", // INDIASHLTR
    "INE646L01027", // INDIGO
    "INE09VQ01012", // INDIGOPNTS
    "INE095A01012", // INDUSINDBK
    "INE121J01017", // INDUSTOWER
    "INE009A01021", // INFY
    "INE510W01014", // INOXGREEN
    "INE616N01034", // INOXINDIA
    "INE066P01011", // INOXWIND
    "INE306R01017", // INTELLECT
    "INE565A01014", // IOB
    "INE242A01010", // IOC
    "INE570A01022", // IONEXCHANG
    "INE571A01038", // IPCALAB
    "INE821I01022", // IRB
    "INE962Y01021", // IRCON
    "INE335Y01020", // IRCTC
    "INE202E01016", // IREDA
    "INE053F01010", // IRFC
    "INE154A01025", // ITC
    "INE379A01028", // ITCHOTELS
    "INE248A01017", // ITI
    "INE0HV901016", // IXIGO
    "INE168A01041", // J&KBANK
    "INE091G01026", // JAIBALAJI
    "INE0YD401026", // JAINREC
    "INE039C01032", // JAMNAAUTO
    "INE854B01010", // JAYNECOIND
    "INE927D01051", // JBMA
    "INE324A01032", // JINDALSAW
    "INE749A01030", // JINDALSTEL
    "INE758E01017", // JIOFIN
    "INE823G01014", // JKCEMENT
    "INE786A01032", // JKLAKSHMI
    "INE789E01012", // JKPAPER
    "INE573A01042", // JKTYRE
    "INE682M01020", // JLHL
    "INE780C01023", // JMFINANCIL
    "INE351F01018", // JPPOWER
    "INE953L01027", // JSFB
    "INE220G01021", // JSL
    "INE0J5801029", // JSLL
    "INE718I01012", // JSWCEMENT
    "INE133A01011", // JSWDULUX
    "INE121E01018", // JSWENERGY
    "INE880J01026", // JSWINFRA
    "INE019A01038", // JSWSTEEL
    "INE797F01020", // JUBLFOOD
    "INE0BY001018", // JUBLINGREA
    "INE700A01033", // JUBLPHARMA
    "INE599M01018", // JUSTDIAL
    "INE209L01016", // JWL
    "INE668F01031", // JYOTHYLAB
    "INE980O01024", // JYOTICNC
    "INE217B01036", // KAJARIACER
    "INE303R01014", // KALYANKJIL
    "INE531A01024", // KANSAINER
    "INE036D01028", // KARURVYSYA
    "INE918Z01012", // KAYNES
    "INE389H01022", // KEC
    "INE878B01027", // KEI
    "INE138Y01010", // KFINTECH
    "INE967H01025", // KIMS
    "INE732A01036", // KIRLOSBROS
    "INE146L01010", // KIRLOSENG
    "INE811A01020", // KIRLPNU
    "INE602G01020", // KITEX
    "INE634I01029", // KNRCON
    "INE237A01036", // KOTAKBANK
    "INE542W01025", // KPIGREEN
    "INE220B01022", // KPIL
    "INE04I401011", // KPITTECH
    "INE930H01031", // KPRMILL
    "INE001B01026", // KRBL
    "INE0Q3J01015", // KRN
    "INE999A01023", // KSB
    "INE455I01029", // KSCL
    "INE614B01018", // KTKBANK
    "INE600L01024", // LALPATHLAB
    "INE0I7C01011", // LATENTVIEW
    "INE947Q01028", // LAURUSLABS
    "INE970X01018", // LEMONTREE
    "INE956O01016", // LENSKART
    "INE324D01010", // LGEINDIA
    "INE115A01026", // LICHSGFIN
    "INE0J1Y01017", // LICI
    "INE473A01011", // LINDEINDIA
    "INE093R01011", // LLOYDSENGG
    "INE080I01025", // LLOYDSENT
    "INE281B01032", // LLOYDSME
    "INE670K01029", // LODHA
    "INE0V9Q01010", // LOTUSDEV
    "INE018A01030", // LT
    "INE498L01015", // LTF
    "INE818H01020", // LTFOODS
    "INE214T01019", // LTM
    "INE010V01017", // LTTS
    "INE872H01027", // LUMAXTECH
    "INE326A01037", // LUPIN
    "INE576O01020", // LXCHEM
    "INE101A01026", // M&M
    "INE774D01024", // M&MFIN
    "INE457A01014", // MAHABANK
    "INE288A01013", // MAHSCOOTER
    "INE271B01025", // MAHSEAMLES
    "INE522D01027", // MANAPPURAM
    "INE634S01028", // MANKIND
    "INE00VM01036", // MANORAMA
    "INE825V01034", // MANYAVAR
    "INE0BV301023", // MAPMYINDIA
    "INE196A01026", // MARICO
    "INE750C01026", // MARKSANS
    "INE585B01010", // MARUTI
    "INE759A01021", // MASTEK
    "INE027H01010", // MAXHEALTH
    "INE249Z01020", // MAZDOCK
    "INE745G01043", // MCX
    "INE474Q01031", // MEDANTA
    "INE804L01022", // MEDPLUS
    "INE0VDM01015", // MEESHO
    "INE112L01020", // METROPOLIS
    "INE180A01020", // MFSL
    "INE002S01010", // MGL
    "INE099Z01011", // MIDHANI
    "INE842C01021", // MINDACORP
    "INE123F01029", // MMTC
    "INE490G01020", // MOIL
    "INE775A01035", // MOTHERSON
    "INE338I01027", // MOTILALOFS
    "INE356A01018", // MPHASIS
    "INE883A01011", // MRF
    "INE103A01014", // MRPL
    "INE255X01014", // MSTCLTD
    "INE0FS801015", // MSUMI
    "INE864I01014", // MTARTECH
    "INE414G01012", // MUTHOOTFIN
    "INE298J01013", // NAM-INDIA
    "INE987B01026", // NATCOPHARM
    "INE139A01034", // NATIONALUM
    "INE663F01032", // NAUKRI
    "INE725A01030", // NAVA
    "INE048G01026", // NAVINFLUOR
    "INE418L01047", // NAZARA
    "INE095N01031", // NBCC
    "INE868B01028", // NCC
    "INE136S01016", // NEOGEN
    "INE317F01035", // NESCO
    "INE239A01024", // NESTLEIND
    "INE0NT901020", // NETWEB
    "INE870H01013", // NETWORK18
    "INE794A01010", // NEULANDLAB
    "INE619B01017", // NEWGEN
    "INE870D01012", // NFL
    "INE410P01011", // NH
    "INE848E01016", // NHPC
    "INE470Y01017", // NIACL
    "INE995S01015", // NIVABUPA
    "INE589A01014", // NLCINDIA
    "INE584A01023", // NMDC
    "INE0NNS01018", // NSLNISP
    "INE733E01010", // NTPC
    "INE0ONG01011", // NTPCGREEN
    "INE531F01023", // NUVAMA
    "INE118D01016", // NUVOCO
    "INE388Y01029", // NYKAA
    "INE093I01010", // OBEROIRLTY
    "INE881D01027", // OFSS
    "INE274J01014", // OIL
    "INE0LXG01040", // OLAELEC
    "INE260D01016", // OLECTRA
    "INE013P01021", // ONESOURCE
    "INE213A01029", // ONGC
    "INE350C01017", // OPTIEMUS
    "INE876N01018", // ORIENTCEM
    "INE16NZ01023", // ORKLAINDIA
    "INE0BYP01024", // OSWALPUMPS
    "INE761H01022", // PAGEIND
    "INE088F01024", // PARADEEP
    "INE045601023", // PARAS
    "INE119201023", // PARKHOSPS
    "INE619A01035", // PATANJALI
    "INE982J01020", // PAYTM
    "INE602A01031", // PCBL
    "INE785M01021", // PCJEWELLER
    "INE262H01021", // PERSISTENT
    "INE347G01014", // PETRONET
    "INE134E01011", // PFC
    "INE182A01018", // PFIZER
    "INE367G01038", // PFOCUS
    "INE457L01029", // PGEL
    "INE940H01022", // PGIL
    "INE211B01039", // PHOENIXLTD
    "INE546C01010", // PICCADIL
    "INE318A01026", // PIDILITIND
    "INE603J01030", // PIIND
    "INE15B701018", // PINELABS
    "INE202B01038", // PIRAMALFIN
    "INE160A01022", // PNB
    "INE572E01012", // PNBHOUSING
    "INE195J01029", // PNCINFRA
    "INE953R01016", // PNGJL
    "INE417T01026", // POLICYBZR
    "INE455K01017", // POLYCAB
    "INE205C01021", // POLYMED
    "INE511C01022", // POONAWALLA
    "INE752E01010", // POWERGRID
    "INE07Y701011", // POWERINDIA
    "INE211R01019", // POWERMECH
    "INE0DK501011", // PPLPHARMA
    "INE074A01025", // PRAJIND
    "INE0BS701011", // PREMIERENE
    "INE811K01011", // PRESTIGE
    "INE726V01018", // PRICOLLTD
    "INE959A01019", // PRIVISCL
    "INE010A01011", // PRSMJOHNSN
    "INE00F201020", // PRUDENT
    "INE877F01012", // PTC
    "INE596F01018", // PTCIL
    "INE323I01011", // PURVA
    "INE191H01014", // PVRINOX
    "INE0LP301011", // PWL
    "INE0SII01026", // QPOWER
    "INE615P01015", // QUESS
    "INE944F01028", // RADICO
    "INE0DD101019", // RAILTEL
    "INE855B01025", // RAIN
    "INE961O01016", // RAINBOW
    "INE613A01020", // RALLIS
    "INE331A01037", // RAMCOCEM
    "INE0CLI01024", // RATEGAIN
    "INE703B01027", // RATNAMANI
    "INE02ID01020", // RAYMONDLSL
    "INE07T201019", // RBA
    "INE976G01028", // RBLBANK
    "INE027A01015", // RCF
    "INE020B01018", // RECLTD
    "INE891D01026", // REDINGTON
    "INE0LXT01019", // REDTAPE
    "INE056I01025", // REFEX
    "INE131B01039", // RELAXO
    "INE002A01018", // RELIANCE
    "INE621H01010", // RELIGARE
    "INE087H01022", // RENUKA
    "INE743M01012", // RHIM
    "INE320J01015", // RITES
    "INE399G01023", // RKFORGE
    "INE450U01017", // ROUTE
    "INE614G01033", // RPOWER
    "INE777K01022", // RRKABEL
    "INE834M01019", // RTNINDIA
    "INE399K01017", // RTNPOWER
    "INE506V01022", // RUBICON
    "INE415G01027", // RVNL
    "INE13B501022", // SAATVIKGL
    "INE429E01023", // SAFARI
    "INE0W2G01015", // SAGILITY
    "INE114A01011", // SAIL
    "INE570L01029", // SAILIFE
    "INE08U801020", // SAMHI
    "INE148I01020", // SAMMAANCAP
    "INE149K01016", // SANDUMA
    "INE0UOS01011", // SANOFICONR
    "INE953O01021", // SANSERA
    "INE806T01020", // SAPPHIRE
    "INE385C01021", // SARDAEN
    "INE979A01025", // SAREGAMA
    "INE423Y01016", // SBFC
    "INE018E01016", // SBICARD
    "INE123W01016", // SBILIFE
    "INE062A01020", // SBIN
    "INE513A01022", // SCHAEFFLER
    "INE839M01018", // SCHNEIDER
    "INE109A01011", // SCI
    "INE602W01027", // SENCO
    "INE916U01025", // SFL
    "INE151G01028", // SHAILY
    "INE908D01010", // SHAKTIPUMP
    "INE221J01015", // SHARDACROP
    "INE932X01026", // SHAREINDIA
    "INE790G01031", // SHILPAMED
    "INE070A01015", // SHREECEM
    "INE526E01018", // SHRIPISTON
    "INE721A01047", // SHRIRAMFIN
    "INE810G01011", // SHYAMMETL
    "INE003A01024", // SIEMENS
    "INE903U01023", // SIGNATURE
    "INE002L01015", // SJVN
    "INE640A01023", // SKFINDIA
    "INE2J8701016", // SKFINDUS
    "INE439E01022", // SKIPPER
    "INE01IU01018", // SKYGOLD
    "INE0NAZ01010", // SMARTWORKS
    "INE294B01019", // SMLMAH
    "INE671H01015", // SOBHA
    "INE343H01029", // SOLARINDS
    "INE073K01018", // SONACOMS
    "INE269A01021", // SONATSOFTW
    "INE683A01023", // SOUTHBANK
    "INE232I01014", // SPARC
    "INE663A01033", // SPLPETRO
    "INE647A01010", // SRF
    "INE939A01011", // STAR
    "INE460H01021", // STARCEMENT
    "INE575P01011", // STARHEALTH
    "INE089C01029", // STLTECH
    "INE04VU01023", // STYL
    "INE189B01011", // STYRENIX
    "INE287B01021", // SUBROS
    "INE659A01023", // SUDARSCHEM
    "INE0QPI01025", // SUDEEPPHRM
    "INE258G01013", // SUMICHEM
    "INE660A01013", // SUNDARMFIN
    "INE044A01036", // SUNPHARMA
    "INE805D01034", // SUNTECK
    "INE424H01027", // SUNTV
    "INE195A01028", // SUPREMEIND
    "INE07RO01027", // SUPRIYA
    "INE335A01020", // SURYAROSNI
    "INE040H01021", // SUZLON
    "INE665A01038", // SWANCORP
    "INE00H001014", // SWIGGY
    "INE00M201021", // SWSOLAR
    "INE398R01022", // SYNGENE
    "INE0DYJ01015", // SYRMA
    "INE483C01032", // TANLA
    "INE0EK901012", // TARC
    "INE763I01026", // TARIL
    "INE976I01016", // TATACAP
    "INE092A01019", // TATACHEM
    "INE151A01013", // TATACOMM
    "INE192A01025", // TATACONSUM
    "INE670A01012", // TATAELXSI
    "INE672A01026", // TATAINVEST
    "INE245A01021", // TATAPOWER
    "INE081A01020", // TATASTEEL
    "INE142M01025", // TATATECH
    "INE673O01025", // TBOTEK
    "INE467B01029", // TCS
    "INE419M01027", // TDPOWERSYS
    "INE669C01036", // TECHM
    "INE285K01026", // TECHNOE
    "INE011K01018", // TEGA
    "INE010J01012", // TEJASNET
    "INE19RI01016", // TENNIND
    "INE621L01012", // TEXRAIL
    "INE085J01014", // THANGAMAYL
    "INE0AQ201015", // THELEELA
    "INE152A01029", // THERMAX
    "INE332A01027", // THOMASCOOK
    "INE594H01019", // THYROCARE
    "INE133E01013", // TI
    "INE974X01010", // TIINDIA
    "INE508G01029", // TIMETECHNO
    "INE325A01013", // TIMKEN
    "INE716B01029", // TIPSMUSIC
    "INE615H01020", // TITAGARH
    "INE280A01028", // TITAN
    "INE668A01016", // TMB
    "INE1TAE01010", // TMCV
    "INE155A01022", // TMPV
    "INE685A01028", // TORNTPHARM
    "INE813H01021", // TORNTPOWER
    "INE454P01035", // TRANSRAILL
    "INE103V01028", // TRAVELFOOD
    "INE849A01020", // TRENT
    "INE064C01022", // TRIDENT
    "INE152M01016", // TRITURBINE
    "INE256C01024", // TRIVENI
    "INE202Z01029", // TSFINV
    "INE517B01013", // TTML
    "INE494B01023", // TVSMOTOR
    "INE395N01027", // TVSSCS
    "INE686F01025", // UBL
    "INE691A01018", // UCOBANK
    "INE551W01018", // UJJIVANSFB
    "INE481G01011", // ULTRACEMCO
    "INE692A01016", // UNIONBANK
    "INE854D01024", // UNITDSPR
    "INE405E01023", // UNOMINDA
    "INE628A01036", // UPL
    "INE0CAZ01013", // URBANCO
    "INE228A01035", // USHAMART
    "INE094J01016", // UTIAMC
    "INE12UR01024", // UTLSOLAR
    "INE945H01021", // V2RETAIL
    "INE884A01027", // VAIBHAVGBL
    "INE665L01035", // VARROC
    "INE200M01039", // VBL
    "INE205A01025", // VEDL
    "INE951I01027", // VGUARD
    "INE043W01024", // VIJAYA
    "INE078V01014", // VIKRAMSOLR
    "INE054A01027", // VIPIND
    "INE807F01027", // VIYASH
    "INE665J01013", // VMART
    "INE01EA01019", // VMM
    "INE540H01012", // VOLTAMP
    "INE226A01021", // VOLTAS
    "INE825A01020", // VTL
    "INE377N01017", // WAAREEENER
    "INE299N01021", // WAAREERTL
    "INE956G01038", // WABAG
    "INE0E7301029", // WAKEFIT
    "INE855C01023", // WEBELSOLAR
    "INE191B01025", // WELCORP
    "INE625G01013", // WELENT
    "INE192B01031", // WELSPUNLIV
    "INE274F01020", // WESTLIFE
    "INE085001019", // WEWORK
    "INE716A01013", // WHIRLPOOL
    "INE075A01022", // WIPRO
    "INE049B01025", // WOCKPHARMA
    "INE0JO301016", // YATHARTH
    "INE528G01035", // YESBANK
    "INE07K301024", // ZAGGLE
    "INE256A01028", // ZEEL
    "INE520A01027", // ZENSARTECH
    "INE251B01027", // ZENTEC
    "INE342J01019", // ZFCVINDIA
    "INE010B01027", // ZYDUSLIFE
    "INE768C01028", // ZYDUSWELL
];

/// NSE's own ISIN for each name of [`FNO_UNDERLYINGS`], at the same index.
///
/// **This one is a join, and the join is the caveat.** The other five arrays
/// each come from the exchange file that publishes that exact list.
/// [`FNO_UNDERLYINGS`] has no such file here — it is DERIVED from the two
/// vendor masters — so its ISINs are taken from the *constituent* file,
/// `ind_niftytotalmarket_list.csv` (49,178 bytes, sha256
/// `e67c8d99d10b541c56a9b10fd0b9de15ae9b6eae34347a6531c78104b5924c1e`), by
/// matching the F&O symbol against that file's `Symbol` column. Every value
/// here is therefore still a value NSE printed, on a row NSE published, beside
/// that symbol — no broker master was consulted and nothing was derived, which
/// is what `CLAUDE.md` §3 rule 1 asks. What it is NOT is a claim that NSE
/// publishes this ISIN *as the F&O underlying's*; it publishes it as that
/// share's. For a share those are the same instrument.
///
/// **208 of 213 carry one.** The five that do not are `BANKNIFTY`,
/// `FINNIFTY`, `MIDCPNIFTY`, `NIFTY` and `NIFTYNXT50` — indices, which are not
/// securities and are issued no ISIN by any numbering agency. Their absence is
/// the correct answer rather than a gap: the same five are the ones the
/// existing note above [`FNO_UNDERLYINGS`] records as carrying no cross-vendor
/// ISIN check either, for exactly this reason.
pub const FNO_UNDERLYINGS_ISIN: [&str; 213] = [
    "INE466L01038", // 360ONE
    "INE117A01022", // ABB
    "INE674K01013", // ABCAPITAL
    "INE931S01010", // ADANIENSOL
    "INE423A01024", // ADANIENT
    "INE364U01010", // ADANIGREEN
    "INE742F01042", // ADANIPORTS
    "INE814H01029", // ADANIPOWER
    "INE540L01014", // ALKEM
    "INE371P01015", // AMBER
    "INE079A01024", // AMBUJACEM
    "INE732I01021", // ANGELONE
    "INE702C01027", // APLAPOLLO
    "INE437A01024", // APOLLOHOSP
    "INE208A01029", // ASHOKLEY
    "INE021A01026", // ASIANPAINT
    "INE006I01046", // ASTRAL
    "INE949L01017", // AUBANK
    "INE406A01037", // AUROPHARMA
    "INE238A01034", // AXISBANK
    "INE917I01010", // BAJAJ-AUTO
    "INE918I01026", // BAJAJFINSV
    "INE118A01012", // BAJAJHLDNG
    "INE296A01032", // BAJFINANCE
    "INE545U01014", // BANDHANBNK
    "INE028A01039", // BANKBARODA
    "INE084A01016", // BANKINDIA
    ISIN_ABSENT,    // BANKNIFTY
    "INE171Z01026", // BDL
    "INE263A01024", // BEL
    "INE465A01025", // BHARATFORG
    "INE397D01024", // BHARTIARTL
    "INE257A01026", // BHEL
    "INE376G01013", // BIOCON
    "INE472A01039", // BLUESTARCO
    "INE323A01026", // BOSCHLTD
    "INE029A01011", // BPCL
    "INE216A01030", // BRITANNIA
    "INE118H01025", // BSE
    "INE596I01020", // CAMS
    "INE476A01022", // CANBK
    "INE736A01011", // CDSL
    "INE067A01029", // CGPOWER
    "INE121A01024", // CHOLAFIN
    "INE059A01026", // CIPLA
    "INE522F01014", // COALINDIA
    "INE704P01025", // COCHINSHIP
    "INE591G01025", // COFORGE
    "INE259A01022", // COLPAL
    "INE111A01025", // CONCOR
    "INE299U01018", // CROMPTON
    "INE298A01020", // CUMMINSIND
    "INE016A01026", // DABUR
    "INE00R701025", // DALBHARAT
    "INE148O01028", // DELHIVERY
    "INE361B01024", // DIVISLAB
    "INE935N01020", // DIXON
    "INE271C01023", // DLF
    "INE192R01011", // DMART
    "INE089A01031", // DRREDDY
    "INE066A01021", // EICHERMOT
    "INE758T01015", // ETERNAL
    "INE171A01029", // FEDERALBNK
    ISIN_ABSENT,    // FINNIFTY
    "INE451A01017", // FORCEMOT
    "INE061F01013", // FORTIS
    "INE129A01019", // GAIL
    "INE935A01035", // GLENMARK
    "INE776C01039", // GMRAIRPORT
    "INE260B01028", // GODFRYPHLP
    "INE102D01028", // GODREJCP
    "INE484J01027", // GODREJPROP
    "INE047A01021", // GRASIM
    "INE200A01026", // GVT&D
    "INE066F01020", // HAL
    "INE176B01034", // HAVELLS
    "INE860A01027", // HCLTECH
    "INE127D01025", // HDFCAMC
    "INE040A01034", // HDFCBANK
    "INE795G01014", // HDFCLIFE
    "INE158A01026", // HEROMOTOCO
    "INE038A01020", // HINDALCO
    "INE094A01015", // HINDPETRO
    "INE030A01027", // HINDUNILVR
    "INE267A01025", // HINDZINC
    "INE0V6F01027", // HYUNDAI
    "INE090A01021", // ICICIBANK
    "INE765G01017", // ICICIGI
    "INE726G01019", // ICICIPRULI
    "INE669E01016", // IDEA
    "INE092T01019", // IDFCFIRSTB
    "INE022Q01020", // IEX
    "INE053A01029", // INDHOTEL
    "INE562A01011", // INDIANB
    "INE646L01027", // INDIGO
    "INE095A01012", // INDUSINDBK
    "INE121J01017", // INDUSTOWER
    "INE009A01021", // INFY
    "INE066P01011", // INOXWIND
    "INE242A01010", // IOC
    "INE202E01016", // IREDA
    "INE053F01010", // IRFC
    "INE154A01025", // ITC
    "INE749A01030", // JINDALSTEL
    "INE758E01017", // JIOFIN
    "INE121E01018", // JSWENERGY
    "INE019A01038", // JSWSTEEL
    "INE797F01020", // JUBLFOOD
    "INE303R01014", // KALYANKJIL
    "INE918Z01012", // KAYNES
    "INE878B01027", // KEI
    "INE138Y01010", // KFINTECH
    "INE237A01036", // KOTAKBANK
    "INE04I401011", // KPITTECH
    "INE947Q01028", // LAURUSLABS
    "INE115A01026", // LICHSGFIN
    "INE0J1Y01017", // LICI
    "INE670K01029", // LODHA
    "INE018A01030", // LT
    "INE498L01015", // LTF
    "INE214T01019", // LTM
    "INE326A01037", // LUPIN
    "INE101A01026", // M&M
    "INE522D01027", // MANAPPURAM
    "INE634S01028", // MANKIND
    "INE196A01026", // MARICO
    "INE585B01010", // MARUTI
    "INE027H01010", // MAXHEALTH
    "INE249Z01020", // MAZDOCK
    "INE745G01043", // MCX
    "INE180A01020", // MFSL
    ISIN_ABSENT,    // MIDCPNIFTY
    "INE775A01035", // MOTHERSON
    "INE338I01027", // MOTILALOFS
    "INE356A01018", // MPHASIS
    "INE414G01012", // MUTHOOTFIN
    "INE298J01013", // NAM-INDIA
    "INE139A01034", // NATIONALUM
    "INE663F01032", // NAUKRI
    "INE095N01031", // NBCC
    "INE239A01024", // NESTLEIND
    "INE848E01016", // NHPC
    ISIN_ABSENT,    // NIFTY
    ISIN_ABSENT,    // NIFTYNXT50
    "INE584A01023", // NMDC
    "INE733E01010", // NTPC
    "INE388Y01029", // NYKAA
    "INE093I01010", // OBEROIRLTY
    "INE881D01027", // OFSS
    "INE274J01014", // OIL
    "INE213A01029", // ONGC
    "INE761H01022", // PAGEIND
    "INE619A01035", // PATANJALI
    "INE982J01020", // PAYTM
    "INE262H01021", // PERSISTENT
    "INE347G01014", // PETRONET
    "INE134E01011", // PFC
    "INE457L01029", // PGEL
    "INE211B01039", // PHOENIXLTD
    "INE318A01026", // PIDILITIND
    "INE603J01030", // PIIND
    "INE160A01022", // PNB
    "INE572E01012", // PNBHOUSING
    "INE417T01026", // POLICYBZR
    "INE455K01017", // POLYCAB
    "INE752E01010", // POWERGRID
    "INE07Y701011", // POWERINDIA
    "INE0BS701011", // PREMIERENE
    "INE811K01011", // PRESTIGE
    "INE944F01028", // RADICO
    "INE976G01028", // RBLBANK
    "INE020B01018", // RECLTD
    "INE002A01018", // RELIANCE
    "INE415G01027", // RVNL
    "INE114A01011", // SAIL
    "INE018E01016", // SBICARD
    "INE123W01016", // SBILIFE
    "INE062A01020", // SBIN
    "INE070A01015", // SHREECEM
    "INE721A01047", // SHRIRAMFIN
    "INE003A01024", // SIEMENS
    "INE343H01029", // SOLARINDS
    "INE073K01018", // SONACOMS
    "INE647A01010", // SRF
    "INE044A01036", // SUNPHARMA
    "INE195A01028", // SUPREMEIND
    "INE040H01021", // SUZLON
    "INE00H001014", // SWIGGY
    "INE192A01025", // TATACONSUM
    "INE670A01012", // TATAELXSI
    "INE245A01021", // TATAPOWER
    "INE081A01020", // TATASTEEL
    "INE467B01029", // TCS
    "INE669C01036", // TECHM
    "INE974X01010", // TIINDIA
    "INE280A01028", // TITAN
    "INE155A01022", // TMPV
    "INE685A01028", // TORNTPHARM
    "INE849A01020", // TRENT
    "INE494B01023", // TVSMOTOR
    "INE481G01011", // ULTRACEMCO
    "INE692A01016", // UNIONBANK
    "INE854D01024", // UNITDSPR
    "INE405E01023", // UNOMINDA
    "INE628A01036", // UPL
    "INE200M01039", // VBL
    "INE205A01025", // VEDL
    "INE01EA01019", // VMM
    "INE226A01021", // VOLTAS
    "INE377N01017", // WAAREEENER
    "INE075A01022", // WIPRO
    "INE528G01035", // YESBANK
    "INE010B01027", // ZYDUSLIFE
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

/// The ISIN the exchange itself prints beside this symbol, in its own
/// constituent file.
///
/// `None` for a symbol NSE does not list as a Total Market constituent, and
/// `None` for the six positions that carry [`ISIN_ABSENT`] — an index, which
/// is issued no ISIN, or `AGL`, which the exchange's file has no row for at
/// all. A caller that needs to tell those two apart asks
/// [`NIFTY_TOTAL_MARKET_ISIN`] directly; a caller that wants an identity to
/// join on wants exactly this, because both cases mean *this build cannot name
/// an NSE-issued ISIN for that symbol* and neither may be joined on.
///
/// # Why the Total Market array and not the tier the caller asked about
///
/// The five constituent arrays NEST — `the_published_tiers_nest_one_inside_the
/// _next` proves 50 ⊂ 100 ⊂ 200 ⊂ 500 ⊂ 750 — and the six ISIN arrays were
/// transcribed from five DIFFERENT published files yet agree on every symbol
/// they share, which `the_isin_arrays_are_positionally_aligned_with_the_names`
/// asserts. So the Total Market array holds an answer for every equity name in
/// any tier, and it is the same answer the tier's own array holds. One probe
/// answers all five.
///
/// # Cost
///
/// One [`MemberIndex::position`] probe — hash, mask, at most 6 steps when the
/// symbol IS a Total Market constituent and at most 11 when it is not — one
/// index, and one 12-byte parse. Two numbers because a hit and a miss are
/// different walks: the hit stops at the matching slot, the miss runs on to
/// the first empty one. `the_probe_length_is_bounded_which_is_what_makes_it_o1`
/// pins the first and
/// `a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too` pins the
/// second; the miss is what the `NIFTY` and `ZZZZNOTREAL` lines of the example
/// below exercise.
/// Both are constant in the size of the list, which is what `CLAUDE.md` §3
/// rule 4 requires.
///
/// # Why the parse is not cached
///
/// [`Isin`] is 12 bytes and [`Copy`], and `Isin::new` is a fixed-length walk
/// with no allocation. Storing 750 parsed `Isin`s would need a `LazyLock` and
/// a runtime allocation in a crate that `core` deliberately builds without;
/// re-parsing costs less than the lock would.
///
/// # Examples
///
/// ```
/// use brutex_core::universe::nse_isin;
/// assert_eq!(nse_isin("RELIANCE").map(|i| i.to_string()).as_deref(), Some("INE002A01018"));
/// // An index is issued no ISIN.
/// assert_eq!(nse_isin("NIFTY"), None);
/// // Not a constituent at all.
/// assert_eq!(nse_isin("ZZZZNOTREAL"), None);
/// ```
#[must_use]
#[expect(
    clippy::indexing_slicing,
    reason = "`at` is the index `NTM_INDEX` was built from -- it is bounded by \
              `NIFTY_TOTAL_MARKET.len()` because `build` set it from that \
              array's own loop counter -- and \
              `the_isin_arrays_are_positionally_aligned_with_the_names` \
              asserts `NIFTY_TOTAL_MARKET_ISIN` has that same length. \
              `.get()` here would add a `None` arm no test could ever enter, \
              which is the uncoverable region CLAUDE.md S9's 100% floor \
              forbids."
)]
pub fn nse_isin(symbol: &str) -> Option<Isin> {
    let at = NTM_INDEX.position(symbol)?;
    // `ISIN_ABSENT` is empty and `Isin::new` refuses it on length, so an
    // absent position becomes `None` here without a second branch to test.
    // A MALFORMED entry would take the same path -- which is why
    // `every_transcribed_isin_is_well_formed` walks all 1,813 positions and
    // makes that a BUILD failure rather than a silent `None`.
    Isin::new(NIFTY_TOTAL_MARKET_ISIN[at]).ok()
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
/// time. Lookup hashes once, masks, and probes.
///
/// "Power-of-two" is a REQUIREMENT and not a description of the tables that
/// happen to exist. The probe advances with `& (N - 1)`, which is the modulo
/// `N` this walk needs only when `N - 1` is a solid run of low bits; at any
/// other size the step function cycles inside part of the table and cannot
/// reach the rest. [`Self::build`] asserts it. It did not until now — this
/// line asserted it in prose, the termination argument below rested on it, and
/// nothing checked it.
///
/// The table is sized so it is at most half full, which is a TERMINATION
/// property and not a cost one. Half full means an empty slot EXISTS; the
/// power-of-two size is what lets a probe reach one; together they end the
/// loop. Neither says how many steps it takes, so the steps are measured, and
/// measured twice because a hit and a miss are different walks:
/// `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1`
/// pins the hit at `<= 8` and
/// `core::universe::a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`
/// pins the miss at `<= 12`. Both walk the table the way [`Self::contains`]
/// does and both pin a NUMBER, so the bounds are measured rather than assumed.
/// With 750 entries in 2048 slots the expected probe is under 1.5; the worst
/// is 6 on a hit and 11 on a miss.
///
/// Costs one slot per entry whether or not it holds anything, and [`Self::ords`]
/// adds a `usize` beside each — 8 bytes a slot on a 64-bit target, 16 KiB on
/// the 2048-slot tables and 54 KiB across all six. That is the space traded for
/// the time, and it is constant rather than growing with the data.
pub struct MemberIndex<const N: usize> {
    /// `pub(crate)` so the probe-length tests can walk the table the way
    /// [`Self::contains`] does and COUNT the steps. Layer 4's bound is the
    /// probe length, and a test that cannot see the slots can only assert the
    /// answer, never the cost. Crate-visible and no wider: nothing outside
    /// `core` has a reason to reach past `contains`.
    pub(crate) slots: [Option<&'static str>; N],
    /// The position the symbol in the same slot has in the list
    /// [`Self::build`] was given.
    ///
    /// # Why it is stored rather than recovered
    ///
    /// [`Self::position`] is what makes the ISIN arrays reachable: the tables
    /// answer *whether* a symbol is a member, and an ISIN needs *where*.
    /// Recovering the index by searching the source list would be the O(n)
    /// scan `CLAUDE.md` §3 rule 4 forbids -- and it would be a SECOND copy of
    /// the ordering, free to disagree with this one. `build` already knows the
    /// index at the moment it inserts; keeping it costs one `usize` per slot
    /// and no time at all.
    ///
    /// A slot holding `None` in [`Self::slots`] holds a meaningless `0` here.
    /// It is never read: [`Self::position`] reads this only after
    /// [`Self::slots`] has matched.
    ords: [usize; N],
}

impl<const N: usize> MemberIndex<N> {
    /// Builds the table at compile time from a list of members.
    ///
    /// # Panics
    ///
    /// At COMPILE time if `N` is not a power of two, or if the table cannot
    /// hold the list — a `const` panic is a build error, not a runtime one, so
    /// neither a mis-sized nor an over-full table can ever ship. The two guard
    /// DIFFERENT properties and only the second was here; see the comment on
    /// the first assertion for the twelve-slot table that satisfies one and
    /// breaks the other.
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
        // THE MASK IS A MODULO ONLY WHEN N IS A POWER OF TWO, AND THE PROBE
        // REACHES EVERY SLOT ONLY WHEN THE MASK IS A MODULO.
        //
        // `mask` and both probe loops step with `& (N - 1)`. That walks the
        // whole table exactly once before repeating only if `N - 1` is a solid
        // run of low bits. At N = 12 it is `0b1011`: from slot 8 the walk is 9,
        // 10, 11, then `12 & 11 == 8` -- a four-slot cycle that never reaches
        // the eight slots below it. This loop would spin forever on a full
        // cluster and `position` would answer `None` for a member the table
        // holds.
        //
        // The half-full assertion below does NOT cover this, and the doc on
        // this type used to read as though it did. Half full guarantees an
        // empty slot EXISTS; it says nothing about the probe being able to
        // reach one. `MemberIndex::<12>::build(&[])` satisfies the second
        // assertion (0 * 2 <= 12) and breaks the first.
        //
        // Every table in this workspace already is a power of two -- the six
        // below and the three in `crate::vendor` -- so this pins a property
        // that holds rather than changing one, and it is here so the next
        // table cannot be the first to break it silently. A `static` is a
        // build failure; `a_table_size_that_is_not_a_power_of_two_is_refused`
        // proves the runtime call is refused too, and
        // `the_probe_reaches_every_slot_only_because_n_is_a_power_of_two`
        // proves the arithmetic this rests on.
        assert!(
            N.is_power_of_two(),
            "the table size must be a power of two so `& (N - 1)` is a modulo"
        );
        assert!(
            members.len() * 2 <= N,
            "the table must stay at most half full so probing stays bounded"
        );
        let mut slots = [None; N];
        let mut ords = [0; N];
        let mut i = 0;
        while i < members.len() {
            let mut at = mask(fnv1a(members[i]), N);
            // Linear probing. Terminates because an empty slot exists (the
            // half-full assertion) and this step function can reach it (the
            // power-of-two assertion). Both are needed; neither alone.
            while slots[at].is_some() {
                at = (at + 1) & (N - 1);
            }
            slots[at] = Some(members[i]);
            ords[at] = i;
            i += 1;
        }
        Self { slots, ords }
    }

    /// Whether the table holds this symbol.
    ///
    /// Hash, mask, probe. The probe stops at the first empty slot, which
    /// exists because the table is at most half full.
    ///
    /// Delegates to [`Self::position`] rather than probing again. There is one
    /// probe loop in this file and there will not be two: a second copy is
    /// free to drift, and a `contains` that said yes where `position` said
    /// where -- or worse, said yes about a DIFFERENT slot -- would hand a
    /// caller another company's ISIN with no test able to see the two answers
    /// disagree.
    #[must_use]
    pub fn contains(&self, symbol: &str) -> bool {
        self.position(symbol).is_some()
    }

    /// Where this symbol sits in the list the table was built from.
    ///
    /// The same hash, mask and probe [`Self::contains`] costs -- it IS what
    /// `contains` costs -- returning the index instead of a bool. That index
    /// is what makes a positionally aligned array beside the list readable in
    /// constant time: `crate::universe::nse_isin` is one call to this plus one
    /// index.
    ///
    /// # Two bounds, because a hit and a miss are different walks
    ///
    /// `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1`
    /// asserts `<= 8` and measures 6 on 750 members and 7 on 213 — **and it
    /// probes only for symbols the table HOLDS.** A miss cannot stop at a
    /// match; it runs on to the first empty slot, which is never the shorter
    /// walk. That is not the rare case: `crate::universe::of_equity` probes six
    /// tables, and a name outside every tier misses in all six.
    ///
    /// So the miss is measured too, by
    /// `core::universe::a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`,
    /// which asserts `<= 12` and measures 11 on 750 members and 10 on 213. It
    /// takes the worst over every SLOT rather than over a list of sample
    /// strings, so it is the true worst case over every possible miss and not a
    /// lucky draw.
    ///
    /// `docs/06-limits.md` §11 still carries the hit figures alone and is the
    /// stale copy to fix; `CLAUDE.md` §10 says which of the two wins. D-0065 is
    /// where the correction from `binary_search` is signed.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "`at` comes from `mask`, which masks to N-1, so it cannot \
                  reach N. The probe advances by the same mask, so it stays in \
                  range for every iteration. `ords` is the same length as \
                  `slots` and is read only at an index `slots` just matched."
    )]
    pub fn position(&self, symbol: &str) -> Option<usize> {
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
            return None;
        }
        let mut at = mask(fnv1a(symbol), N);
        while let Some(held) = self.slots[at] {
            if held == symbol {
                return Some(self.ords[at]);
            }
            at = (at + 1) & (N - 1);
        }
        None
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
/// `n` must be a power of two — `hash & (n - 1)` is the modulo `n` a probe
/// needs only then, and it is `0` for `n = 0` after the subtraction wraps.
/// [`MemberIndex::build`] asserts that for every table, and this function is
/// `pub(crate)` and reached from nowhere that does not go through one.
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
/// bare string a caller had to remember to derive correctly.
///
/// # An index IS looked up in the equity lists
///
/// **This paragraph said "an index is [`Universe::INDEX`] and is never looked
/// up in the equity lists".** The `Kind::Index` arm one line below unions
/// [`of_equity`] into the answer and always has, so the sentence denied the
/// call it sat on top of — and then went on to explain what that very call
/// finds. The true fact it was reaching for is the narrower one in the next
/// paragraph.
///
/// What happens: an index is `INDEX ∪ of_equity(symbol)`. `NIFTY` is in
/// [`FNO_UNDERLYINGS`] as the *underlying of its options*, so the NIFTY spot
/// index carries `INDEX` and `FNO` together. `api::server`'s F&O filter
/// depends on that overlap: treating the two as disjoint drops NIFTY from the
/// one view that most needs it.
///
/// What the lookup does NOT do is make an index a share. No spot index appears
/// in [`NIFTY_TOTAL_MARKET`] or in any of the four tier arrays, so
/// `TOTAL_MARKET` and the tier bits stay clear for an index without anything
/// here having to special-case them —
/// `no_spot_index_is_a_constituent_of_an_equity_list` asserts that data fact and
/// `an_index_is_its_own_universe_and_a_live_derivative_is_in_none` asserts the
/// answer it produces. Being the underlying of a contract and being a
/// constituent of an index are different facts, read from different files.
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

    /// The longest walk `position` performs for a symbol the table HOLDS.
    ///
    /// Module-level rather than nested inside one test because the miss bound
    /// is measured beside it and the two must agree about what a step is. A
    /// second copy of this loop would be free to count differently and the
    /// comparison between the two numbers would then mean nothing -- the same
    /// argument `MemberIndex::contains` makes for delegating to `position`.
    ///
    /// The start index comes from `mask` and `fnv1a` themselves, never from a
    /// re-implementation: a copy is free to disagree with the thing under test
    /// and would then measure a probe nobody performs.
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

    /// The longest walk `position` performs for a symbol the table does NOT
    /// hold -- taken over every slot, so over every string that can miss.
    ///
    /// A miss cannot stop at a match, so it runs to the first EMPTY slot. Its
    /// cost therefore depends only on where it starts, and the worst start is
    /// the head of the longest run of occupied slots. Walking all `N` starts
    /// computes exactly that, which is why this is a BOUND and not a sample:
    /// no list of unlucky strings could beat it, and no lucky list could hide
    /// it. Sampling is how the hit number came to be quoted for both.
    fn worst_miss_probe<const N: usize>(idx: &MemberIndex<N>) -> usize {
        let mut worst = 0;
        for start in 0..N {
            let mut at = start;
            let mut steps = 1;
            // Terminates for the same two reasons `position` does: the table
            // is at most half full so an empty slot exists, and `N` is a power
            // of two so this step function can reach it. `MemberIndex::build`
            // asserts both.
            while idx.slots[at].is_some() {
                at = (at + 1) & (N - 1);
                steps += 1;
            }
            worst = worst.max(steps);
        }
        worst
    }

    #[test]
    fn the_probe_length_is_bounded_which_is_what_makes_it_o1() {
        // THE CLAIM UNDER TEST, AND ONLY HALF OF IT. `binary_search` cost ~10
        // comparisons and grew with the list; this must not grow at all.
        // Measured by walking the table the same way `contains` does and
        // counting the steps.
        //
        // The bound is asserted as a NUMBER, not as "small": a probe length
        // that crept up with a future member would otherwise pass silently.
        //
        // Every symbol here is one the table HOLDS, so every walk measured
        // below is a HIT, and this test says nothing about a miss. It was read
        // as though it did -- the `<= 8` it pins was quoted as the probe bound
        // in four places, and `of_equity`'s commonest input misses in all six
        // tables. The miss is bounded separately, and higher, by
        // `a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`.
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
    fn a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too() {
        // THE OTHER HALF OF THE BOUND, AND THE HALF THAT WAS NEVER MEASURED.
        //
        // `the_probe_length_is_bounded_which_is_what_makes_it_o1` walks the
        // table only for symbols it HOLDS. A hit stops at the matching slot; a
        // miss cannot, and runs on to the first EMPTY slot, so it is never the
        // shorter walk. The `<= 8` that test pins was quoted -- in this file's
        // header, on `MemberIndex`, on `position` and on `nse_isin` -- as
        // though it covered both.
        //
        // It does not, and the gap is not academic. `of_equity` probes six
        // tables and the question it is asked most is about a name outside
        // every tier: `crate::vendor::Skip::SmeBoard` declines 1,117 real
        // shares on exactly that ground, and every one of them is a miss six
        // times over.
        let ntm = worst_miss_probe(&NTM_INDEX);
        let fno = worst_miss_probe(&FNO_INDEX);
        let n500 = worst_miss_probe(&NIFTY_500_INDEX);
        let n200 = worst_miss_probe(&NIFTY_200_INDEX);
        let n100 = worst_miss_probe(&NIFTY_100_INDEX);
        let n50 = worst_miss_probe(&NIFTY_50_INDEX);

        // Pinned as a NUMBER for the same reason the hit bound is: a table
        // whose clusters merged would cost more without answering anything
        // differently, and nothing else in this crate would notice. 12 is one
        // step above the worst measured, which is the same headroom the hit
        // bound of 8 leaves over its worst of 7.
        for (name, slots, worst) in [
            ("NTM", 2048, ntm),
            ("FNO", 1024, fno),
            ("NIFTY 500", 2048, n500),
            ("NIFTY 200", 1024, n200),
            ("NIFTY 100", 512, n100),
            ("NIFTY 50", 256, n50),
        ] {
            assert!(
                worst <= 12,
                "{name} in {slots} slots must MISS in at most 12 steps, got {worst}"
            );
        }

        // A miss is never cheaper than a hit on the same table, which is why
        // quoting one number for both was wrong in the unsafe direction. It is
        // provable rather than lucky: a member sits inside the occupied run its
        // own start slot begins, so the hit stops at or before the empty slot
        // the miss walks to.
        for (name, hit, miss) in [
            ("NTM", worst_probe(&NTM_INDEX, &NIFTY_TOTAL_MARKET), ntm),
            ("FNO", worst_probe(&FNO_INDEX, &FNO_UNDERLYINGS), fno),
            ("NIFTY 500", worst_probe(&NIFTY_500_INDEX, &NIFTY_500), n500),
            ("NIFTY 200", worst_probe(&NIFTY_200_INDEX, &NIFTY_200), n200),
            ("NIFTY 100", worst_probe(&NIFTY_100_INDEX, &NIFTY_100), n100),
            ("NIFTY 50", worst_probe(&NIFTY_50_INDEX, &NIFTY_50), n50),
        ] {
            assert!(
                miss >= hit,
                "{name}: a miss ({miss}) cannot cost less than a hit ({hit})"
            );
        }

        // What `of_equity` actually costs when the answer is NONE: six misses.
        // The header quotes this sum, so it is computed here rather than added
        // up by hand in a comment that cannot be re-run.
        let six = ntm + fno + n500 + n200 + n100 + n50;
        assert!(
            six <= 72,
            "of_equity is six probes and each is bounded at 12, got {six}"
        );
        println!(
            "worst miss: NTM {ntm}, FNO {fno}, 500 {n500}, 200 {n200}, \
             100 {n100}, 50 {n50} -- of_equity total {six}"
        );

        // And the walk above is the walk `position` performs, not a model of
        // it: a symbol the table does not hold is answered `None`, having
        // ended at the empty slot this counted to.
        assert_eq!(NTM_INDEX.position("ZZZZNOTREAL"), None);
        assert!(NIFTY_50_INDEX.position("RELIANCE").is_some());
    }

    #[test]
    fn the_probe_reaches_every_slot_only_because_n_is_a_power_of_two() {
        // WHAT THE COMPILE-TIME ASSERTION IN `build` IS FOR, PROVED ON THE
        // ARITHMETIC RATHER THAN ON A TABLE.
        //
        // `build` and `position` both step with `at = (at + 1) & (N - 1)`.
        // That is a modulo -- and therefore a walk over every slot -- only when
        // `N - 1` is a solid run of low bits. The half-full assertion
        // guarantees an empty slot EXISTS; this is what guarantees the probe
        // can REACH it, and the type's doc used to rest the second on the
        // first.
        //
        // The real guard is the `const` assertion, because every table in this
        // workspace is a `static` and a `const` panic is a build failure: a bad
        // N cannot ship, and cannot be constructed here to walk either. So the
        // property is proved on the step function itself, and the refusal is
        // proved separately by
        // `a_table_size_that_is_not_a_power_of_two_is_refused`.
        fn slots_reached(n: usize, from: usize) -> usize {
            let mut seen = vec![false; n];
            let mut at = from;
            for _ in 0..n {
                seen[at] = true;
                at = (at + 1) & (n - 1);
            }
            seen.iter().filter(|s| **s).count()
        }

        // A power of two: every start reaches all N slots, so a probe always
        // finds the empty slot the half-full assertion guarantees exists.
        for n in [2_usize, 4, 8, 16, 256, 512, 1024, 2048] {
            for from in [0, 1, n / 2, n - 1] {
                assert_eq!(slots_reached(n, from), n, "N={n} starting at {from}");
            }
        }

        // N = 12 is the counter-example the assertion exists for, and it is the
        // one the finding named. `N - 1` is `0b1011`; from slot 8 the walk is
        // 9, 10, 11, then `12 & 11 == 8`. Four slots, forever, while eight sit
        // empty and unreachable below them -- `build` would spin and `position`
        // would answer `None` for a member the table holds.
        assert_eq!(
            slots_reached(12, 8),
            4,
            "the mask cycles instead of walking"
        );
        assert!(slots_reached(12, 8) < 12);
        // It is not only slot 8: no start on a 12-slot table sees everything.
        for from in 0..12 {
            assert!(slots_reached(12, from) < 12, "N=12 from {from}");
        }

        // Every table size this workspace builds satisfies the requirement --
        // the six here and the three in `crate::vendor` -- which is why adding
        // the assertion changed no behaviour and only closed the hole.
        for n in [2048_usize, 1024, 512, 256, 16, 8] {
            assert!(n.is_power_of_two(), "{n} is a shipped table size");
        }
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn a_table_size_that_is_not_a_power_of_two_is_refused() {
        // The assertion fires at COMPILE time for a `static`, which is how
        // every real table is declared and the form that actually guards the
        // invariant. That form cannot be tested from here: a `const` panic is
        // a build failure, and a build failure is not a red test.
        //
        // So the same call is made at RUNTIME, where the same assertion panics
        // and can be observed. Twelve slots for an empty list is the case that
        // shows the two assertions are not one: `0 * 2 <= 12` satisfies the
        // half-full rule, so before this change `build` accepted a table whose
        // probe could not walk.
        let _refused: MemberIndex<12> = MemberIndex::build(&[]);
    }

    #[test]
    fn no_spot_index_is_a_constituent_of_an_equity_list() {
        // THE DATA FACT `of_instrument` RESTS ON, now that its doc says what it
        // actually does. An index is `INDEX u of_equity(symbol)` -- the equity
        // lists ARE consulted for it -- and the reason that does not turn the
        // NIFTY spot index into a share is not a special case in the code. It
        // is that no index appears in any of the five constituent arrays.
        //
        // Checked rather than asserted in a comment, because the arrays are
        // transcriptions and the next refresh could add a row. If one ever
        // does, `of_instrument` starts stamping TOTAL_MARKET on a spot index
        // and nothing else in this crate would catch it.
        for idx in ["NIFTY", "BANKNIFTY", "FINNIFTY", "MIDCPNIFTY", "NIFTYNXT50"] {
            assert!(
                !NIFTY_TOTAL_MARKET.contains(&idx),
                "{idx} is an index, not a Total Market constituent"
            );
            assert!(!NIFTY_500.contains(&idx), "{idx} is not a NIFTY 500 name");
            assert!(!NIFTY_200.contains(&idx), "{idx} is not a NIFTY 200 name");
            assert!(!NIFTY_100.contains(&idx), "{idx} is not a NIFTY 100 name");
            assert!(!NIFTY_50.contains(&idx), "{idx} is not a NIFTY 50 name");
            // And it IS in the F&O list, which is the other half of the same
            // sentence: the underlying of a contract, not a share.
            assert!(
                FNO_UNDERLYINGS.contains(&idx),
                "{idx} is the underlying of its own options"
            );
            // So the bit set is exactly FNO -- no tier bit, and no INDEX bit,
            // because `of_equity` does not know it was handed an index.
            assert_eq!(
                of_equity(idx),
                Universe::FNO,
                "{idx} is an F&O underlying and nothing else"
            );
        }
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

    /// Every array and the ISIN array beside it, by name.
    ///
    /// A function rather than a `const`, so a tier appended without a matching
    /// ISIN array is a COMPILE error at the one place that lists them rather
    /// than a check that quietly walks five of six.
    fn tiers() -> [(
        &'static str,
        &'static [&'static str],
        &'static [&'static str],
    ); 6] {
        [
            ("NIFTY_50", NIFTY_50.as_slice(), NIFTY_50_ISIN.as_slice()),
            ("NIFTY_100", NIFTY_100.as_slice(), NIFTY_100_ISIN.as_slice()),
            ("NIFTY_200", NIFTY_200.as_slice(), NIFTY_200_ISIN.as_slice()),
            ("NIFTY_500", NIFTY_500.as_slice(), NIFTY_500_ISIN.as_slice()),
            (
                "NIFTY_TOTAL_MARKET",
                NIFTY_TOTAL_MARKET.as_slice(),
                NIFTY_TOTAL_MARKET_ISIN.as_slice(),
            ),
            (
                "FNO_UNDERLYINGS",
                FNO_UNDERLYINGS.as_slice(),
                FNO_UNDERLYINGS_ISIN.as_slice(),
            ),
        ]
    }

    #[test]
    fn the_isin_arrays_are_positionally_aligned_with_the_names() {
        // LENGTH. The types already say it -- `[&str; 50]` beside `[&str; 50]`
        // -- and it is asserted anyway, because the next tier is appended by
        // copying this pair and a mismatched literal length is the first way
        // that goes wrong.
        for (name, names, isins) in tiers() {
            assert_eq!(
                names.len(),
                isins.len(),
                "{name} and {name}_ISIN must be the same length"
            );
        }

        // ALIGNMENT. Index i must name the same instrument in both arrays --
        // which is a claim about a file that is not in this repository, so it
        // cannot be checked against the source here. What CAN be checked, and
        // is far stronger than a length: the six arrays were transcribed from
        // FIVE DIFFERENT published files, they overlap heavily, and every
        // symbol they share must carry the SAME ISIN in every one of them.
        //
        // A row that slipped by one anywhere -- a name inserted into an ISIN
        // column, a header counted as data, an off-by-one in any single
        // transcription -- shifts every entry below it and disagrees with the
        // other four files at the first shared symbol. 1,807 positions carry
        // an ISIN, they name 749 distinct symbols, and 1,058 of them are
        // therefore a second, third, fourth or fifth opinion on a symbol
        // another published file already named.
        let mut seen: Vec<(&str, &str, &str)> = Vec::new();
        for (name, names, isins) in tiers() {
            for (sym, isin) in names.iter().zip(isins.iter()) {
                if *isin == ISIN_ABSENT {
                    continue;
                }
                if let Some((_, other_isin, other_name)) =
                    seen.iter().find(|(s, _, _)| s == sym).copied()
                {
                    assert_eq!(
                        other_isin, *isin,
                        "{sym} is {other_isin} in {other_name} and {isin} in {name} \
                         -- one of the two transcriptions is off by a row"
                    );
                }
                seen.push((sym, isin, name));
            }
        }
        // The cross-check is only evidence if the arrays really do overlap.
        // Pinned as a NUMBER so a future edit that stopped them overlapping
        // would fail here rather than pass a vacuous loop.
        assert_eq!(seen.len(), 1807, "1,813 positions less the 6 absent ones");

        // And an ISIN may not appear twice within one array: two names sharing
        // an identifier is the other shape a shifted row takes.
        for (name, _, isins) in tiers() {
            let mut sorted: Vec<&str> = isins
                .iter()
                .copied()
                .filter(|i| *i != ISIN_ABSENT)
                .collect();
            sorted.sort_unstable();
            for w in sorted.windows(2) {
                assert_ne!(w[0], w[1], "{name} carries {} twice", w[0]);
            }
        }
    }

    #[test]
    fn every_transcribed_isin_is_well_formed() {
        // Three separate claims, and the weakest is the one stated first.
        //
        // 1. INE + nine. Every equity ISIN NSE issues begins `INE` -- `IN` is
        //    the ISO 3166 country code and `E` is the issuer type for a
        //    company. A value that does not is not an NSE equity ISIN, whatever
        //    else it may be, and `IN1520250085` -- the state development loan
        //    `crate::isin` names -- is exactly the shape this catches.
        // 2. Twelve characters. ISO 6166 fixes it.
        // 3. The CHECK DIGIT verifies. This is the one that turns a
        //    transcription slip into a build failure: a mistyped character
        //    produces a value that still looks like an ISIN and fails here.
        //    `Isin::new` computes it; nothing in this test recomputes it, so
        //    there is no second copy of that arithmetic free to disagree.
        for (name, names, isins) in tiers() {
            for (sym, isin) in names.iter().zip(isins.iter()) {
                if *isin == ISIN_ABSENT {
                    continue;
                }
                assert!(
                    isin.starts_with("INE"),
                    "{name}[{sym}] = {isin} is not an NSE equity ISIN"
                );
                assert_eq!(isin.len(), 12, "{name}[{sym}] = {isin} is not 12 chars");
                assert!(
                    Isin::new(isin).is_ok(),
                    "{name}[{sym}] = {isin} fails the ISO 6166 check digit"
                );
            }
        }
    }

    #[test]
    fn the_absent_isins_are_exactly_these_six_names() {
        // Named, not counted. "six are absent" would still pass if a different
        // six went absent tomorrow, and the whole point of transcribing this
        // column was to stop identity resting on an unnamed hop.
        let mut absent: Vec<&str> = Vec::new();
        for (_, names, isins) in tiers() {
            for (sym, isin) in names.iter().zip(isins.iter()) {
                if *isin == ISIN_ABSENT {
                    absent.push(sym);
                }
            }
        }
        absent.sort_unstable();
        let mut expected = vec![
            // Five indices. Not securities, so no numbering agency issues them
            // an ISIN -- absence here is the CORRECT answer, and the same five
            // the F&O note records as having no cross-vendor ISIN check.
            "BANKNIFTY",
            "FINNIFTY",
            "MIDCPNIFTY",
            "NIFTY",
            "NIFTYNXT50",
            // And the one real gap: the exchange's Total Market file has no
            // row for `AGL`. `docs/06-limits.md` §11 carries it, and this
            // build still does not know what instrument it is.
            "AGL",
        ];
        expected.sort_unstable();
        assert_eq!(
            absent, expected,
            "the absent set is a MEASUREMENT and it changed"
        );
    }

    #[test]
    fn the_placeholder_scrips_are_malformed_and_are_not_here() {
        // The published Total Market file has 752 rows; two are placeholders
        // mirroring `INOXGREEN` and `TRIVENI`, and they carry `DUM`-prefixed
        // pseudo-ISINs. They are malformed by CONSTRUCTION and this proves it
        // rather than asserting it -- both fail the ISO 6166 check digit, so
        // even a build that somehow admitted the symbol could not admit the
        // identifier.
        for fake in ["DUM510W01014", "DUM256C01024"] {
            assert!(!fake.starts_with("INE"), "{fake} is not an NSE equity ISIN");
            assert!(Isin::new(fake).is_err(), "{fake} must fail the check digit");
        }
        // And neither the symbol nor the pseudo-ISIN reached any array.
        for (name, names, isins) in tiers() {
            for dummy in ["DUMMYINXGN", "DUMMYTRVN"] {
                assert!(!names.contains(&dummy), "{dummy} must not be in {name}");
                assert_eq!(nse_isin(dummy), None, "{dummy} must never resolve");
            }
            for fake in ["DUM510W01014", "DUM256C01024"] {
                assert!(!isins.contains(&fake), "{fake} must not be in {name}_ISIN");
            }
        }
    }

    #[test]
    fn a_position_probe_finds_the_index_the_table_was_built_from() {
        // `contains` answers whether; `position` answers where, and the ISIN
        // arrays are useless without the second. Exhaustive over all six
        // tables: a table that found a member but returned the WRONG index
        // would hand back another company's ISIN, which is the single worst
        // failure this file can produce.
        for (m, at) in NIFTY_TOTAL_MARKET.iter().zip(0..) {
            assert_eq!(NTM_INDEX.position(m), Some(at), "{m}");
        }
        for (m, at) in FNO_UNDERLYINGS.iter().zip(0..) {
            assert_eq!(FNO_INDEX.position(m), Some(at), "{m}");
        }
        for (m, at) in NIFTY_500.iter().zip(0..) {
            assert_eq!(NIFTY_500_INDEX.position(m), Some(at), "{m}");
        }
        for (m, at) in NIFTY_200.iter().zip(0..) {
            assert_eq!(NIFTY_200_INDEX.position(m), Some(at), "{m}");
        }
        for (m, at) in NIFTY_100.iter().zip(0..) {
            assert_eq!(NIFTY_100_INDEX.position(m), Some(at), "{m}");
        }
        for (m, at) in NIFTY_50.iter().zip(0..) {
            assert_eq!(NIFTY_50_INDEX.position(m), Some(at), "{m}");
        }

        // A miss ends at an empty slot.
        assert_eq!(NTM_INDEX.position("ZZZZNOTREAL"), None);
        // And the length guard `contains` documents is the same guard here:
        // a string too long to be a `Symbol` is refused before it is hashed.
        let too_long = "A".repeat(crate::symbol::SYMBOL_CAPACITY + 1);
        assert_eq!(NTM_INDEX.position(&too_long), None);

        // Built at runtime rather than at compile time, for the same reason
        // `the_table_builder_is_exercised_at_runtime_not_only_at_compile_time`
        // does it: `ords` is filled by a `const fn` the coverage
        // instrumentation never enters.
        let idx: MemberIndex<8> = MemberIndex::build(&["A", "BB", "CCC"]);
        assert_eq!(idx.position("A"), Some(0));
        assert_eq!(idx.position("BB"), Some(1));
        assert_eq!(idx.position("CCC"), Some(2));
        assert_eq!(idx.position("D"), None);
    }

    #[test]
    fn nse_isin_answers_from_the_exchanges_own_column() {
        // A share, and the value is the one the exchange's file prints.
        assert_eq!(
            nse_isin("RELIANCE").map(|i| i.to_string()).as_deref(),
            Some("INE002A01018")
        );
        assert_eq!(
            nse_isin("TCS").map(|i| i.to_string()).as_deref(),
            Some("INE467B01029")
        );
        // The narrowest tier resolves through the widest array.
        for m in NIFTY_50 {
            assert!(nse_isin(m).is_some(), "{m} is a NIFTY 50 constituent");
        }
        // An index carries no ISIN, and neither does the one name the
        // exchange's file has no row for.
        for none in ["NIFTY", "BANKNIFTY", "FINNIFTY", "MIDCPNIFTY", "NIFTYNXT50"] {
            assert_eq!(nse_isin(none), None, "{none} is an index");
        }
        assert_eq!(nse_isin("AGL"), None, "the file has no row for AGL");
        // Not a constituent.
        assert_eq!(nse_isin("ZZZZNOTREAL"), None);
        assert_eq!(nse_isin(""), None);

        // EVERY name that resolves, resolves to the array's own entry -- so
        // this function cannot drift from the data it reads.
        for (sym, isin) in NIFTY_TOTAL_MARKET
            .iter()
            .zip(NIFTY_TOTAL_MARKET_ISIN.iter())
        {
            assert_eq!(
                nse_isin(sym).map(|i| i.to_string()).as_deref(),
                (*isin != ISIN_ABSENT).then_some(*isin),
                "{sym}"
            );
        }
    }

    #[test]
    fn union_is_idempotent_on_a_bit_both_sides_already_hold() {
        // Overlap is the ONLY case that separates union from symmetric
        // difference: on disjoint sets `|` and `^` return the same answer,
        // and every other union test here happens to use disjoint sets. A
        // stock that is both an F&O underlying and a Total Market
        // constituent is the real shape this protects.
        let fno = Universe::FNO;
        let both = Universe::FNO.union(Universe::TOTAL_MARKET);

        assert_eq!(fno.union(fno), fno, "union with itself is itself");
        assert_eq!(
            both.union(fno),
            both,
            "re-adding a bit the set already holds must not clear it"
        );
        assert!(
            both.union(fno).contains(Universe::FNO),
            "FNO survived being added twice"
        );
    }
}
