//! A local inspection surface over the application's real read handlers.
//! This utility does not activate a release, install telemetry, acquire the
//! writer lock, start recovery, create a broker, or run a sweep. Build directly
//! against named, hashed local libraries; their source is explicitly unverified.
//! Usage: main-inspector MASTERS STORE WEB LOOPBACK_IP PORT [SAVED_RESULTS_URL]
//! BRUTEX_STORE and BRUTEX_MASTERS must match the explicit roots; set
//! BRUTEX_AUTOPILOT=pause and BRUTEX_ARCHIVE_SUGGESTIONS=0 before launch.
#![forbid(unsafe_code)]

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs::{self, File},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
};

type Result<T> = std::result::Result<T, String>;
const MODE: &str = "read-only-main-app";
const MAX_URI_BYTES: usize = 8192;
const MAX_PATH_BYTES: usize = 1024;
const MAX_READS: usize = 4;
const MAX_STATIC_FILES: usize = 4096;
const MAX_STATIC_ENTRIES: usize = 8192;
const MAX_STATIC_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 12;
const API_HASH: &str = env!("BRUTEX_INSPECTOR_API_SHA256");
const SOURCE_HASH: &str = env!("BRUTEX_INSPECTOR_SOURCE_SHA256");

// These are reads of local masters, manifests, bars, immutable results, or
// bounded log tails. In-memory caches are permitted; disk cache builders and
// network-capable pages are excluded. No catch-all JSON suffix is admitted.
const READ_PATHS: &[&str] = &[
    "/inspection.json",
    "/health",
    "/feeds.json",
    "/instruments.json",
    "/universes.json",
    "/store.json",
    "/bars.json",
    "/bars/window.json",
    "/calendar.json",
    "/vocab.json",
    "/indexmap.json",
    "/backtest.json",
    "/backtest/run.json",
    "/trades.json",
    "/frontier.json",
    "/sweep-evidence.json",
    "/candidate-trades.json",
    "/expression-search.json",
    "/live.json",
    "/logs.json",
    "/audit.json",
];
const PAGE_PATHS: &[&str] = &["/", "/db", "/backtest", "/mapping", "/terminal", "/logs"];

fn err(value: impl std::fmt::Display) -> String {
    value.to_string()
}

fn directory(value: &OsStr) -> Result<PathBuf> {
    let path = Path::new(value);
    if !path.is_absolute() || !path.is_dir() {
        return Err("roots must be existing absolute directories".into());
    }
    fs::canonicalize(path).map_err(err)
}

#[derive(Clone, Debug)]
struct Config {
    masters: PathBuf,
    store: PathBuf,
    web: PathBuf,
    address: SocketAddr,
    saved_results_url: Option<String>,
}

impl Config {
    fn parse(args: &[OsString]) -> Result<Self> {
        let (roots, saved_results_url) =
            match args {
                [masters, store, web, ip, port] => ([masters, store, web, ip, port], None),
                [masters, store, web, ip, port, saved] => {
                    ([masters, store, web, ip, port], Some(saved_link(saved)?))
                }
                _ => return Err(
                    "usage: main-inspector MASTERS STORE WEB LOOPBACK_IP PORT [SAVED_RESULTS_URL]"
                        .into(),
                ),
            };
        let [masters, store, web, ip, port] = roots;
        let ip: IpAddr = ip.to_str().ok_or("IP is not UTF-8")?.parse().map_err(err)?;
        if !ip.is_loopback() {
            return Err("inspection binds only an explicit loopback IP".into());
        }
        let port = port.to_str().ok_or("port is not UTF-8")?;
        if port.is_empty() || port.starts_with('0') || !port.bytes().all(|b| b.is_ascii_digit()) {
            return Err("port must be a canonical positive decimal".into());
        }
        let port: u16 = port.parse().map_err(err)?;
        let masters = directory(masters)?;
        let store = directory(store)?;
        let web = directory(web)?;
        for (first, second) in [(&masters, &store), (&masters, &web), (&store, &web)] {
            if first.starts_with(second) || second.starts_with(first) {
                return Err(
                    "masters, store and web must be separate non-nested directories".into(),
                );
            }
        }
        if !web.join("build").is_dir() {
            return Err("the explicit web root has no built frontend".into());
        }
        Ok(Self {
            masters,
            store,
            web,
            address: SocketAddr::new(ip, port),
            saved_results_url,
        })
    }

