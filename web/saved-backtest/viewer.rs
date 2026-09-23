//! Standalone read-only saved-results viewer. Direct rustc link to local API
//! libraries; never call API startup, Site, recovery, ingest or the full router.
//! Usage: viewer STORE WEB LOOPBACK_IP PORT OBSERVATION_BYTES REPLAY_NODES
//!        [RECORDED_VOCABULARY_JSON BLAKE3]
//! Launch environment must contain the same BRUTEX_STORE,
//! BRUTEX_BOOLEAN_OBSERVATION_BYTES and BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES.
//! No environment mutation, child process, credential discovery or telemetry sink.
#![forbid(unsafe_code)]

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io::Read,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
};

// Reuse the actual API configuration parsers, including addressability refusal.
const MAX_SCAN_BYTES: u64 = api::detail::MAX_SCAN_BYTES;
const MAX_CONCURRENT: usize = api::detail::MAX_CONCURRENT;
#[path = "../../crates/api/src/boolean_observation_budget.rs"]
mod boolean_observation_budget;
#[path = "../../crates/api/src/boolean_search_budget.rs"]
mod boolean_search_budget;

const MAX_STATIC_FILES: usize = 256;
const MAX_STATIC_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 8 * 1024 * 1024;
const MAX_STATIC_DEPTH: usize = 8;
const ROUTES: [&str; 8] = [
    "/boolean-qualified-search.json",
    "/boolean-qualified-campaign.json",
    "/boolean-campaign.json",
    "/boolean-qualification.json",
    "/boolean-admission.json",
    "/boolean-statistics.json",
    "/boolean-oos.json",
    "/boolean-candidates.json",
];
type Result<T> = std::result::Result<T, String>;
fn err(value: impl std::fmt::Display) -> String {
    value.to_string()
}

