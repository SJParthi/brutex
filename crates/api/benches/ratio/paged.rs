//! V0 (to be replaced): today's whole-month read, one `/bars.json` ask.

/// The most rows one `/bars.json` answer serves.
pub(crate) const MONTH_ANSWER_CAP_ROWS: usize = 60_000;

/// One answer to one page.
pub(crate) struct Page {
    pub(crate) status: u16,
    pub(crate) body: String,
}

/// What reading the month took.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MonthRead {
    pub(crate) rows: usize,
    pub(crate) pages: usize,
    pub(crate) first_t: Option<i64>,
    pub(crate) last_t: Option<i64>,
}

/// Today's read: the whole month in one `/bars.json` answer.
pub(crate) fn read_month(query: &str, mut ask: impl FnMut(&str) -> Page) -> Result<MonthRead, String> {
    let month = query
        .split('&')
        .filter(|kv| !kv.starts_with("to="))
        .map(|kv| kv.replacen("from=", "month=", 1))
        .collect::<Vec<_>>()
        .join("&");
    let page = ask(&format!("/bars.json?{month}"));
    if page.status != 200 {
        return Err(format!("answered {}: {}", page.status, page.body));
    }
    let stamps: Vec<i64> = page
        .body
        .match_indices("{\"t\":")
        .filter_map(|(at, _)| {
            let rest = page.body.get(at + 5..)?;
            let end = rest.find(',')?;
            rest.get(..end)?.parse().ok()
        })
        .collect();
    Ok(MonthRead {
        rows: stamps.len(),
        pages: 1,
        first_t: stamps.first().copied(),
        last_t: stamps.last().copied(),
    })
}

/// The words `/bars.json`'s refusal names its cap in.
pub(crate) fn cap_sentence(cap: usize) -> String {
    format!("past the {cap} one /bars.json answer serves")
}