    fn environment(&self, mut read: impl FnMut(&str) -> Option<OsString>) -> Result<()> {
        for (name, wanted) in [
            ("BRUTEX_STORE", &self.store),
            ("BRUTEX_MASTERS", &self.masters),
        ] {
            let actual = read(name)
                .ok_or_else(|| format!("{name} must explicitly match the launch root"))?;
            if directory(&actual)? != *wanted {
                return Err(format!("{name} differs from the explicit launch root"));
            }
        }
        for (name, wanted) in [
            ("BRUTEX_AUTOPILOT", "pause"),
            ("BRUTEX_ARCHIVE_SUGGESTIONS", "0"),
        ] {
            if read(name).as_deref() != Some(OsStr::new(wanted)) {
                return Err(format!(
                    "{name} must be {wanted} for this inspection process"
                ));
            }
        }
        Ok(())
    }
}

fn saved_link(value: &OsStr) -> Result<String> {
    let raw = value.to_str().ok_or("saved-results link is not UTF-8")?;
    if raw.len() > 4096 || raw.chars().any(char::is_control) || raw.contains(['#', '@', '\\', '%'])
    {
        return Err("saved-results link is not a literal local evidence URL".into());
    }
    let uri: axum::http::Uri = raw.parse().map_err(err)?;
    let authority = uri
        .authority()
        .ok_or("saved-results link has no authority")?;
    let host: IpAddr = authority
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .map_err(err)?;
    if uri.scheme_str() != Some("http")
        || !host.is_loopback()
        || authority.port_u16().is_none_or(|port| port == 0)
        || uri.path() != "/backtest"
    {
        return Err(
            "saved-results link must use an explicit loopback HTTP port and /backtest".into(),
        );
    }
    let mut values = std::collections::BTreeMap::new();
    for item in uri
        .query()
        .ok_or("saved-results link needs a pinned search")?
        .split('&')
    {
        let (key, value) = item
            .split_once('=')
            .ok_or("invalid saved-results selection")?;
        if !["identity", "pin", "batch", "rung", "offset", "setting"].contains(&key)
            || values.insert(key, value).is_some()
        {
            return Err("saved-results selection contains an unknown or repeated field".into());
        }
    }
    for key in ["identity", "pin"] {
        let value = values
            .get(key)
            .ok_or("saved-results selection is not pinned")?;
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("saved-results identity and pin must be lowercase 32-byte hex".into());
        }
    }
    let number = |key: &str| -> Result<u64> {
        let value = values
            .get(key)
            .ok_or("saved-results batch, rung and offset are required")?;
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("saved-results numeric fields must be canonical decimals".into());
        }
        value.parse().map_err(err)
    };
    let _batch = number("batch")?;
    if number("rung")? >= 8 {
        return Err("saved-results rung is outside the eight intraday rungs".into());
    }
    let offset = number("offset")?;
    if values.contains_key("setting") {
        let setting = number("setting")?;
        if setting < offset || setting - offset >= 32 {
            return Err("saved setting is outside its selected page".into());
        }
    }
    Ok(raw.to_owned())
}

fn static_path(path: &str) -> bool {
    path.starts_with("/_app/")
        || path.ends_with("/__data.json")
        || matches!(
            Path::new(path).extension().and_then(OsStr::to_str),
            Some(
                "html"
                    | "css"
                    | "js"
                    | "map"
                    | "svg"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "webp"
                    | "avif"
                    | "ico"
                    | "woff"
                    | "woff2"
                    | "ttf"
            )
        )
}

fn collect_assets(web: &Path) -> Result<BTreeSet<String>> {
    collect_assets_bounded(web, MAX_STATIC_ENTRIES)
}

fn collect_assets_bounded(web: &Path, entry_limit: usize) -> Result<BTreeSet<String>> {
    fn walk(
        dir: &Path,
        base: &Path,
        depth: usize,
        total: &mut u64,
        entries: &mut usize,
        paths: &mut BTreeSet<String>,
        entry_limit: usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err("frontend directory depth exceeds inspection limit".into());
        }
        for entry in fs::read_dir(dir).map_err(err)? {
            let entry = entry.map_err(err)?;
            *entries += 1;
            if *entries > entry_limit {
                return Err("frontend entry count exceeds inspection limit".into());
            }
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(err)?;
            if meta.file_type().is_symlink() {
                return Err("frontend inspection refuses symlinks".into());
            }
            if meta.is_dir() {
                walk(&path, base, depth + 1, total, entries, paths, entry_limit)?;
            } else if meta.is_file() {
                if meta.len() > MAX_ASSET_BYTES {
                    return Err("frontend asset exceeds inspection byte limit".into());
                }
                *total = total
                    .checked_add(meta.len())
                    .ok_or("frontend byte count overflow")?;
                if *total > MAX_STATIC_BYTES || paths.len() >= MAX_STATIC_FILES {
                    return Err("frontend inventory exceeds inspection limit".into());
                }
                let path = path
                    .strip_prefix(base)
                    .map_err(err)?
                    .to_str()
                    .ok_or("frontend path is not UTF-8")?;
                if path.contains(['%', '\\', '?', '#']) || path.chars().any(char::is_control) {
                    return Err("frontend path cannot be admitted as a literal URL".into());
                }
                let path = format!("/{path}");
                // A frontend file must never grant access to a backend route
                // with the same name, such as gaps.json or /pull/recovery.
                if static_path(&path) {
                    paths.insert(path);
                }
            } else {
                return Err("frontend inventory contains a non-regular file".into());
            }
        }
        Ok(())
    }
    let root = web.join("build");
    if fs::symlink_metadata(&root)
        .map_err(err)?
        .file_type()
        .is_symlink()
    {
        return Err("frontend build root must not be a symlink".into());
    }
    let mut paths = BTreeSet::new();
    walk(&root, &root, 0, &mut 0, &mut 0, &mut paths, entry_limit)?;
    if !paths.contains("/index.html") {
        return Err("frontend index.html is absent".into());
    }
    Ok(paths)
}