#[derive(Clone, Debug)]
struct Config {
    store: PathBuf,
    web: PathBuf,
    address: SocketAddr,
    bytes: u64,
    nodes: u64,
    recorded_vocabulary: Option<(PathBuf, String)>,
}
fn directory(value: &std::ffi::OsStr) -> Result<PathBuf> {
    let path = Path::new(value);
    if !path.is_absolute() || !path.is_dir() {
        return Err("store and web roots must be existing absolute directories".into());
    }
    fs::canonicalize(path).map_err(err)
}
fn positive(value: &std::ffi::OsStr) -> Result<u64> {
    let value = value.to_str().ok_or("numeric argument is not UTF-8")?;
    if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("numeric arguments must be canonical positive decimals".into());
    }
    value.parse().map_err(err)
}
impl Config {
    fn parse(args: &[OsString]) -> Result<Self> {
        let (base, recorded_vocabulary) = match args {
            [base @ .., path, pin] if args.len() == 8 => {
                let path = PathBuf::from(path);
                let pin = pin.to_str().ok_or("vocabulary pin is not UTF-8")?;
                if !path.is_absolute() || !hex_digest(pin, 64) {
                    return Err("recorded vocabulary needs an absolute file and lowercase BLAKE3 pin".into());
                }
                (base, Some((path, pin.to_owned())))
            }
            base if args.len() == 6 => (base, None),
            _ => return Err("usage: viewer STORE WEB LOOPBACK_IP PORT OBSERVATION_BYTES REPLAY_NODES [RECORDED_VOCABULARY_JSON BLAKE3]".into()),
        };
        let [store, web, ip, port, bytes, nodes] = base else {
            return Err(
                "usage: viewer STORE WEB LOOPBACK_IP PORT OBSERVATION_BYTES REPLAY_NODES".into(),
            );
        };
        let ip: IpAddr = ip.to_str().ok_or("IP is not UTF-8")?.parse().map_err(err)?;
        if !ip.is_loopback() {
            return Err("viewer binds only an explicit loopback IP".into());
        }
        let port = u16::try_from(positive(port)?).map_err(err)?;
        let observation =
            boolean_observation_budget::BooleanObservationBudget::from_value(Some(bytes))?;
        let replay = boolean_search_budget::ReplayBudget::parse(Some(nodes))?;
        let store = directory(store)?;
        let web = directory(web)?;
        if store == web || web.starts_with(&store) || store.starts_with(&web) {
            return Err("evidence and web roots must be separate non-nested directories".into());
        }
        Ok(Self {
            store,
            web,
            address: SocketAddr::new(ip, port),
            bytes: observation.bytes(),
            nodes: replay.nodes(),
            recorded_vocabulary,
        })
    }
    fn require_environment(&self) -> Result<()> {
        let store = std::env::var_os("BRUTEX_STORE")
            .ok_or("BRUTEX_STORE must explicitly match the launch root")?;
        std::env::var_os("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
            .ok_or("BRUTEX_BOOLEAN_OBSERVATION_BYTES must explicitly match the launch allowance")?;
        std::env::var_os("BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES").ok_or(
            "BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES must explicitly match the launch allowance",
        )?;
        let observation = boolean_observation_budget::BooleanObservationBudget::load()?;
        let replay = boolean_search_budget::ReplayBudget::load()?;
        if directory(&store)? != self.store
            || observation.bytes() != self.bytes
            || replay.nodes() != self.nodes
        {
            return Err(observation
                .context("handler environment differs from the explicit viewer launch contract"));
        }
        Ok(())
    }
}

#[derive(Clone)]
struct Asset {
    body: Bytes,
    mime: &'static str,
}
struct App {
    config: Config,
    assets: BTreeMap<String, Asset>,
    asset_bytes: u64,
    detail: Arc<tokio::sync::Semaphore>,
    vocabulary: Option<serde_json::Value>,
}
fn mime(path: &Path) -> Option<&'static str> {
    Some(match path.extension()?.to_str()? {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => return None,
    })
}
fn safe_path(path: &str) -> bool {
    path.starts_with('/')
        && path.len() <= 1024
        && path.is_ascii()
        && !path.contains(['%', '\\', '\0'])
        && path.split('/').skip(1).all(|part| {
            !matches!(part, "." | "..")
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}
fn read_asset(path: &Path) -> Result<Bytes> {
    read_bounded(path, MAX_ASSET_BYTES)
}
fn read_bounded(path: &Path, max_bytes: u64) -> Result<Bytes> {
    // As on the retained-evidence open path, prevent a final symlink/FIFO race.
    // Intermediate directories still require the trusted reviewed web root.
    let before = fs::symlink_metadata(path).map_err(err)?;
    if !before.is_file() || before.len() > max_bytes {
        return Err("static asset is not a regular bounded file".into());
    }
    let mut file = open_static(path)?;
    let held = file.metadata().map_err(err)?;
    if !held.is_file() || held.len() != before.len() {
        return Err("static asset changed during admission".into());
    }
    let mut body = Vec::new();
    body.try_reserve_exact(usize::try_from(held.len()).map_err(err)?)
        .map_err(err)?;
    file.by_ref()
        .take(max_bytes + 1)
        .read_to_end(&mut body)
        .map_err(err)?;
    let after = file.metadata().map_err(err)?;
    if body.len() as u64 != held.len()
        || after.len() != held.len()
        || after.modified().map_err(err)? != held.modified().map_err(err)?
    {
        return Err("static asset changed during read".into());
    }
    Ok(Bytes::from(body))
}
#[cfg(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
fn open_static(path: &Path) -> Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    // Same verified O_NOFOLLOW | O_NONBLOCK values as cli::readonly_file.
    #[cfg(target_os = "macos")]
    let flags = 0x100 | 0x4;
    #[cfg(target_os = "linux")]
    let flags = 0x20_000 | 0x800;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(err)?;
    if !file.metadata().map_err(err)?.is_file() {
        return Err("static handle is not regular".into());
    }
    Ok(file)
}
#[cfg(not(any(
    target_os = "macos",
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
fn open_static(_path: &Path) -> Result<File> {
    Err("static admission requires verified macOS or Linux file flags".into())
}
impl App {
    fn load(config: Config) -> Result<Arc<Self>> {
        let vocabulary = match &config.recorded_vocabulary {
            Some((path, pin)) => Some(recorded_vocabulary(path, pin)?),
            None => cli::commit_stamp().map(native_vocabulary).transpose()?,
        };
        let mut pending = vec![(config.web.clone(), 0usize)];
        let mut assets = BTreeMap::new();
        let mut entries = 0usize;
        let mut asset_bytes = 0u64;
        while let Some((directory, depth)) = pending.pop() {
            for entry in fs::read_dir(directory).map_err(err)? {
                entries += 1;
                if entries > MAX_STATIC_FILES {
                    return Err("static entry count exceeds 256".into());
                }
                let entry = entry.map_err(err)?;
                let path = entry.path();
                let ty = entry.file_type().map_err(err)?;
                if ty.is_dir() {
                    if depth >= MAX_STATIC_DEPTH {
                        return Err("static directory depth exceeds eight".into());
                    }
                    pending.push((path, depth + 1));
                    continue;
                }
                if !ty.is_file() {
                    return Err("static tree contains a symlink or special file".into());
                }
                let Some(mime) = mime(&path) else {
                    continue;
                };
                let name = format!(
                    "/{}",
                    path.strip_prefix(&config.web)
                        .map_err(err)?
                        .to_str()
                        .ok_or("non-UTF8 static path")?
                );
                if !safe_path(&name) {
                    return Err("unsupported static URL path".into());
                }
                let body = read_asset(&path)?;
                asset_bytes = asset_bytes
                    .checked_add(body.len() as u64)
                    .ok_or("static size overflow")?;
                if asset_bytes > MAX_STATIC_BYTES {
                    return Err("static snapshot exceeds 16MiB".into());
                }
                if assets.insert(name, Asset { body, mime }).is_some() {
                    return Err("duplicate static path".into());
                }
            }
        }
        if !assets.contains_key("/index.html") {
            return Err("web root must contain index.html".into());
        }
        Ok(Arc::new(Self {
            config,
            assets,
            asset_bytes,
            detail: Arc::new(tokio::sync::Semaphore::new(1)),
            vocabulary,
        }))
    }
}

fn refusal(status: StatusCode, why: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::json!({"status":"refused","refusal":why,"rows":[]}).to_string(),
    )
        .into_response()
}
async fn boundary(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return refusal(
            StatusCode::METHOD_NOT_ALLOWED,
            "saved viewer is GET/HEAD only; no control operations exist",
        );
    }
    let authority = app.config.address.to_string();
    if request.headers().get_all(header::HOST).iter().count() != 1
        || request
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
            != Some(authority.as_str())
    {
        return refusal(
            StatusCode::FORBIDDEN,
            "Host must match the explicit loopback listener",
        );
    }
    let origin = format!("http://{authority}");
    let page_navigation = request.method() == Method::GET
        && matches!(request.uri().path(), "/" | "/backtest" | "/index.html")
        && !request.headers().contains_key(header::ORIGIN)
        && [
            ("sec-fetch-mode", "navigate"),
            ("sec-fetch-dest", "document"),
            ("sec-fetch-user", "?1"),
        ]
        .iter()
        .all(|(key, expected)| {
            request.headers().get_all(*key).iter().count() == 1
                && request
                    .headers()
                    .get(*key)
                    .and_then(|value| value.to_str().ok())
                    == Some(*expected)
        });
    let repeated_origin = request.headers().get_all(header::ORIGIN).iter().count() > 1;
    let repeated_site = request.headers().get_all("sec-fetch-site").iter().count() > 1;
    if repeated_origin
        || repeated_site
        || request
            .headers()
            .get(header::ORIGIN)
            .is_some_and(|v| v.to_str().ok() != Some(origin.as_str()))
        || !page_navigation
            && request
                .headers()
                .get("sec-fetch-site")
                .is_some_and(|v| !matches!(v.to_str(), Ok("same-origin" | "none")))
    {
        return refusal(
            StatusCode::FORBIDDEN,
            "cross-origin saved evidence request refused",
        );
    }
    if request.headers().contains_key(header::TRANSFER_ENCODING)
        || request
            .headers()
            .get_all(header::CONTENT_LENGTH)
            .iter()
            .count()
            > 1
        || request
            .headers()
            .get(header::CONTENT_LENGTH)
            .is_some_and(|value| value.as_bytes() != b"0")
    {
        return refusal(
            StatusCode::BAD_REQUEST,
            "saved viewer reads do not accept request bodies",
        );
    }
    if request.uri().to_string().len() > 2048 {
        return refusal(StatusCode::URI_TOO_LONG, "viewer URI exceeds 2048 bytes");
    }
    let is_detail = ROUTES.contains(&request.uri().path());
    let _permit = if is_detail {
        match Arc::clone(&app.detail).try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                return refusal(
                    StatusCode::TOO_MANY_REQUESTS,
                    "one saved detail read is already active; nothing queued",
                );
            }
        }
    } else {
        None
    };
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        axum::http::HeaderValue::from_static("nosniff"),
    );
    response
}
async fn status(State(app): State<Arc<App>>) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], serde_json::json!({
        "schema_version":1,"mode":"read-only-saved-results","api_commit":cli::commit_stamp(),
        "reader_build_verification":"unverified_cached_build","api_source_commit":null,
        "reader_library_sha256":env!("BRUTEX_VIEWER_API_SHA256"),
        "wrapper_source_sha256":env!("BRUTEX_VIEWER_SOURCE_SHA256"),
        "commit_stamp_scope":"embedded label only; cached library bytes do not match the retained release receipt",
        "vocabulary_commit":app.vocabulary.as_ref().map(|v| &v["api_commit"]),
        "vocabulary_provenance":app.vocabulary.as_ref().map(|v| &v["provenance"]),
        "vocabulary_receipt_blake3":app.vocabulary.as_ref().and_then(|v| v.get("receipt_blake3")),
        "store":app.config.store,"web":app.config.web,"address":app.config.address.to_string(),
        "observation_bytes":app.config.bytes.to_string(),"replay_nodes":app.config.nodes.to_string(),
        "static_files":app.assets.len(),"static_bytes":app.asset_bytes.to_string(),
        "static_snapshot":"loaded once at startup; later disk edits require explicit viewer restart",
        "detail_jobs":1,"routes":ROUTES,"vocabulary_route":"/vocab.json",
        "pull_capability":false,"sweep_capability":false,
        "recovery_started":false,"live_service_replaced":false,"release_clearance":false
    }).to_string()).into_response()
}
fn hex_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn digest(bytes: &[u8]) -> String {
    brutex_core::blake3::hash(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct RecordedBit {
    i: u16,
    name: String,
    live: bool,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct RecordedVocabulary {
    api_commit: String,
    commit_digest: String,
    vocab_version: u32,
    count: usize,
    bits: Vec<RecordedBit>,
    label_contract: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt_blake3: Option<String>,
}
fn recorded_vocabulary(path: &Path, pin: &str) -> Result<serde_json::Value> {
    let bytes = read_bounded(path, 128 * 1024)?;
    if bytes.len() > 128 * 1024 || digest(&bytes) != pin {
        return Err("recorded vocabulary bytes differ from the explicit BLAKE3 pin".into());
    }
    let strict: RecordedVocabulary = serde_json::from_slice(&bytes).map_err(err)?;
    let recorded = serde_json::to_value(strict).map_err(err)?;
    let commit = recorded["api_commit"]
        .as_str()
        .ok_or("recorded vocabulary commit is absent")?;
    let mut native = native_vocabulary(commit)?;
    for key in [
        "api_commit",
        "commit_digest",
        "vocab_version",
        "count",
        "bits",
    ] {
        if native[key] != recorded[key] {
            return Err(format!(
                "recorded vocabulary {key} differs from the complete linked Rust table"
            ));
        }
    }
    native["provenance"] = "explicitly pinned saved vocabulary; all rows match the linked Rust table; this is not reader-source verification".into();
    native["receipt_blake3"] = pin.into();
    Ok(native)
}
fn vocabulary(app: &App, query: Option<&str>) -> Result<serde_json::Value> {
    let value = app.vocabulary.as_ref().ok_or(
        "vocabulary unavailable: reader is unstamped and no recorded vocabulary was supplied",
    )?;
    if let Some(query) = query {
        let mut expected_commit = None;
        let mut expected_version = None;
        for field in query.split('&') {
            let (key, value) = field
                .split_once('=')
                .ok_or("malformed vocabulary selector")?;
            let slot = match key {
                "commit" => &mut expected_commit,
                "vocab_version" => &mut expected_version,
                _ => return Err("unknown vocabulary selector".into()),
            };
            if slot.replace(value).is_some() {
                return Err("duplicate vocabulary selector".into());
            }
        }
        if expected_commit != value["commit_digest"].as_str()
            || expected_version != Some(vocab::VOCAB_VERSION.to_string().as_str())
        {
            return Err("vocabulary build/version differs from the saved grid".into());
        }
    }
    Ok(value.clone())
}
fn native_vocabulary(commit: &str) -> Result<serde_json::Value> {
    if !hex_digest(commit, 40) {
        return Err("vocabulary commit must be 40 lowercase hexadecimal characters".into());
    }
    let mut bits = Vec::with_capacity(vocab::table::COUNT);
    for index in 0..vocab::table::COUNT {
        let position = u16::try_from(index).map_err(err)?;
        let def = vocab::table::definition(position).ok_or("vocabulary table is incomplete")?;
        if def.index != position {
            return Err("vocabulary table position differs".into());
        }
        bits.push(serde_json::json!({"i":def.index,"name":def.name,"live":vocab::table::is_live(position)}));
    }
    Ok(
        serde_json::json!({"vocab_version":vocab::VOCAB_VERSION,"count":bits.len(),"bits":bits,
        "api_commit":commit,"commit_digest":digest(commit.as_bytes()),
        "provenance":"linked Rust vocabulary with embedded commit label",
        "label_contract":"compare commit_digest with both saved grids before decoding; unknown bits stay numeric"}),
    )
}
async fn vocab_json(State(app): State<Arc<App>>, request: Request) -> Response {
    match vocabulary(&app, request.uri().query()) {
        Ok(value) => (
            [(header::CONTENT_TYPE, "application/json")],
            value.to_string(),
        )
            .into_response(),
        Err(why) => refusal(StatusCode::SERVICE_UNAVAILABLE, &why),
    }
}
async fn static_file(State(app): State<Arc<App>>, request: Request) -> Response {
    let path = request.uri().path();
    if !safe_path(path) {
        return refusal(StatusCode::BAD_REQUEST, "invalid static path");
    }
    let key = match path {
        "/" | "/backtest" => "/index.html",
        other => other,
    };
    let Some(asset) = app.assets.get(key) else {
        return refusal(
            StatusCode::NOT_FOUND,
            "path is not a saved-viewer route or admitted asset",
        );
    };
    let body = if request.method() == Method::HEAD {
        Body::empty()
    } else {
        Body::from(asset.body.clone())
    };
    let mut response = ([(header::CONTENT_TYPE, asset.mime)], body).into_response();
    if let Ok(value) = axum::http::HeaderValue::from_str(&asset.body.len().to_string()) {
        response.headers_mut().insert(header::CONTENT_LENGTH, value);
    }
    response
}
fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/viewer.json", get(status))
        .route("/vocab.json", get(vocab_json))
        .route(
            "/boolean-qualified-search.json",
            get(api::booleansearchjson::search_json),
        )
        .route(
            "/boolean-qualified-campaign.json",
            get(api::booleancampaignjson::qualified_campaign_json),
        )
        .route(
            "/boolean-campaign.json",
            get(api::booleancampaignjson::campaign_json),
        )
        .route(
            "/boolean-qualification.json",
            get(api::booleanevidencejson::qualification_json),
        )
        .route(
            "/boolean-admission.json",
            get(api::booleanevidencejson::admission_json),
        )
        .route(
            "/boolean-statistics.json",
            get(api::booleanevidencejson::statistics_json),
        )
        .route("/boolean-oos.json", get(api::booleanoosjson::later_json))
        .route(
            "/boolean-candidates.json",
            get(api::booleanjson::boolean_json),
        )
        .fallback(static_file)
        .layer(middleware::from_fn_with_state(Arc::clone(&app), boundary))
        .with_state(app)
}
fn run() -> Result<()> {
    let config = Config::parse(&std::env::args_os().skip(1).collect::<Vec<_>>())?;
    config.require_environment()?;
    let app = App::load(config.clone())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(2)
        .enable_all()
        .build()
        .map_err(err)?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(config.address).await.map_err(err)?;
        println!("READ_ONLY_VIEWER http://{}/backtest store={} api_commit={:?}; no recovery, pull, sweep or release clearance", config.address, config.store.display(), cli::commit_stamp());
        axum::serve(listener, router(app)).with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await.map_err(err)
    })
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("VIEWER_REFUSED: {why}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt as _;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "brutex-saved-viewer-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).map_err(err)?;
            fs::create_dir(path.join("store")).map_err(err)?;
            fs::create_dir(path.join("web")).map_err(err)?;
            fs::write(path.join("web/index.html"), b"generated read-only shell").map_err(err)?;
            fs::write(path.join("web/app.js"), b"// generated static fixture").map_err(err)?;
            Ok(Self(path))
        }
        fn args(&self) -> Vec<OsString> {
            vec![
                self.0.join("store").into_os_string(),
                self.0.join("web").into_os_string(),
                "127.0.0.1".into(),
                "18081".into(),
                "6442450944".into(),
                "12000000".into(),
            ]
        }
        fn app(&self) -> Result<Arc<App>> {
            App::load(Config::parse(&self.args())?)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn runtime() -> Result<tokio::runtime::Runtime> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(err)
    }
    async fn call(
        app: Arc<App>,
        method: Method,
        path: &str,
        host: &str,
    ) -> Result<(StatusCode, String)> {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, host)
            .body(Body::empty())
            .map_err(err)?;
        let response = router(app).oneshot(request).await.map_err(err)?;
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .map_err(err)?;
        Ok((status, String::from_utf8(bytes.to_vec()).map_err(err)?))
    }
    #[test]
    fn launch_refuses_nonloopback_malformed_and_overlapping_roots() -> Result<()> {
        let f = Fixture::new()?;
        let good = f.args();
        assert!(Config::parse(&good).is_ok());
        for (index, value) in [
            (2, "0.0.0.0"),
            (2, "192.0.2.1"),
            (3, "0"),
            (3, "65536"),
            (3, "018081"),
            (4, "0"),
            (4, "18446744073709551615"),
            (5, "01"),
        ] {
            let mut args = good.clone();
            args[index] = value.into();
            assert!(Config::parse(&args).is_err(), "{index} {value}");
        }
        let mut args = good;
        args[1] = args[0].clone();
        assert!(Config::parse(&args).is_err());
        args[0] = "relative-store".into();
        assert!(Config::parse(&args).is_err());
        assert!(Config::parse(&[]).is_err());
        Ok(())
    }
    #[test]
    fn only_saved_get_routes_exist_and_routing_creates_no_store_history() -> Result<()> {
        let f = Fixture::new()?;
        let app = f.app()?;
        runtime()?.block_on(async {
            for path in [
                "/backtest/run",
                "/pull/run",
                "/autopilot/resume",
                "/ingest/queue",
                "/pull/recovery.json",
            ] {
                assert_eq!(
                    call(Arc::clone(&app), Method::POST, path, "127.0.0.1:18081")
                        .await?
                        .0,
                    StatusCode::METHOD_NOT_ALLOWED
                );
                assert_eq!(
                    call(Arc::clone(&app), Method::GET, path, "127.0.0.1:18081")
                        .await?
                        .0,
                    StatusCode::NOT_FOUND
                );
            }
            for path in ROUTES {
                assert_eq!(
                    call(Arc::clone(&app), Method::GET, path, "127.0.0.1:18081")
                        .await?
                        .0,
                    StatusCode::BAD_REQUEST
                );
            }
            let (code, text) = call(app, Method::GET, "/viewer.json", "127.0.0.1:18081").await?;
            assert_eq!(code, StatusCode::OK);
            let body: serde_json::Value = serde_json::from_str(&text).map_err(err)?;
            for key in [
                "pull_capability",
                "sweep_capability",
                "recovery_started",
                "live_service_replaced",
                "release_clearance",
            ] {
                assert_eq!(body[key], false);
            }
            Ok::<_, String>(())
        })?;
        assert_eq!(fs::read_dir(f.0.join("store")).map_err(err)?.count(), 0);
        Ok(())
    }
    #[test]
    fn static_paths_are_bounded_and_snapshot_does_not_follow_later_changes() -> Result<()> {
        let f = Fixture::new()?;
        let app = f.app()?;
        fs::write(f.0.join("web/index.html"), b"later replacement").map_err(err)?;
        runtime()?.block_on(async {
            for path in ["/", "/backtest"] {
                assert_eq!(
                    call(Arc::clone(&app), Method::GET, path, "127.0.0.1:18081").await?,
                    (StatusCode::OK, "generated read-only shell".into())
                );
            }
            for path in ["/../viewer.rs", "/%2e%2e/viewer.rs", "/%2fetc/passwd"] {
                assert_eq!(
                    call(Arc::clone(&app), Method::GET, path, "127.0.0.1:18081")
                        .await?
                        .0,
                    StatusCode::BAD_REQUEST
                );
            }
            assert_eq!(
                call(
                    Arc::clone(&app),
                    Method::GET,
                    "/viewer.rs",
                    "127.0.0.1:18081"
                )
                .await?
                .0,
                StatusCode::NOT_FOUND
            );
            assert_eq!(
                call(Arc::clone(&app), Method::HEAD, "/", "127.0.0.1:18081")
                    .await?
                    .1,
                ""
            );
            assert_eq!(
                call(app, Method::GET, "/", "foreign.invalid:18081")
                    .await?
                    .0,
                StatusCode::FORBIDDEN
            );
            Ok::<_, String>(())
        })
    }
    #[cfg(unix)]
    #[test]
    fn static_admission_refuses_symlinks_missing_shell_and_oversize() -> Result<()> {
        let f = Fixture::new()?;
        std::os::unix::fs::symlink(f.0.join("web/index.html"), f.0.join("web/link.html"))
            .map_err(err)?;
        assert!(f.app().is_err());
        fs::remove_file(f.0.join("web/link.html")).map_err(err)?;
        File::create(f.0.join("web/huge.js"))
            .map_err(err)?
            .set_len(MAX_ASSET_BYTES + 1)
            .map_err(err)?;
        assert!(f.app().is_err());
        fs::remove_file(f.0.join("web/huge.js")).map_err(err)?;
        fs::remove_file(f.0.join("web/index.html")).map_err(err)?;
        assert!(f.app().is_err());
        Ok(())
    }
    #[test]
    fn foreign_origins_and_excess_detail_work_refuse_before_handler_admission() -> Result<()> {
        let f = Fixture::new()?;
        let app = f.app()?;
        runtime()?.block_on(async {
            for (name, value) in [
                ("origin", "http://foreign.invalid"),
                ("sec-fetch-site", "cross-site"),
                ("sec-fetch-site", "same-site"),
            ] {
                let request = Request::builder()
                    .uri("/viewer.json")
                    .header(header::HOST, "127.0.0.1:18081")
                    .header(name, value)
                    .body(Body::empty())
                    .map_err(err)?;
                assert_eq!(
                    router(Arc::clone(&app))
                        .oneshot(request)
                        .await
                        .map_err(err)?
                        .status(),
                    StatusCode::FORBIDDEN
                );
            }
            for method in [Method::PUT, Method::DELETE, Method::PATCH, Method::OPTIONS] {
                assert_eq!(
                    call(
                        Arc::clone(&app),
                        method,
                        "/boolean-qualified-search.json",
                        "127.0.0.1:18081"
                    )
                    .await?
                    .0,
                    StatusCode::METHOD_NOT_ALLOWED
                );
            }
            let held = Arc::clone(&app.detail).try_acquire_owned().map_err(err)?;
            assert_eq!(
                call(
                    Arc::clone(&app),
                    Method::GET,
                    "/boolean-qualified-search.json",
                    "127.0.0.1:18081"
                )
                .await?
                .0,
                StatusCode::TOO_MANY_REQUESTS
            );
            drop(held);
            assert_eq!(
                call(
                    app,
                    Method::GET,
                    "/boolean-qualified-search.json",
                    "127.0.0.1:18081"
                )
                .await?
                .0,
                StatusCode::BAD_REQUEST
            );
            Ok::<_, String>(())
        })
    }
    #[test]
    fn user_document_navigation_crosses_ports_but_evidence_and_frames_do_not() -> Result<()> {
        let f = Fixture::new()?;
        let app = f.app()?;
        runtime()?.block_on(async {
            for site in ["same-site", "cross-site"] {
                for path in ["/", "/backtest", "/index.html", "/viewer.json", ROUTES[0]] {
                    for mode in ["navigate", "cors"] {
                        for dest in ["document", "iframe"] {
                            for user in ["?1", "?0"] {
                                let request = Request::builder()
                                    .uri(path)
                                    .header(header::HOST, "127.0.0.1:18081")
                                    .header("sec-fetch-site", site)
                                    .header("sec-fetch-mode", mode)
                                    .header("sec-fetch-dest", dest)
                                    .header("sec-fetch-user", user)
                                    .body(Body::empty())
                                    .map_err(err)?;
                                let expected = if matches!(path, "/" | "/backtest" | "/index.html")
                                    && mode == "navigate"
                                    && dest == "document"
                                    && user == "?1"
                                {
                                    StatusCode::OK
                                } else {
                                    StatusCode::FORBIDDEN
                                };
                                assert_eq!(
                                    router(Arc::clone(&app))
                                        .oneshot(request)
                                        .await
                                        .map_err(err)?
                                        .status(),
                                    expected,
                                    "{site} {path} {mode} {dest} {user}"
                                );
                            }
                        }
                    }
                }
            }
            Ok::<_, String>(())
        })
    }
    #[test]
    fn navigation_does_not_admit_foreign_origins_duplicate_headers_or_bodies() -> Result<()> {
        let f = Fixture::new()?;
        let app = f.app()?;
        runtime()?.block_on(async {
            for (name, value, expected) in [
                ("origin", "http://127.0.0.1:18080", StatusCode::FORBIDDEN),
                ("host", "127.0.0.1:18081", StatusCode::FORBIDDEN),
                ("sec-fetch-site", "same-site", StatusCode::FORBIDDEN),
                ("sec-fetch-mode", "navigate", StatusCode::FORBIDDEN),
                ("sec-fetch-dest", "document", StatusCode::FORBIDDEN),
                ("sec-fetch-user", "?1", StatusCode::FORBIDDEN),
                ("transfer-encoding", "chunked", StatusCode::BAD_REQUEST),
                ("content-length", "1", StatusCode::BAD_REQUEST),
            ] {
                let request = Request::builder()
                    .uri("/backtest")
                    .header(header::HOST, "127.0.0.1:18081")
                    .header("sec-fetch-site", "same-site")
                    .header("sec-fetch-mode", "navigate")
                    .header("sec-fetch-dest", "document")
                    .header("sec-fetch-user", "?1")
                    .header(name, value)
                    .body(Body::empty())
                    .map_err(err)?;
                assert_eq!(
                    router(Arc::clone(&app))
                        .oneshot(request)
                        .await
                        .map_err(err)?
                        .status(),
                    expected,
                    "{name}: {value}"
                );
            }
            let request = Request::builder()
                .method(Method::HEAD)
                .uri("/backtest")
                .header(header::HOST, "127.0.0.1:18081")
                .header("sec-fetch-site", "same-site")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-dest", "document")
                .header("sec-fetch-user", "?1")
                .body(Body::empty())
                .map_err(err)?;
            assert_eq!(
                router(app).oneshot(request).await.map_err(err)?.status(),
                StatusCode::FORBIDDEN
            );
            Ok::<_, String>(())
        })
    }
    #[test]
    fn vocabulary_comes_from_linked_rust_table_and_foreign_grid_refuses() -> Result<()> {
        let f = Fixture::new()?;
        let path = std::env::var_os("BRUTEX_VIEWER_TEST_VOCABULARY")
            .ok_or("supply the captured vocabulary path for the read-only integration test")?;
        let pin = std::env::var_os("BRUTEX_VIEWER_TEST_VOCABULARY_PIN")
            .ok_or("supply the captured vocabulary BLAKE3 for the read-only integration test")?;
        let mut args = f.args();
        args.extend([path, pin]);
        let app = App::load(Config::parse(&args)?)?;
        let expected = app
            .vocabulary
            .clone()
            .ok_or("recorded vocabulary missing")?;
        runtime()?.block_on(async {
            let (code, text) = call(
                Arc::clone(&app),
                Method::GET,
                "/vocab.json",
                "127.0.0.1:18081",
            )
            .await?;
            assert_eq!(code, StatusCode::OK);
            let body: serde_json::Value = serde_json::from_str(&text).map_err(err)?;
            assert_eq!(body["api_commit"], expected["api_commit"]);
            assert_eq!(body["receipt_blake3"], expected["receipt_blake3"]);
            assert_eq!(body["vocab_version"], vocab::VOCAB_VERSION);
            let rows = body["bits"].as_array().ok_or("missing vocabulary rows")?;
            assert_eq!(rows.len(), vocab::table::COUNT);
            for (index, row) in rows.iter().enumerate() {
                let position = u16::try_from(index).map_err(err)?;
                let def = vocab::table::definition(position).ok_or("missing native row")?;
                assert_eq!(row["i"], def.index);
                assert_eq!(row["name"], def.name);
                assert_eq!(row["live"], vocab::table::is_live(position));
            }
            let digest = body["commit_digest"]
                .as_str()
                .ok_or("missing commit digest")?;
            let exact = format!(
                "/vocab.json?commit={digest}&vocab_version={}",
                vocab::VOCAB_VERSION
            );
            assert_eq!(
                call(Arc::clone(&app), Method::GET, &exact, "127.0.0.1:18081")
                    .await?
                    .0,
                StatusCode::OK
            );
            for path in [
                format!("{exact}&commit={digest}"),
                format!("/vocab.json?commit={digest}&vocab_version=0"),
                format!(
                    "/vocab.json?commit={}&vocab_version={}",
                    "00".repeat(32),
                    vocab::VOCAB_VERSION
                ),
            ] {
                assert_eq!(
                    call(Arc::clone(&app), Method::GET, &path, "127.0.0.1:18081")
                        .await?
                        .0,
                    StatusCode::SERVICE_UNAVAILABLE
                );
            }
            assert_eq!(
                call(app, Method::POST, "/vocab.json", "127.0.0.1:18081")
                    .await?
                    .0,
                StatusCode::METHOD_NOT_ALLOWED
            );
            Ok::<_, String>(())
        })?;
        assert_eq!(fs::read_dir(f.0.join("store")).map_err(err)?.count(), 0);
        Ok(())
    }
    #[test]
    fn recorded_vocabulary_refuses_changed_pins_tables_versions_and_commits() -> Result<()> {
        let f = Fixture::new()?;
        let path = f.0.join("generated-vocabulary.json");
        // Generated fixture only: no market identity or historical result is asserted.
        let good = native_vocabulary(&"11".repeat(20))?;
        let bytes = serde_json::to_vec(&good).map_err(err)?;
        fs::write(&path, &bytes).map_err(err)?;
        let pin = digest(&bytes);
        assert!(recorded_vocabulary(&path, &pin).is_ok());
        assert!(recorded_vocabulary(&path, &"00".repeat(32)).is_err());
        let text = String::from_utf8(bytes).map_err(err)?;
        for altered in [
            text.replacen('{', "{\"count\":0,", 1),
            text.replacen('{', "{\"unknown\":0,", 1),
            text.replacen("\"live\":true", "\"live\":true,\"live\":true", 1),
        ] {
            fs::write(&path, &altered).map_err(err)?;
            assert!(recorded_vocabulary(&path, &digest(altered.as_bytes())).is_err());
        }
        for kind in 0..8 {
            let mut bad = good.clone();
            match kind {
                0 => bad["api_commit"] = "not-a-commit".into(),
                1 => bad["commit_digest"] = "00".repeat(32).into(),
                2 => bad["vocab_version"] = 0.into(),
                3 => bad["count"] = 0.into(),
                4 => bad["bits"][0]["name"] = "different-rule".into(),
                5 => bad["bits"][0]["i"] = 1.into(),
                6 => bad["bits"][0]["live"] = false.into(),
                _ => {
                    bad["bits"]
                        .as_array_mut()
                        .ok_or("generated rows missing")?
                        .pop();
                }
            }
            let bytes = serde_json::to_vec(&bad).map_err(err)?;
            fs::write(&path, &bytes).map_err(err)?;
            assert!(
                recorded_vocabulary(&path, &digest(&bytes)).is_err(),
                "mutation {kind}"
            );
        }
        fs::write(&path, b"{malformed").map_err(err)?;
        assert!(recorded_vocabulary(&path, &digest(b"{malformed")).is_err());
        File::create(&path)
            .map_err(err)?
            .set_len(128 * 1024 + 1)
            .map_err(err)?;
        assert!(recorded_vocabulary(&path, &pin).is_err());
        let mut args = f.args();
        args.extend([path.as_os_str().into(), "00".repeat(32).into()]);
        assert!(Config::parse(&args).is_ok());
        assert!(App::load(Config::parse(&args)?).is_err());
        args[6] = "relative-file.json".into();
        assert!(Config::parse(&args).is_err());
        args[6] = path.into_os_string();
        args[7] = "AA".repeat(32).into();
        assert!(Config::parse(&args).is_err());
        Ok(())
    }
    #[test]
    fn exact_saved_search_rejects_foreign_pin_through_existing_handler() -> Result<()> {
        // Explicit standalone test inputs; never silently skip this integration.
        let identity = std::env::var("BRUTEX_VIEWER_TEST_SEARCH")
            .map_err(|_| "supply the completed search ID for the read-only integration test")?;
        let store = std::env::var_os("BRUTEX_STORE").ok_or("test BRUTEX_STORE missing")?;
        let f = Fixture::new()?;
        let mut args = f.args();
        args[0] = store;
        let config = Config::parse(&args)?;
        config.require_environment()?;
        let app = App::load(config)?;
        runtime()?.block_on(async {
            let (code, text) = call(
                Arc::clone(&app),
                Method::GET,
                &format!("/boolean-qualified-search.json?identity={identity}"),
                "127.0.0.1:18081",
            )
            .await?;
            assert_eq!(code, StatusCode::OK, "{text}");
            let body: serde_json::Value = serde_json::from_str(&text).map_err(err)?;
            let pin = body["pin"].as_str().ok_or("actual search pin absent")?;
            let wrong = if pin == "11".repeat(32) {
                "22".repeat(32)
            } else {
                "11".repeat(32)
            };
            let (code, text) = call(
                app,
                Method::GET,
                &format!("/boolean-qualified-search.json?identity={identity}&pin={wrong}"),
                "127.0.0.1:18081",
            )
            .await?;
            assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
            assert!(text.contains("checkpoint pin changed"), "{text}");
            Ok::<_, String>(())
        })
    }
}
