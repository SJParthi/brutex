# costs-table.json (copied as Markdown: the repository tracks .json only under .claude/)

```json
{
 "meta": {
  "version": "costs-table/2026-10-30/r1",
  "scope": "NSE cash equity, intraday (MIS), F&O stocks; operator broker Dhan",
  "window": [
   "2018-09-03",
   "2026-10-30"
  ],
  "read_on": "2026-10-30",
  "read_via": "curl + pdftotext (PDFs), bsdtar (zips), curl (HTML), NSE circular-list API for subject searches; image PDFs FA50832 and FA53913 read visually; sha256 of every document in costs-evidence/sha256-official.txt",
  "clocks": {
   "dated": "rates in force on the trade date (rows with clocks containing \"dated\" whose [from, to] covers the date)",
   "today": "rates as Dhan charges on 2026-10-30 applied to the trade (rows with clocks containing \"today\")"
  },
  "headline_rule": "for each trade, the larger of the dated worst-reading total and the today worst-reading total (operator rule 7); both totals shown",
  "dhan_rounding_quote": "Stamp Duty and STT are rounded off to the nearest rupee and all other charges to the nearest 2 decimals. (dhan.co/pricing, read 2026-10-30)",
  "label": "Cost table, not a backtest result. Backtest results priced with it carry: current F&O members only (survivorship bias not removed)",
  "units": "rate_per_crore in rupees per crore of the base; rate_pct in percent; dates inclusive; null = open or unknown (see from_note)"
 },
 "rows": [
  {
   "id": "BRK-1",
   "component": "Brokerage (Dhan), equity intraday",
   "rate": "min(Rs 20, 0.03% of the order value) per executed order",
   "rate_pct": 0.03,
   "cap_rs": 20,
   "base": "value of each executed order",
   "side": "both (buy order and sell order each)",
   "from": null,
   "from_note": "UNVERIFIED: Dhan page is undated; archived Dhan page of 2023-02-01 shows the same rate (sources-charges.json B-01); operator decision 13 Oct 2026 applies it to every date",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "2 decimals (Dhan page note 1)",
   "source": {
    "doc": "Dhan pricing page, Equity tab",
    "url": "https://dhan.co/pricing/",
    "read": "2026-10-30",
    "quote": "₹20 or 0.03% of trade value per executed order, whichever is lower.",
    "status": "PRIMARY (broker)"
   }
  },
  {
   "id": "BRK-SQ",
   "component": "Dhan auto square-off fee",
   "rate": "Rs 20 per order + GST 18%",
   "base": "each order Dhan places to square off an open intraday position",
   "side": "the square-off order",
   "from": null,
   "from_note": "UNVERIFIED (undated page)",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "exact",
   "source": {
    "doc": "Dhan pricing page, Other Charges",
    "url": "https://dhan.co/pricing/",
    "read": "2026-10-30",
    "quote": "₹ 20 / order + GST. For intraday order automatically squared by our team",
    "status": "PRIMARY (broker)"
   },
   "notes": "Time: Dhan support page (read 2026-10-30): equity cash intraday auto square-off at 3:10 PM from the day of CAS application. https://dhan.co/support/general/market-session-status-and-timing/what-are-the-dhan-intraday-auto-square-off-timings/"
  },
  {
   "id": "BRK-DP",
   "component": "DP transaction charge (Dhan)",
   "rate": "Rs 12.50 per instruction per ISIN + GST 18% = Rs 14.75",
   "base": "per scrip per day with a delivery sell",
   "side": "sell",
   "from": null,
   "from_note": "UNVERIFIED (undated page)",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "exact",
   "source": {
    "doc": "Dhan pricing page, Demat Account Related Charges; Dhan support FAQ on DP charges",
    "url": "https://dhan.co/pricing/",
    "read": "2026-10-30",
    "quote": "₹ 12.50 / instruction / ISIN + GST.",
    "status": "PRIMARY (broker); applicability to intraday UNVERIFIED"
   },
   "notes": "Precise reading: not charged on a same-day squared-off position (no demat debit). Worst reading (operator rule 7): charged once per scrip-day with a sell, until a Dhan contract note settles it."
  },
  {
   "id": "BRK-CT",
   "component": "Call & trade (Dhan)",
   "rate": "Rs 50 per order + GST",
   "base": "orders placed through the trade desk",
   "side": "either",
   "from": null,
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "exact",
   "source": {
    "doc": "Dhan pricing page",
    "url": "https://dhan.co/pricing/",
    "read": "2026-10-30",
    "quote": "₹ 50 / order + GST. For every order placed through our trade desk.",
    "status": "PRIMARY (broker)"
   },
   "notes": "Not used: the strategy places orders by API."
  },
  {
   "id": "EX-2009",
   "component": "NSE transaction charge, cash market",
   "rate": "member monthly-turnover slabs Rs 3.25 / 3.20 / 3.15 / 3.10 / 3.05 / 3.00 per lakh each side; top slab Rs 325 per crore used (worst for a client)",
   "rate_per_crore": 325,
   "base": "traded value (turnover), each side",
   "side": "both",
   "from": "2009-10-01",
   "to": "2020-12-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/F&A/13028, 7 Sep 2009; confirmed as the \"Existing\" column of NSE/FA/46730",
    "url": "https://nsearchives.nseindia.com/content/circulars/faac13028.pdf",
    "read": "2026-10-30",
    "quote": "Up to First Rs. 1250 cores Rs. 3.25 each side",
    "status": "PRIMARY"
   },
   "notes": "An incremental-volume concession (NSE/FA/24578) ran 2013-10-01 to 2014-05-31 and was discontinued by NSE/FA/26775. Slabs apply to the member; which slab a client was passed is not recorded, so the worst (top) slab is used."
  },
  {
   "id": "EX-2020C",
   "component": "NSE transaction charge, cash market, concession for EQ-series stocks outside NIFTY 50 and NIFTY Next 50 (not GSM)",
   "rate": "Rs 2.75 per lakh each side up to Rs 7,500 crore a month (to 2022-03-31) or Rs 12,500 crore (from 2022-04-01); above it Rs 1.75 (to 2022-03-31) or Rs 2.25",
   "rate_per_crore": 275,
   "base": "traded value of eligible securities",
   "side": "both",
   "from": "2020-08-01",
   "to": "2023-03-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/45147, 46225, 47844, 49796, 50832, 51855, 53913",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA53913.pdf",
    "read": "2026-10-30",
    "quote": "applicable for a period of six months from October 1, 2022 to March 31, 2023",
    "status": "PRIMARY"
   },
   "notes": "Informational. Lower than the regular slab, so never the worst reading. No extension after 2023-03-31 was found in a subject search of the NSE circular list (absence is not proof). FA50832 and FA53913 are image PDFs, read visually."
  },
  {
   "id": "EX-2021",
   "component": "NSE transaction charge, cash market",
   "rate": "slabs Rs 3.45 / 3.40 / 3.35 / 3.30 / 3.25 / 3.20 per lakh each side; top slab Rs 345 per crore used",
   "rate_per_crore": 345,
   "base": "traded value, each side",
   "side": "both",
   "from": "2021-01-01",
   "to": "2023-03-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/46730, 18 Dec 2020",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA46730.pdf",
    "read": "2026-10-30",
    "quote": "will be applicable with effect from January 01, 2021.",
    "status": "PRIMARY"
   }
  },
  {
   "id": "EX-2023",
   "component": "NSE transaction charge, cash market",
   "rate": "slabs Rs 3.25 .. 3.00 per lakh each side; top slab Rs 325 per crore used",
   "rate_per_crore": 325,
   "base": "traded value, each side",
   "side": "both",
   "from": "2023-04-01",
   "to": "2024-03-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/56129, 24 Mar 2023",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA56129.pdf",
    "read": "2026-10-30",
    "quote": "will be applicable with effect from April 01, 2023.",
    "status": "PRIMARY"
   }
  },
  {
   "id": "EX-2024A",
   "component": "NSE transaction charge, cash market",
   "rate": "slabs Rs 3.22 .. 2.97 per lakh each side; top slab Rs 322 per crore used",
   "rate_per_crore": 322,
   "base": "traded value, each side",
   "side": "both",
   "from": "2024-04-01",
   "to": "2024-09-30",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/61137, 14 Mar 2024",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA61137.pdf",
    "read": "2026-10-30",
    "quote": "will be applicable with effect from April 01, 2024.",
    "status": "PRIMARY"
   }
  },
  {
   "id": "EX-2024B",
   "component": "NSE transaction charge, cash market",
   "rate": "flat Rs 2.97 per lakh each side (Rs 297 per crore), uniform for all members, true to label",
   "rate_per_crore": 297,
   "base": "traded value, each side",
   "side": "both",
   "from": "2024-10-01",
   "to": "2026-02-28",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/64232, 27 Sep 2024 (SEBI/HO/MRD/TPD-1/P/CIR/2024/92)",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA64232.pdf",
    "read": "2026-10-30",
    "quote": "Cash Market: Rs. 2.97 each side per lakh of traded value",
    "status": "PRIMARY"
   }
  },
  {
   "id": "EX-2026",
   "component": "NSE transaction charge, cash market",
   "rate": "Rs 306.99 per crore each side",
   "rate_per_crore": 306.99,
   "base": "traded value, each side",
   "side": "both",
   "from": "2026-03-01",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/73061, 27 Feb 2026; Dhan page \"NSE: 0.0030699%\"",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA73061.pdf",
    "read": "2026-10-30",
    "quote": "The above revision will be applicable with effect from March 1, 2026.",
    "status": "PRIMARY"
   }
  },
  {
   "id": "IP-OLD",
   "component": "Contribution to NSE IPFT, cash market",
   "rate": "Rs 0.01 per crore each side",
   "rate_per_crore": 0.01,
   "base": "traded value, each side",
   "side": "both",
   "from": null,
   "from_note": "UNVERIFIED start; FA56129 calls Rs 0.01 the \"existing\" level on 24 Mar 2023 and FA73061 the level \"levied prior to April 1, 2023\"; no earlier circular found by subject search",
   "to": "2023-03-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/56129",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA56129.pdf",
    "read": "2026-10-30",
    "quote": "from existing Rs.0.01 per crore of the traded value to Rs.10",
    "status": "PRIMARY (level); start date UNVERIFIED"
   },
   "notes": "Rs 0.001 on Rs 10 lakh of turnover: below one paisa. Applied on every date in the worst reading."
  },
  {
   "id": "IP-2023",
   "component": "Contribution to NSE IPFT, cash market",
   "rate": "Rs 10 per crore each side",
   "rate_per_crore": 10,
   "base": "traded value, each side",
   "side": "both",
   "from": "2023-04-01",
   "to": "2026-02-28",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/56129",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA56129.pdf",
    "read": "2026-10-30",
    "quote": "Rs.10 per crore of the traded value in Cash Equities Market Segment",
    "status": "PRIMARY"
   }
  },
  {
   "id": "IP-2026",
   "component": "Contribution to NSE IPFT, cash market",
   "rate": "Rs 0.01 per crore each side",
   "rate_per_crore": 0.01,
   "base": "traded value, each side",
   "side": "both",
   "from": "2026-03-01",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FA/73061; Dhan page \"IPFT Contribution 0.0000001% of the turnover\"",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA73061.pdf",
    "read": "2026-10-30",
    "quote": "Rs. 0.01 each side",
    "status": "PRIMARY"
   }
  },
  {
   "id": "SF-2017",
   "component": "SEBI turnover fee, cash segment",
   "rate": "0.00015% of the price at which securities are purchased or sold (Rs 15 per crore)",
   "rate_per_crore": 15,
   "base": "buy value and sell value (turnover)",
   "side": "both",
   "from": "2017-03-06",
   "from_note": "regulations in force on publication in the Gazette, 6 Mar 2017",
   "to": "2019-03-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "SEBI/LAD/NRO/GN/2016-17/037, 6 Mar 2017, annexed to NSE/FA/34501",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA34501.zip",
    "read": "2026-10-30",
    "quote": "0.00015 per cent of the price at which the securities are purchased or sold",
    "status": "PRIMARY"
   }
  },
  {
   "id": "SF-2019",
   "component": "SEBI turnover fee, cash segment",
   "rate": "0.00010% of turnover (Rs 10 per crore)",
   "rate_per_crore": 10,
   "base": "turnover",
   "side": "both",
   "from": "2019-04-01",
   "to": "2020-05-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "SEBI/L.A.D.-N.R.O./G.N./2019/03, 22 Mar 2019, annexed to NSE/FA/40632",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA40632.pdf",
    "read": "2026-10-30",
    "quote": "They shall come into force with effect from April 01, 2019.",
    "status": "PRIMARY"
   }
  },
  {
   "id": "SF-2020",
   "component": "SEBI turnover fee, cash segment, COVID relief",
   "rate": "50% of the fee: Rs 5 per crore",
   "rate_per_crore": 5,
   "base": "turnover",
   "side": "both",
   "from": "2020-06-01",
   "to": "2021-03-31",
   "clocks": [
    "dated"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "SEBI/LAD-NRO/GN/2020/011, 8 May 2020, annexed to NSE/FA/44514",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA44514.pdf",
    "read": "2026-10-30",
    "quote": "pay only 50% (fifty percent) of fees as calculated therein",
    "status": "PRIMARY"
   },
   "notes": "The relief is to the broker; whether a broker passed Rs 5 or Rs 10 to clients is not recorded. Today's clock (Rs 10) is the worse and wins the headline."
  },
  {
   "id": "SF-2021",
   "component": "SEBI turnover fee, cash segment",
   "rate": "Rs 10 per crore",
   "rate_per_crore": 10,
   "base": "turnover",
   "side": "both",
   "from": "2021-04-01",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "SF-2019 rate after the 2020 relief lapsed; NSE levies page (updated 17/04/2026); Dhan page \"0.0001% of the turnover\"",
    "url": "https://www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies",
    "read": "2026-10-30",
    "quote": "0.0001 per cent of the price at which the securities are purchased or sold (Rs.10 per crore)",
    "status": "PRIMARY"
   }
  },
  {
   "id": "STT-ND",
   "component": "STT, equity sold otherwise than by delivery (intraday)",
   "rate": "0.025%",
   "rate_pct": 0.025,
   "base": "NSE method: min(day buy qty, day sell qty) x VWAP of all the client's trades in the security that day, VWAP rounded to 2 decimals",
   "side": "sell (seller pays)",
   "from": "2013-06-01",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "each security to 2 decimals; client total for the day to the nearest rupee, 50 paise up (NSE/CMTR/5588 section D)",
   "source": {
    "doc": "Finance Act 2013 via NSE/FATAX/23500; unchanged in FATAX/56235 (2023), 63809 (2024), 73524 (2026); method NSE/CMTR/5588",
    "url": "https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf",
    "read": "2026-10-30",
    "quote": "settled otherwise than by the actual delivery or transfer of such share or unit",
    "status": "PRIMARY"
   },
   "notes": "Dhan page: \"0.025% on sell transactions\"; Dhan calculator applies it to the sell value. Both bases are computed and the worse is used (operator rule 7)."
  },
  {
   "id": "SD-STATE",
   "component": "Stamp duty under each State's stamp law, on the contract note",
   "rate": "state-specific; e.g. Maharashtra 0.002%, Tamil Nadu 0.006%, Telangana 0.01% max Rs 100/day (secondary table, all states in COSTS.md)",
   "base": "contract-note value (buy and sell)",
   "side": "both (PIB: payable by both seller and buyer)",
   "from": null,
   "to": "2020-06-30",
   "clocks": [
    "dated"
   ],
   "rounding": "UNVERIFIED",
   "source": {
    "doc": "PIB release 30 Jun 2020 (PRID 1635399) for the two-sided incidence; rates from Zerodha's archived calculator script (snapshot 2019-04-03), SECONDARY",
    "url": "https://www.pib.gov.in/PressReleasePage.aspx?PRID=1635399",
    "read": "2026-10-30",
    "quote": "stamp duty was payable by both seller and buyer",
    "status": "UNVERIFIED (rates secondary; client state unknown)"
   },
   "state_rates": [
    {
     "state": "Andhra Pradesh, Odisha",
     "rate_pct": 0.005,
     "cap_rs": 50
    },
    {
     "state": "Arunachal Pradesh",
     "rate_pct": 0.003
    },
    {
     "state": "Assam",
     "rate_pct": 0.018,
     "cap_rs": 49.5
    },
    {
     "state": "Delhi, Gujarat, Kerala, Maharashtra",
     "rate_pct": 0.002
    },
    {
     "state": "Goa",
     "rate_pct": 0.005
    },
    {
     "state": "Haryana",
     "rate_pct": 0.002,
     "cap_rs": 200
    },
    {
     "state": "Himachal Pradesh",
     "flat_rs_per_day": 50
    },
    {
     "state": "Karnataka, Meghalaya, Others",
     "rate_pct": 0.003
    },
    {
     "state": "Madhya Pradesh",
     "rate_pct": 0.002
    },
    {
     "state": "Rajasthan",
     "rate_pct": 0.003
    },
    {
     "state": "Tamil Nadu",
     "rate_pct": 0.006
    },
    {
     "state": "Uttar Pradesh",
     "rate_pct": 0.002,
     "cap_rs_per_day": 1000
    },
    {
     "state": "West Bengal",
     "rate_pct": 0.002
    },
    {
     "state": "Telangana",
     "rate_pct": 0.01,
     "cap_rs_per_day": 100
    },
    {
     "state": "Jammu and Kashmir",
     "rate_pct": 0.003
    }
   ],
   "state_rates_source": "http://web.archive.org/web/20190403072919js_/https://zerodha.com/static/app.js?v=7.0.7 (EQ Intraday column; state codes from the 2018-07-14 snapshot of zerodha.com/brokerage-calculator); SECONDARY; extracts in costs-evidence/zerodha-archived-state-*.txt",
   "notes": "Worst reading: the highest of the listed states on the day's turnover (Telangana Rs 100 cap for a Rs 10 lakh day; Tamil Nadu 0.006% uncapped above Rs 16.67 lakh a day). Settle by the client's state of residence and that state's schedule."
  },
  {
   "id": "SD-ND",
   "component": "Stamp duty, transfer of security other than debenture on non-delivery basis",
   "rate": "0.003%",
   "rate_pct": 0.003,
   "base": "FIFO buy value of the non-deliverable quantity (for a squared-off day: the whole day's buy value)",
   "side": "buy (buyer pays)",
   "from": "2020-07-01",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "each security to 2 decimals; client total for the day to the nearest rupee (NCL/CMPT/43115 Annexure 1 item 2; Stamp Rules 2019 rule 7(3))",
   "source": {
    "doc": "Indian Stamp Act 1899 as amended by Finance Act 2019; Indian Stamp (Collection ...) Rules 2019 (G.S.R. 901(E)); NCL/CMPT/43115; start moved to 1 Jul 2020 by Gazette of 30 Mar 2020 (NCL/CMPT/44032)",
    "url": "https://nsearchives.nseindia.com/content/circulars/CMPT43115.zip",
    "read": "2026-10-30",
    "quote": "Transfer of Security other than debenture on non-delivery basis 0.003% Buyer",
    "status": "PRIMARY"
   }
  },
  {
   "id": "GST-1",
   "component": "GST on broker and exchange services",
   "rate": "18% (CGST 9% + SGST 9%, or IGST 18%)",
   "rate_pct": 18,
   "base": "brokerage + NSE transaction charge + IPFT (+ Dhan square-off fee, DP charge when charged)",
   "side": "both",
   "from": "2017-07-01",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "Notification 11/2017-Central Tax (Rate), Sl. 15 heading 9971 (v): 9% central tax; NSE levies page \"Stock brokers Services 18.00% (GST)\"; Dhan page",
    "url": "https://gstcouncil.gov.in/sites/default/files/2024-05/download_4.pdf",
    "read": "2026-10-30",
    "quote": "This notification shall come into force with effect from 1st day of July, 2017.",
    "status": "PRIMARY (SGST/IGST notifications not read; GST on the exchange charge rests on Dhan's statement)"
   }
  },
  {
   "id": "GST-SEBI",
   "component": "GST on the SEBI turnover fee",
   "rate": "18%",
   "rate_pct": 18,
   "base": "SEBI turnover fee",
   "side": "both",
   "from": "2022-07-18",
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "2 decimals",
   "source": {
    "doc": "NSE/FATAX/52993, 18 Jul 2022 (Notification 04/2022-Central Tax (Rate), 13 Jul 2022)",
    "url": "https://nsearchives.nseindia.com/content/circulars/FATAX52993.pdf",
    "read": "2026-10-30",
    "quote": "levy GST @ 18% on the fees payable to SEBI, from their clients from July 18, 2022",
    "status": "PRIMARY"
   },
   "notes": "Before 2022-07-18 the dated clock puts no GST on the SEBI fee; today's clock always does."
  },
  {
   "id": "OT-CLR",
   "component": "Clearing charge",
   "rate": "none found",
   "base": "-",
   "side": "-",
   "from": null,
   "to": null,
   "clocks": [
    "dated",
    "today"
   ],
   "rounding": "-",
   "source": {
    "doc": "NSE/FA/64232 and FA73061 list only transaction charges and IPFT; Dhan page lists none; subject search of NSE circulars 2008-2026",
    "url": "https://nsearchives.nseindia.com/content/circulars/FA73061.pdf",
    "read": "2026-10-30",
    "quote": "levy the transaction charges and contribution to NSE IPFT",
    "status": "UNVERIFIED (absence is not proof)"
   }
  },
  {
   "id": "OT-INT",
   "component": "Interest on intraday leverage",
   "rate": "none for a position squared off the same day (inference)",
   "base": "-",
   "side": "-",
   "from": null,
   "to": null,
   "clocks": [
    "today"
   ],
   "rounding": "-",
   "source": {
    "doc": "Dhan page lists interest only on non-MTF debit and 50:50 shortfall",
    "url": "https://dhan.co/pricing/",
    "read": "2026-10-30",
    "quote": "0.0438% per day on outstanding non MTF debit",
    "status": "UNVERIFIED"
   }
  }
 ],
 "readings": [
  {
   "id": "RD-BRK",
   "component": "brokerage",
   "readings": [
    "per executed order: min(20, 0.03% x order value)",
    "per fill (each traded GDFL second in which the order filled): sum of min(20, 0.03% x fill value)"
   ],
   "worst": "max; per fill is the larger whenever an order fills in more than one second",
   "settles_it": "a Dhan contract note for one order filled in several trades"
  },
  {
   "id": "RD-STT-BASE",
   "component": "STT base",
   "readings": [
    "NSE/CMTR/5588: squared quantity x day VWAP of all trades (rounded to 2 decimals)",
    "Dhan page and calculator: 0.025% of the sell value",
    "sell value per sell order, each rounded to the rupee"
   ],
   "worst": "max of the three",
   "settles_it": "a Dhan contract note with buy and sell at different prices"
  },
  {
   "id": "RD-ROUND-TAX",
   "component": "STT and stamp rounding level",
   "readings": [
    "client total for the day to the rupee (NSE, NCL, Stamp Rules 7(3))",
    "each security-day to the rupee",
    "each order (leg) to the rupee"
   ],
   "worst": "max",
   "settles_it": "a Dhan contract note with two or more stocks on one day"
  },
  {
   "id": "RD-ROUND-EX",
   "component": "exchange charge and SEBI fee rounding",
   "readings": [
    "once per client-security-day (one line)",
    "transaction charge and IPFT rounded as separate lines",
    "once per order"
   ],
   "worst": "max",
   "settles_it": "a Dhan contract note"
  },
  {
   "id": "RD-GST",
   "component": "GST rounding",
   "readings": [
    "18% of the base, rounded once",
    "CGST 9% and SGST 9% each rounded to 2 decimals"
   ],
   "worst": "max (differs by at most one paisa)",
   "settles_it": "a Dhan contract note"
  },
  {
   "id": "RD-DP",
   "component": "DP charge on intraday",
   "readings": [
    "none (no demat debit on a squared-off day)",
    "Rs 12.50 + GST once per scrip-day with a sell"
   ],
   "worst": "Rs 14.75",
   "settles_it": "a Dhan contract note or ledger for an intraday-only day"
  },
  {
   "id": "RD-SQ",
   "component": "auto square-off fee",
   "readings": [
    "Rs 20 + GST on top of the order's brokerage",
    "Rs 20 + GST instead of brokerage"
   ],
   "worst": "on top",
   "settles_it": "a Dhan contract note with an auto square-off"
  },
  {
   "id": "RD-STATE",
   "component": "stamp duty before 2020-07-01",
   "readings": [
    "each listed state's rate on the day's contract-note value"
   ],
   "worst": "the highest listed state for that day",
   "settles_it": "the operator names the client's state of residence for 2018-2020; that state's schedule"
  },
  {
   "id": "RD-SLAB",
   "component": "NSE transaction charge before 2024-10-01",
   "readings": [
    "each member slab",
    "the concession for eligible stocks (2020-08-01 to 2023-03-31)"
   ],
   "worst": "top (first) slab",
   "settles_it": "not needed: worst-case rule"
  },
  {
   "id": "RD-TIE",
   "component": "quantity at an exact tie (Rs 5 lakh / price = n + 0.5)",
   "readings": [
    "n",
    "n + 1"
   ],
   "worst": "n + 1 for charges and exposure (the spec must pin one; CN22 and CN23 price both)",
   "settles_it": "operator decision"
  }
 ],
 "test_vectors": [
  {
   "id": "CN01",
   "date": "2018-09-03",
   "why": "window start; SEBI Rs 15/cr; NSE Rs 325/cr top slab; state stamp duty",
   "ref_price": 1250,
   "quantity": 400,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 400,
       "price": 1250
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 400,
       "price": 1262.5
      }
     ]
    }
   ],
   "buy_value": 500000,
   "sell_value": 505000,
   "turnover": 1005000,
   "vwap": 1256.25,
   "gross": 5000,
   "dated_precise": {
    "brk": 40,
    "ex": 32.66,
    "sebi": 1.51,
    "stt": 126,
    "stamp": 100,
    "gst": 13.08,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 313.25
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.66,
    "sebi": 1.51,
    "stt": 126,
    "stamp": 100,
    "gst": 13.08,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 328
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.85,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.79
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.85,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.94,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 240.55
   },
   "headline": 328,
   "net": 4672
  },
  {
   "id": "CN02",
   "date": "2019-03-29",
   "why": "last trading day before SEBI 15 -> 10 (short trade)",
   "ref_price": 812.35,
   "quantity": 615,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 615,
       "price": 812.35
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 615,
       "price": 805
      }
     ]
    }
   ],
   "buy_value": 495075,
   "sell_value": 499595.25,
   "turnover": 994670.25,
   "vwap": 808.68,
   "gross": 4520.25,
   "dated_precise": {
    "brk": 40,
    "ex": 32.33,
    "sebi": 1.49,
    "stt": 124,
    "stamp": 99.47,
    "gst": 13.02,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 310.31
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.33,
    "sebi": 1.49,
    "stt": 125,
    "stamp": 99.47,
    "gst": 13.02,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 326.06
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.54,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.41
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.54,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.88,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239.17
   },
   "headline": 326.06,
   "net": 4194.19
  },
  {
   "id": "CN03",
   "date": "2019-04-01",
   "why": "first day SEBI Rs 10/cr (losing long)",
   "ref_price": 2345.65,
   "quantity": 213,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 213,
       "price": 2345.65
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 213,
       "price": 2330.1
      }
     ]
    }
   ],
   "buy_value": 499623.45,
   "sell_value": 496311.3,
   "turnover": 995934.75,
   "vwap": 2337.88,
   "gross": -3312.15,
   "dated_precise": {
    "brk": 40,
    "ex": 32.37,
    "sebi": 1,
    "stt": 124,
    "stamp": 99.59,
    "gst": 13.03,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 309.99
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.37,
    "sebi": 1,
    "stt": 124,
    "stamp": 100,
    "gst": 13.03,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 325.15
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.58,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.46
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.58,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 238.21
   },
   "headline": 325.15,
   "net": -3637.3
  },
  {
   "id": "CN04",
   "date": "2020-05-29",
   "why": "last day before SEBI 50% relief",
   "ref_price": 156.4,
   "quantity": 3197,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 3197,
       "price": 156.4
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 3197,
       "price": 159.85
      }
     ]
    }
   ],
   "buy_value": 500010.8,
   "sell_value": 511040.45,
   "turnover": 1011051.25,
   "vwap": 158.13,
   "gross": 11029.65,
   "dated_precise": {
    "brk": 40,
    "ex": 32.86,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 100,
    "gst": 13.11,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 312.98
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.86,
    "sebi": 1.01,
    "stt": 128,
    "stamp": 100,
    "gst": 13.12,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 329.74
   },
   "today_precise": {
    "brk": 40,
    "ex": 31.04,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.97,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 226.02
   },
   "today_worst": {
    "brk": 40,
    "ex": 31.04,
    "sebi": 1.01,
    "stt": 128,
    "stamp": 15,
    "gst": 12.97,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 242.77
   },
   "headline": 329.74,
   "net": 10699.91
  },
  {
   "id": "CN05",
   "date": "2020-06-01",
   "why": "first day SEBI Rs 5/cr (50%), short trade",
   "ref_price": 4020,
   "quantity": 124,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 124,
       "price": 4020
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 124,
       "price": 3990.55
      }
     ]
    }
   ],
   "buy_value": 494828.2,
   "sell_value": 498480,
   "turnover": 993308.2,
   "vwap": 4005.28,
   "gross": 3651.8,
   "dated_precise": {
    "brk": 40,
    "ex": 32.28,
    "sebi": 0.5,
    "stt": 124,
    "stamp": 99.33,
    "gst": 13.01,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 309.12
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.28,
    "sebi": 0.5,
    "stt": 125,
    "stamp": 99.33,
    "gst": 13.02,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 324.88
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.49,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 12.87,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.35
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.49,
    "sebi": 0.99,
    "stt": 125,
    "stamp": 15,
    "gst": 12.87,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239.1
   },
   "headline": 324.88,
   "net": 3326.92
  },
  {
   "id": "CN06",
   "date": "2020-06-30",
   "why": "last day of state stamp duty (losing long)",
   "ref_price": 600,
   "quantity": 833,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 833,
       "price": 600
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 833,
       "price": 594.05
      }
     ]
    }
   ],
   "buy_value": 499800,
   "sell_value": 494843.65,
   "turnover": 994643.65,
   "vwap": 597.03,
   "gross": -4956.35,
   "dated_precise": {
    "brk": 40,
    "ex": 32.33,
    "sebi": 0.5,
    "stt": 124,
    "stamp": 99.46,
    "gst": 13.02,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 309.31
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.33,
    "sebi": 0.5,
    "stt": 124,
    "stamp": 99.46,
    "gst": 13.02,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 324.06
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.54,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.41
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.54,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 238.16
   },
   "headline": 324.06,
   "net": -5280.41
  },
  {
   "id": "CN07",
   "date": "2020-07-01",
   "why": "first day unified stamp 0.003% buy",
   "ref_price": 600,
   "quantity": 833,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 833,
       "price": 600
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 833,
       "price": 606
      }
     ]
    }
   ],
   "buy_value": 499800,
   "sell_value": 504798,
   "turnover": 1004598,
   "vwap": 603,
   "gross": 4998,
   "dated_precise": {
    "brk": 40,
    "ex": 32.65,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.08,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 227.23
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.65,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.08,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 241.98
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.84,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.77
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.84,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 12.94,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 240.53
   },
   "headline": 241.98,
   "net": 4756.02
  },
  {
   "id": "CN08",
   "date": "2020-12-31",
   "why": "last day of NSE Rs 325/cr top slab",
   "ref_price": 1000,
   "quantity": 500,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 500,
       "price": 1000
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 500,
       "price": 1015
      }
     ]
    }
   ],
   "buy_value": 500000,
   "sell_value": 507500,
   "turnover": 1007500,
   "vwap": 1007.5,
   "gross": 7500,
   "dated_precise": {
    "brk": 40,
    "ex": 32.74,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.09,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 227.33
   },
   "dated_worst": {
    "brk": 40,
    "ex": 32.74,
    "sebi": 0.5,
    "stt": 127,
    "stamp": 15,
    "gst": 13.1,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 243.09
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.93,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.95,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.89
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.93,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 12.95,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 241.64
   },
   "headline": 243.09,
   "net": 7256.91
  },
  {
   "id": "CN09",
   "date": "2021-01-01",
   "why": "first day NSE Rs 345/cr top slab (losing short)",
   "ref_price": 1000,
   "quantity": 500,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 500,
       "price": 1000
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 500,
       "price": 1012
      }
     ]
    }
   ],
   "buy_value": 506000,
   "sell_value": 500000,
   "turnover": 1006000,
   "vwap": 1006,
   "gross": -6000,
   "dated_precise": {
    "brk": 40,
    "ex": 34.71,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.45,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 229.66
   },
   "dated_worst": {
    "brk": 40,
    "ex": 34.71,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.45,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 244.41
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.88,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.94,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.83
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.88,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.94,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 240.58
   },
   "headline": 244.41,
   "net": -6244.41
  },
  {
   "id": "CN10",
   "date": "2021-03-31",
   "why": "last day SEBI Rs 5/cr",
   "ref_price": 499.95,
   "quantity": 1000,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 1000,
       "price": 499.95
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 1000,
       "price": 505
      }
     ]
    }
   ],
   "buy_value": 499950,
   "sell_value": 505000,
   "turnover": 1004950,
   "vwap": 502.48,
   "gross": 5050,
   "dated_precise": {
    "brk": 40,
    "ex": 34.67,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.44,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 229.61
   },
   "dated_worst": {
    "brk": 40,
    "ex": 34.67,
    "sebi": 0.5,
    "stt": 126,
    "stamp": 15,
    "gst": 13.44,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 244.36
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.85,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.78
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.85,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.94,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 240.55
   },
   "headline": 244.36,
   "net": 4805.64
  },
  {
   "id": "CN11",
   "date": "2021-04-01",
   "why": "first day SEBI back to Rs 10/cr (same trade as CN10)",
   "ref_price": 499.95,
   "quantity": 1000,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 1000,
       "price": 499.95
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 1000,
       "price": 505
      }
     ]
    }
   ],
   "buy_value": 499950,
   "sell_value": 505000,
   "turnover": 1004950,
   "vwap": 502.48,
   "gross": 5050,
   "dated_precise": {
    "brk": 40,
    "ex": 34.67,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 13.44,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 230.11
   },
   "dated_worst": {
    "brk": 40,
    "ex": 34.67,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 13.44,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 244.87
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.85,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.78
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.85,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.94,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 240.55
   },
   "headline": 244.87,
   "net": 4805.13
  },
  {
   "id": "CN12",
   "date": "2022-07-15",
   "why": "last trading day without GST on the SEBI fee",
   "ref_price": 2500,
   "quantity": 200,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 200,
       "price": 2500
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 200,
       "price": 2537.5
      }
     ]
    }
   ],
   "buy_value": 500000,
   "sell_value": 507500,
   "turnover": 1007500,
   "vwap": 2518.75,
   "gross": 7500,
   "dated_precise": {
    "brk": 40,
    "ex": 34.76,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 13.46,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 230.23
   },
   "dated_worst": {
    "brk": 40,
    "ex": 34.76,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 13.46,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 245.98
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.93,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.95,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.89
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.93,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 12.95,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 241.64
   },
   "headline": 245.98,
   "net": 7254.02
  },
  {
   "id": "CN13",
   "date": "2022-07-18",
   "why": "first day GST 18% on the SEBI fee (same trade as CN12)",
   "ref_price": 2500,
   "quantity": 200,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 200,
       "price": 2500
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 200,
       "price": 2537.5
      }
     ]
    }
   ],
   "buy_value": 500000,
   "sell_value": 507500,
   "turnover": 1007500,
   "vwap": 2518.75,
   "gross": 7500,
   "dated_precise": {
    "brk": 40,
    "ex": 34.76,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 13.64,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 230.41
   },
   "dated_worst": {
    "brk": 40,
    "ex": 34.76,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 13.64,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 246.16
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.93,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.95,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 225.89
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.93,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 12.95,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 241.64
   },
   "headline": 246.16,
   "net": 7253.84
  },
  {
   "id": "CN14",
   "date": "2023-03-31",
   "why": "last day Rs 345 + IPFT 0.01 (short, low price)",
   "ref_price": 77.85,
   "quantity": 6423,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 6423,
       "price": 77.85
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 6423,
       "price": 76.3
      }
     ]
    }
   ],
   "buy_value": 490074.9,
   "sell_value": 500030.55,
   "turnover": 990105.45,
   "vwap": 77.08,
   "gross": 9955.65,
   "dated_precise": {
    "brk": 40,
    "ex": 34.16,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 13.53,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 227.68
   },
   "dated_worst": {
    "brk": 40,
    "ex": 34.16,
    "sebi": 0.99,
    "stt": 125,
    "stamp": 15,
    "gst": 13.53,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 243.43
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.4,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 12.85,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.24
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.4,
    "sebi": 0.99,
    "stt": 125,
    "stamp": 15,
    "gst": 12.86,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239
   },
   "headline": 243.43,
   "net": 9712.22
  },
  {
   "id": "CN15",
   "date": "2023-04-03",
   "why": "first trading day Rs 325 + IPFT 10 (same trade as CN14)",
   "ref_price": 77.85,
   "quantity": 6423,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 6423,
       "price": 77.85
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 6423,
       "price": 76.3
      }
     ]
    }
   ],
   "buy_value": 490074.9,
   "sell_value": 500030.55,
   "turnover": 990105.45,
   "vwap": 77.08,
   "gross": 9955.65,
   "dated_precise": {
    "brk": 40,
    "ex": 33.17,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 13.35,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 226.51
   },
   "dated_worst": {
    "brk": 40,
    "ex": 33.17,
    "sebi": 0.99,
    "stt": 125,
    "stamp": 15,
    "gst": 13.35,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 242.26
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.4,
    "sebi": 0.99,
    "stt": 124,
    "stamp": 15,
    "gst": 12.85,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.24
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.4,
    "sebi": 0.99,
    "stt": 125,
    "stamp": 15,
    "gst": 12.86,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239
   },
   "headline": 242.26,
   "net": 9713.39
  },
  {
   "id": "CN16",
   "date": "2024-03-28",
   "why": "last trading day Rs 325 + 10",
   "ref_price": 1601,
   "quantity": 312,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 312,
       "price": 1601
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 312,
       "price": 1640
      }
     ]
    }
   ],
   "buy_value": 499512,
   "sell_value": 511680,
   "turnover": 1011192,
   "vwap": 1620.5,
   "gross": 12168,
   "dated_precise": {
    "brk": 40,
    "ex": 33.87,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 13.48,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 229.36
   },
   "dated_worst": {
    "brk": 40,
    "ex": 33.87,
    "sebi": 1.01,
    "stt": 128,
    "stamp": 15,
    "gst": 13.48,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 246.11
   },
   "today_precise": {
    "brk": 40,
    "ex": 31.04,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.97,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 226.02
   },
   "today_worst": {
    "brk": 40,
    "ex": 31.05,
    "sebi": 1.01,
    "stt": 128,
    "stamp": 15,
    "gst": 12.98,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 242.79
   },
   "headline": 246.11,
   "net": 11921.89
  },
  {
   "id": "CN17",
   "date": "2024-04-01",
   "why": "first day Rs 322 + 10 (same trade as CN16)",
   "ref_price": 1601,
   "quantity": 312,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 312,
       "price": 1601
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 312,
       "price": 1640
      }
     ]
    }
   ],
   "buy_value": 499512,
   "sell_value": 511680,
   "turnover": 1011192,
   "vwap": 1620.5,
   "gross": 12168,
   "dated_precise": {
    "brk": 40,
    "ex": 33.57,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 13.42,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 229
   },
   "dated_worst": {
    "brk": 40,
    "ex": 33.57,
    "sebi": 1.01,
    "stt": 128,
    "stamp": 15,
    "gst": 13.42,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 245.75
   },
   "today_precise": {
    "brk": 40,
    "ex": 31.04,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 12.97,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 226.02
   },
   "today_worst": {
    "brk": 40,
    "ex": 31.05,
    "sebi": 1.01,
    "stt": 128,
    "stamp": 15,
    "gst": 12.98,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 242.79
   },
   "headline": 245.75,
   "net": 11922.25
  },
  {
   "id": "CN18",
   "date": "2024-09-30",
   "why": "last day of slab ladders (losing long, high price, 51 shares)",
   "ref_price": 9875,
   "quantity": 51,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 51,
       "price": 9875
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 51,
       "price": 9800
      }
     ]
    }
   ],
   "buy_value": 503625,
   "sell_value": 499800,
   "turnover": 1003425,
   "vwap": 9837.5,
   "gross": -3825,
   "dated_precise": {
    "brk": 40,
    "ex": 33.31,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 13.38,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 227.69
   },
   "dated_worst": {
    "brk": 40,
    "ex": 33.31,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 13.38,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 242.44
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.81,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 224.74
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.81,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.93,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239.49
   },
   "headline": 242.44,
   "net": -4067.44
  },
  {
   "id": "CN19",
   "date": "2024-10-01",
   "why": "first day flat Rs 297 + 10 (same trade as CN18)",
   "ref_price": 9875,
   "quantity": 51,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 51,
       "price": 9875
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 51,
       "price": 9800
      }
     ]
    }
   ],
   "buy_value": 503625,
   "sell_value": 499800,
   "turnover": 1003425,
   "vwap": 9837.5,
   "gross": -3825,
   "dated_precise": {
    "brk": 40,
    "ex": 30.81,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 224.74
   },
   "dated_worst": {
    "brk": 40,
    "ex": 30.81,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.93,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239.49
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.81,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.93,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 224.74
   },
   "today_worst": {
    "brk": 40,
    "ex": 30.81,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 12.93,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 239.49
   },
   "headline": 239.49,
   "net": -4064.49
  },
  {
   "id": "CN20",
   "date": "2026-02-27",
   "why": "last day Rs 297 + 10; short entry filled in 3 seconds, exit in 2",
   "ref_price": 5000,
   "quantity": 100,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 40,
       "price": 5000
      },
      {
       "qty": 35,
       "price": 4999.5
      },
      {
       "qty": 25,
       "price": 4999
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 60,
       "price": 4950
      },
      {
       "qty": 40,
       "price": 4951.5
      }
     ]
    }
   ],
   "buy_value": 495060,
   "sell_value": 499957.5,
   "turnover": 995017.5,
   "vwap": 4975.09,
   "gross": 4897.5,
   "dated_precise": {
    "brk": 40,
    "ex": 30.55,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.43
   },
   "dated_worst": {
    "brk": 100,
    "ex": 30.55,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 23.68,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 309.98
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.55,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.43
   },
   "today_worst": {
    "brk": 100,
    "ex": 30.55,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 23.68,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 309.98
   },
   "headline": 309.98,
   "net": 4587.52
  },
  {
   "id": "CN21",
   "date": "2026-03-02",
   "why": "first trading day Rs 306.99 + 0.01 (same trade as CN20)",
   "ref_price": 5000,
   "quantity": 100,
   "orders": [
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 40,
       "price": 5000
      },
      {
       "qty": 35,
       "price": 4999.5
      },
      {
       "qty": 25,
       "price": 4999
      }
     ]
    },
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 60,
       "price": 4950
      },
      {
       "qty": 40,
       "price": 4951.5
      }
     ]
    }
   ],
   "buy_value": 495060,
   "sell_value": 499957.5,
   "turnover": 995017.5,
   "vwap": 4975.09,
   "gross": 4897.5,
   "dated_precise": {
    "brk": 40,
    "ex": 30.55,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.43
   },
   "dated_worst": {
    "brk": 100,
    "ex": 30.55,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 23.68,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 309.98
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.55,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.43
   },
   "today_worst": {
    "brk": 100,
    "ex": 30.55,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 23.68,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 309.98
   },
   "headline": 309.98,
   "net": 4587.52
  },
  {
   "id": "CN22",
   "date": "2026-10-30",
   "why": "QUANTITY TIE at Rs 5 lakh: 5,00,000 / 1,600 = 312.5; reading 313 (worse for charges); entry in 8 seconds, exit partly by own order and the rest by Dhan auto square-off after 15:10",
   "ref_price": 1600,
   "quantity": 313,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 40,
       "price": 1600
      },
      {
       "qty": 40,
       "price": 1600.05
      },
      {
       "qty": 40,
       "price": 1600.1
      },
      {
       "qty": 40,
       "price": 1600.1
      },
      {
       "qty": 40,
       "price": 1600.2
      },
      {
       "qty": 40,
       "price": 1600.25
      },
      {
       "qty": 40,
       "price": 1600.3
      },
      {
       "qty": 33,
       "price": 1600.4
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 120,
       "price": 1617
      },
      {
       "qty": 80,
       "price": 1616.5
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": true,
     "fills": [
      {
       "qty": 113,
       "price": 1611.05
      }
     ]
    }
   ],
   "buy_value": 500853.2,
   "sell_value": 505408.65,
   "turnover": 1006261.85,
   "vwap": 1607.45,
   "gross": 4555.45,
   "dated_precise": {
    "brk": 60,
    "ex": 30.89,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 16.54,
    "dp": 0,
    "dpGst": 0,
    "sq": 20,
    "sqGst": 3.6,
    "total": 273.04
   },
   "dated_worst": {
    "brk": 210.24,
    "ex": 30.9,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 43.59,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 20,
    "sqGst": 3.6,
    "total": 466.09
   },
   "today_precise": {
    "brk": 60,
    "ex": 30.89,
    "sebi": 1.01,
    "stt": 126,
    "stamp": 15,
    "gst": 16.54,
    "dp": 0,
    "dpGst": 0,
    "sq": 20,
    "sqGst": 3.6,
    "total": 273.04
   },
   "today_worst": {
    "brk": 210.24,
    "ex": 30.9,
    "sebi": 1.01,
    "stt": 127,
    "stamp": 15,
    "gst": 43.59,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 20,
    "sqGst": 3.6,
    "total": 466.09
   },
   "headline": 466.09,
   "net": 4089.36
  },
  {
   "id": "CN23",
   "date": "2026-10-30",
   "why": "QUANTITY TIE, the other side: 312 shares, same fills as CN22 less one share on the last entry fill and the square-off",
   "ref_price": 1600,
   "quantity": 312,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 40,
       "price": 1600
      },
      {
       "qty": 40,
       "price": 1600.05
      },
      {
       "qty": 40,
       "price": 1600.1
      },
      {
       "qty": 40,
       "price": 1600.1
      },
      {
       "qty": 40,
       "price": 1600.2
      },
      {
       "qty": 40,
       "price": 1600.25
      },
      {
       "qty": 40,
       "price": 1600.3
      },
      {
       "qty": 32,
       "price": 1600.4
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 120,
       "price": 1617
      },
      {
       "qty": 80,
       "price": 1616.5
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": true,
     "fills": [
      {
       "qty": 112,
       "price": 1611.05
      }
     ]
    }
   ],
   "buy_value": 499252.8,
   "sell_value": 503797.6,
   "turnover": 1003050.4,
   "vwap": 1607.45,
   "gross": 4544.8,
   "dated_precise": {
    "brk": 60,
    "ex": 30.79,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 16.52,
    "dp": 0,
    "dpGst": 0,
    "sq": 20,
    "sqGst": 3.6,
    "total": 271.91
   },
   "dated_worst": {
    "brk": 209.76,
    "ex": 30.8,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 43.48,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 20,
    "sqGst": 3.6,
    "total": 464.39
   },
   "today_precise": {
    "brk": 60,
    "ex": 30.79,
    "sebi": 1,
    "stt": 125,
    "stamp": 15,
    "gst": 16.52,
    "dp": 0,
    "dpGst": 0,
    "sq": 20,
    "sqGst": 3.6,
    "total": 271.91
   },
   "today_worst": {
    "brk": 209.76,
    "ex": 30.8,
    "sebi": 1,
    "stt": 126,
    "stamp": 15,
    "gst": 43.48,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 20,
    "sqGst": 3.6,
    "total": 464.39
   },
   "headline": 464.39,
   "net": 4080.41
  },
  {
   "id": "CN24",
   "date": "2026-10-30",
   "why": "fills under Rs 66,666.67 each: entry in 10 seconds of ~Rs 50,000 (brokerage per fill hits the 0.03% leg); one exit fill",
   "ref_price": 160,
   "quantity": 3125,
   "orders": [
    {
     "side": "buy",
     "by_broker": false,
     "fills": [
      {
       "qty": 313,
       "price": 160
      },
      {
       "qty": 313,
       "price": 160
      },
      {
       "qty": 313,
       "price": 160.05
      },
      {
       "qty": 312,
       "price": 160.05
      },
      {
       "qty": 312,
       "price": 160.1
      },
      {
       "qty": 312,
       "price": 160.1
      },
      {
       "qty": 312,
       "price": 160.15
      },
      {
       "qty": 312,
       "price": 160.15
      },
      {
       "qty": 313,
       "price": 160.2
      },
      {
       "qty": 313,
       "price": 160.2
      }
     ]
    },
    {
     "side": "sell",
     "by_broker": false,
     "fills": [
      {
       "qty": 3125,
       "price": 158.4
      }
     ]
    }
   ],
   "buy_value": 500312.45,
   "sell_value": 495000,
   "turnover": 995312.45,
   "vwap": 159.25,
   "gross": -5312.45,
   "dated_precise": {
    "brk": 40,
    "ex": 30.56,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.44
   },
   "dated_worst": {
    "brk": 170.09,
    "ex": 30.56,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 36.3,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 391.7
   },
   "today_precise": {
    "brk": 40,
    "ex": 30.56,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 12.88,
    "dp": 0,
    "dpGst": 0,
    "sq": 0,
    "sqGst": 0,
    "total": 223.44
   },
   "today_worst": {
    "brk": 170.09,
    "ex": 30.56,
    "sebi": 1,
    "stt": 124,
    "stamp": 15,
    "gst": 36.3,
    "dp": 12.5,
    "dpGst": 2.25,
    "sq": 0,
    "sqGst": 0,
    "total": 391.7
   },
   "headline": 391.7,
   "net": -5704.15
  }
 ]
}

```