#[derive(Clone)]
struct Guard {
    authority: String,
    paths: Arc<BTreeSet<String>>,
    reads: Arc<tokio::sync::Semaphore>,
}

fn guard(config: &Config, assets: BTreeSet<String>) -> Guard {
    let paths = READ_PATHS
        .iter()
        .chain(PAGE_PATHS)
        .map(|p| (*p).to_owned())
        .chain(assets)
        .collect();
    Guard {
        authority: config.address.to_string(),
        paths: Arc::new(paths),
        reads: Arc::new(tokio::sync::Semaphore::new(MAX_READS)),
    }
}

fn refusal(code: StatusCode, reason: &str) -> Response {
    let mut response = (code, axum::Json(serde_json::json!({"mode":MODE,"error":reason,"can_sweep":false,"can_pull":false,"can_write":false}))).into_response();
    if code == StatusCode::METHOD_NOT_ALLOWED {
        response
            .headers_mut()
            .insert(header::ALLOW, "GET, HEAD".parse().expect("static header"));
    }
    response
}

fn admit(request: &Request, guard: &Guard) -> std::result::Result<(), (StatusCode, &'static str)> {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return Err((
            StatusCode::METHOD_NOT_ALLOWED,
            "inspection permits only GET and HEAD; execution is disabled",
        ));
    }
    let path = request.uri().path();
    if request.uri().to_string().len() > MAX_URI_BYTES || path.len() > MAX_PATH_BYTES {
        return Err((
            StatusCode::URI_TOO_LONG,
            "request exceeds the inspection URL limit",
        ));
    }
    if request.uri().scheme().is_some()
        || request.uri().authority().is_some()
        || path.contains(['%', '\\'])
        || path.split('/').any(|part| matches!(part, "." | ".."))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "inspection requires a literal local path",
        ));
    }
    let headers = request.headers();
    if headers.get_all(header::HOST).iter().count() != 1
        || headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(guard.authority.as_str())
    {
        return Err((
            StatusCode::FORBIDDEN,
            "Host must match this exact loopback listener",
        ));
    }
    for name in [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-host",
        "x-forwarded-port",
        "x-forwarded-proto",
    ] {
        if headers.contains_key(name) {
            return Err((
                StatusCode::FORBIDDEN,
                "proxy authority headers are not accepted",
            ));
        }
    }
    if headers.get_all(header::ORIGIN).iter().count() > 1 {
        return Err((
            StatusCode::FORBIDDEN,
            "multiple Origin headers are not accepted",
        ));
    }
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.to_str().ok() != Some(format!("http://{}", guard.authority).as_str())
    {
        return Err((
            StatusCode::FORBIDDEN,
            "cross-origin inspection is not permitted",
        ));
    }
    if headers.get_all("sec-fetch-site").iter().count() > 1
        || headers
            .get("sec-fetch-site")
            .is_some_and(|v| match v.to_str() {
                Ok("same-origin" | "none") => false,
                Ok("same-site" | "cross-site") => !document_navigation(request),
                _ => true,
            })
    {
        return Err((
            StatusCode::FORBIDDEN,
            "cross-site inspection is not permitted",
        ));
    }
    if headers.contains_key(header::TRANSFER_ENCODING)
        || headers.get_all(header::CONTENT_LENGTH).iter().count() > 1
        || headers
            .get(header::CONTENT_LENGTH)
            .is_some_and(|v| v.as_bytes() != b"0")
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "inspection requests must not carry a body",
        ));
    }
    if !guard.paths.contains(path) {
        return Err((
            StatusCode::FORBIDDEN,
            "this route is not an admitted inspection read",
        ));
    }
    Ok(())
}

// A person following a link between the two local app ports performs a
// cross-origin document navigation. This exception permits only an explicit
// top-level user navigation to an admitted HTML page; it never admits JSON,
// background fetches, frames, state changes or an Origin-bearing request.
fn document_navigation(request: &Request) -> bool {
    request.method() == Method::GET
        && PAGE_PATHS.contains(&request.uri().path())
        && !request.headers().contains_key(header::ORIGIN)
        && [
            ("sec-fetch-mode", "navigate"),
            ("sec-fetch-dest", "document"),
            ("sec-fetch-user", "?1"),
        ]
        .into_iter()
        .all(|(name, wanted)| {
            request.headers().get_all(name).iter().count() == 1
                && request.headers().get(name).and_then(|v| v.to_str().ok()) == Some(wanted)
        })
}

async fn protect(State(guard): State<Guard>, request: Request, next: Next) -> Response {
    let head = request.method() == Method::HEAD;
    let response = match admit(&request, &guard) {
        Err((code, reason)) => refusal(code, reason),
        Ok(()) => {
            // Static assets do not consume scarce data-reader slots. Admission
            // remains exact-path for both classes.
            let data = READ_PATHS.contains(&request.uri().path());
            let permit = if data {
                Some(guard.reads.clone().try_acquire_owned())
            } else {
                None
            };
            if matches!(permit, Some(Err(_))) {
                let mut response = refusal(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "inspection readers are busy; retry this read",
                );
                response
                    .headers_mut()
                    .insert(header::RETRY_AFTER, "1".parse().expect("static header"));
                response
            } else {
                let _permit = permit;
                next.run(request).await
            }
        }
    };
    let mut response = response;
    response
        .headers_mut()
        .insert("x-brutex-mode", MODE.parse().expect("static header"));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("static header"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
    if head {
        *response.body_mut() = Body::empty();
    }
    response
}

fn metadata(config: &Config) -> serde_json::Value {
    serde_json::json!({
        "schema":1, "mode":MODE, "can_sweep":false, "can_pull":false, "can_write":false,
        "release_cleared":false, "automatic_recovery":false, "automatic_acquisition":false,
        "source_stamp":{"status":"unverified_cached_build","api_source_commit":null,"api_library_sha256":API_HASH,"wrapper_source_sha256":SOURCE_HASH},
        "store_root":config.store, "masters_root":config.masters,
        "saved_results_url":config.saved_results_url,
        "allowed_read_paths":READ_PATHS, "allowed_pages":PAGE_PATHS,
        "limits":{"concurrent_data_reads":MAX_READS,"uri_bytes":MAX_URI_BYTES,"static_files":MAX_STATIC_FILES,"static_bytes":MAX_STATIC_BYTES},
        "logging":"Reads existing CLI logs; this inspector installs no telemetry sink and starts no background work.",
        "notice":"Inspection of real stored data only. This process cannot start or approve a full sweep. Cached reader-library source provenance has not been verified."
    })
}

fn app(config: &Config) -> Result<Router> {
    let guarded = guard(config, collect_assets(&config.web)?);
    // This constructor leaves Broker::Refused. No serving constructor or API
    // startup entry point is called. Private read handlers remain the actual
    // application's handlers rather than copied implementations.
    let site = Arc::new(api::server::Site::load(&config.masters, &config.store));
    let assets = Arc::new(api::assets::Assets::new(&config.web));
    let inner = api::server::router_serving(site, assets, config.address);
    let info = metadata(config);
    Ok(inner
        .route(
            "/inspection.json",
            get(move || {
                let info = info.clone();
                async move { axum::Json(info) }
            }),
        )
        .layer(middleware::from_fn_with_state(guarded, protect)))
}

fn writer_absence(store: &Path) -> Result<Option<File>> {
    match File::open(store.join("serve.lock")) {
        Ok(file) => {
            file.try_lock_shared()
                .map_err(|why| format!("inspection cannot share the existing store lock: {why}"))?;
            Ok(Some(file))
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(why) => Err(format!("existing store lock is unreadable: {why}")),
    }
}

fn main() -> Result<()> {
    let config = Config::parse(&std::env::args_os().skip(1).collect::<Vec<_>>())?;
    config.environment(|name| std::env::var_os(name))?;
    let _read_lock = writer_absence(&config.store)?;
    let app = app(&config)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(2)
        .enable_all()
        .build()
        .map_err(err)?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(config.address).await.map_err(err)?;
        println!("brutex inspection listening on http://{}/; writes, recovery, acquisition and sweeps disabled; reader source provenance unverified", config.address);
        api::server::serve(listener, app, Box::pin(tokio::signal::ctrl_c())).await.map_err(err)
    })
}

#[cfg(test)]
#[path = "main-inspector-tests.rs"]
mod tests;
