use super::*;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt as _;

static SERIAL: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    config: Config,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-main-inspector-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("unique fixture");
        let root = fs::canonicalize(root).unwrap();
        for path in ["masters", "store", "web", "web/build"] {
            fs::create_dir(root.join(path)).expect("directory");
        }
        fs::write(
            root.join("web/build/index.html"),
            "<h1>Inspection fixture</h1>",
        )
        .unwrap();
        fs::write(
            root.join("store/retained-evidence.bin"),
            b"retained evidence bytes",
        )
        .unwrap();
        let config = Config {
            masters: root.join("masters"),
            store: root.join("store"),
            web: root.join("web"),
            address: "127.0.0.1:8080".parse().unwrap(),
            saved_results_url: None,
        };
        Self { root, config }
    }
    fn args(&self) -> Vec<OsString> {
        vec![
            self.config.masters.clone().into_os_string(),
            self.config.store.clone().into_os_string(),
            self.config.web.clone().into_os_string(),
            "127.0.0.1".into(),
            "8080".into(),
        ]
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("only owned fixture cleanup");
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, current: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                out.insert(relative, None);
                walk(root, &path, out);
            } else {
                out.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn request(method: Method, path: &str) -> Request {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap()
}

fn spy(config: &Config, counter: Arc<AtomicUsize>, assets: BTreeSet<String>) -> Router {
    Router::new()
        .fallback(move || {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                "inner handler entered"
            }
        })
        .layer(middleware::from_fn_with_state(
            guard(config, assets),
            protect,
        ))
}

#[tokio::test]
async fn every_execution_verb_and_unknown_get_refuses_before_inner_handler() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let router = spy(&f.config, count.clone(), BTreeSet::new());
    for path in [
        "/backtest/run",
        "/backtest/descend",
        "/engine/command",
        "/pull/run",
        "/pull/recovery",
        "/autopilot/resume",
        "/masters/refresh",
        "/store.json",
    ] {
        for method in [
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ] {
            let response = router.clone().oneshot(request(method, path)).await.unwrap();
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
            assert_eq!(response.headers()[header::ALLOW], "GET, HEAD");
        }
    }
    for path in [
        "/gaps.json",
        "/pull/recovery",
        "/folder.json",
        "/unknown.json",
        "/unknown",
        "/pull",
        "/%67aps.json",
        "/../store.json",
    ] {
        let response = router
            .clone()
            .oneshot(request(Method::GET, path))
            .await
            .unwrap();
        assert!(response.status().is_client_error(), "{path}");
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn foreign_authorities_duplicate_headers_and_request_bodies_never_enter_inner_handler() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let router = spy(&f.config, count.clone(), BTreeSet::new());
    let mut cases = Vec::new();
    for (key, value) in [
        ("host", "example.com:8080"),
        ("origin", "http://127.0.0.1:8081"),
        ("origin", "null"),
        ("sec-fetch-site", "cross-site"),
        ("sec-fetch-site", "same-site"),
        ("x-forwarded-host", "127.0.0.1:8080"),
        ("forwarded", "host=127.0.0.1:8080"),
        ("content-length", "1"),
        ("transfer-encoding", "chunked"),
    ] {
        let mut req = request(Method::GET, "/store.json");
        req.headers_mut().insert(
            axum::http::HeaderName::from_bytes(key.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
        cases.push(req);
    }
    for key in [
        header::HOST,
        header::ORIGIN,
        axum::http::HeaderName::from_static("sec-fetch-site"),
        header::CONTENT_LENGTH,
    ] {
        let mut req = request(Method::GET, "/store.json");
        req.headers_mut().append(key.clone(), "0".parse().unwrap());
        req.headers_mut().append(key, "0".parse().unwrap());
        cases.push(req);
    }
    let mut missing_host = request(Method::GET, "/store.json");
    missing_host.headers_mut().remove(header::HOST);
    cases.push(missing_host);
    for req in cases {
        assert!(
            router
                .clone()
                .oneshot(req)
                .await
                .unwrap()
                .status()
                .is_client_error()
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn exact_reads_and_head_are_admitted_with_explicit_read_only_headers() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let router = spy(&f.config, count.clone(), BTreeSet::new());
    for method in [Method::GET, Method::HEAD] {
        let mut req = request(method.clone(), "/store.json?feed=zerodha");
        req.headers_mut()
            .insert(header::ORIGIN, "http://127.0.0.1:8080".parse().unwrap());
        req.headers_mut()
            .insert("sec-fetch-site", "same-origin".parse().unwrap());
        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-brutex-mode"], MODE);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(body.is_empty(), method == Method::HEAD);
    }
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn busy_readers_refuse_but_static_assets_still_load() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let admission = guard(&f.config, ["/_app/app.js".to_owned()].into_iter().collect());
    let permit = admission
        .reads
        .clone()
        .acquire_many_owned(MAX_READS as u32)
        .await
        .unwrap();
    let seen = count.clone();
    let router = Router::new()
        .fallback(move || {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                "read"
            }
        })
        .layer(middleware::from_fn_with_state(admission, protect));
    let busy = router
        .clone()
        .oneshot(request(Method::GET, "/store.json"))
        .await
        .unwrap();
    assert_eq!(busy.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(busy.headers()[header::RETRY_AFTER], "1");
    assert_eq!(
        router
            .clone()
            .oneshot(request(Method::GET, "/_app/app.js"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    drop(permit);
    assert_eq!(
        router
            .oneshot(request(Method::GET, "/store.json"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn actual_read_handlers_preserve_every_fixture_byte_and_directory() {
    let f = Fixture::new();
    let before = snapshot(&f.root);
    let router = app(&f.config).expect("real read router");
    for (path, status) in [
        ("/store.json?feed=zerodha", StatusCode::OK),
        ("/feeds.json", StatusCode::OK),
        ("/vocab.json", StatusCode::OK),
        ("/health", StatusCode::SERVICE_UNAVAILABLE),
        ("/inspection.json", StatusCode::OK),
    ] {
        let response = router
            .clone()
            .oneshot(request(Method::GET, path))
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{path}");
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        assert!(!body.is_empty(), "{path} returned an actual response");
        if path.starts_with("/store.json") {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                serde_json::json!([])
            );
        }
        if path == "/vocab.json" {
            let vocab: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let bits = vocab["bits"].as_array().expect("actual vocabulary table");
            assert_eq!(vocab["count"].as_u64().unwrap(), bits.len() as u64);
            assert!(bits.iter().any(|bit| {
                bit["i"] == 52
                    && bit["name"]
                        .as_str()
                        .is_some_and(|name| name.to_ascii_lowercase().contains("vwap"))
            }));
        }
    }
    assert_eq!(
        snapshot(&f.root),
        before,
        "no store, recovery, log, master or frontend writes"
    );
    assert!(!f.config.store.join("audit/recovery-v1").exists());
    assert!(!f.config.store.join("serve.lock").exists());
}

#[test]
fn explicit_roots_environment_and_loopback_are_required_without_secret_discovery() {
    let f = Fixture::new();
    assert!(Config::parse(&f.args()).is_ok());
    for ip in ["0.0.0.0", "192.168.1.1", "example.com"] {
        let mut args = f.args();
        args[3] = ip.into();
        assert!(Config::parse(&args).is_err());
    }
    for port in ["0", "08080", "65536", "-1"] {
        let mut args = f.args();
        args[4] = port.into();
        assert!(Config::parse(&args).is_err());
    }
    let mut args = f.args();
    args[1] = args[0].clone();
    assert!(Config::parse(&args).is_err());
    assert!(f.config.environment(|_| None).is_err());
    let mut names = BTreeSet::new();
    f.config
        .environment(|name| {
            names.insert(name.to_owned());
            match name {
                "BRUTEX_STORE" => Some(f.config.store.clone().into_os_string()),
                "BRUTEX_MASTERS" => Some(f.config.masters.clone().into_os_string()),
                "BRUTEX_AUTOPILOT" => Some("pause".into()),
                "BRUTEX_ARCHIVE_SUGGESTIONS" => Some("0".into()),
                _ => panic!("unexpected environment discovery"),
            }
        })
        .unwrap();
    assert_eq!(names.len(), 4);
}

#[test]
fn asset_inventory_cannot_open_extra_backend_routes_and_bounds_empty_directories() {
    let f = Fixture::new();
    fs::write(f.config.web.join("build/gaps.json"), "{}").unwrap();
    fs::write(
        f.config.web.join("build/pull"),
        "untrusted route-shaped file",
    )
    .unwrap();
    let paths = collect_assets(&f.config.web).unwrap();
    assert!(!paths.contains("/gaps.json"));
    assert!(!paths.contains("/pull"));
    assert!(paths.contains("/index.html"));
    fs::create_dir(f.config.web.join("build/one")).unwrap();
    fs::create_dir(f.config.web.join("build/two")).unwrap();
    assert!(
        collect_assets_bounded(&f.config.web, 4)
            .unwrap_err()
            .contains("entry count")
    );
}

#[cfg(unix)]
#[test]
fn asset_symlinks_and_oversized_files_refuse_without_reading_contents() {
    let f = Fixture::new();
    std::os::unix::fs::symlink(&f.config.store, f.config.web.join("build/leak")).unwrap();
    assert!(
        collect_assets(&f.config.web)
            .unwrap_err()
            .contains("symlink")
    );
    fs::remove_file(f.config.web.join("build/leak")).unwrap();
    let oversized = File::create(f.config.web.join("build/huge.js")).unwrap();
    oversized.set_len(MAX_ASSET_BYTES + 1).unwrap();
    assert!(
        collect_assets(&f.config.web)
            .unwrap_err()
            .contains("byte limit")
    );
}

#[test]
fn read_lock_never_creates_a_file_and_refuses_an_existing_writer() {
    let f = Fixture::new();
    let before = snapshot(&f.config.store);
    assert!(writer_absence(&f.config.store).unwrap().is_none());
    assert_eq!(snapshot(&f.config.store), before);
    let path = f.config.store.join("serve.lock");
    fs::write(&path, b"retained lock marker").unwrap();
    let owner = File::open(&path).unwrap();
    owner.lock().unwrap();
    assert!(writer_absence(&f.config.store).is_err());
    drop(owner);
    assert!(writer_absence(&f.config.store).unwrap().is_some());
    assert_eq!(fs::read(path).unwrap(), b"retained lock marker");
}

fn link() -> String {
    format!(
        "http://127.0.0.1:8081/backtest?identity={}&pin={}&batch=1&rung=0&offset=480&setting=503",
        "a".repeat(64),
        "b".repeat(64)
    )
}

#[test]
fn saved_link_preserves_only_an_explicit_pinned_local_evidence_selection() {
    let good = link();
    assert_eq!(saved_link(OsStr::new(&good)).unwrap(), good);
    for bad in [
        good.replace("127.0.0.1", "example.com"),
        good.replace("http:", "https:"),
        good.replace("/backtest?", "/pull/run?"),
        format!("{good}#fragment"),
        format!("{good}&pin={}", "c".repeat(64)),
        good.replace("&rung=0", "&rung=8"),
        good.replace("&batch=1", "&batch=01"),
        good.replace("&setting=503", "&setting=512"),
        good.replace("&offset=480", "&offset=18446744073709551616"),
        good.replace("identity=a", "identity=%61"),
    ] {
        assert!(saved_link(OsStr::new(&bad)).is_err(), "{bad}");
    }
    let f = Fixture::new();
    let mut args = f.args();
    args.push(good.clone().into());
    let config = Config::parse(&args).unwrap();
    let info = metadata(&config);
    assert_eq!(info["saved_results_url"], good);
    assert_eq!(info["can_sweep"], false);
    assert_eq!(info["release_cleared"], false);
    assert_eq!(info["source_stamp"]["status"], "unverified_cached_build");
    assert!(info["source_stamp"]["api_source_commit"].is_null());
    assert!(metadata(&f.config)["saved_results_url"].is_null());
}

#[tokio::test]
async fn oversize_url_is_refused_without_entering_a_reader() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let router = spy(&f.config, count.clone(), BTreeSet::new());
    let response = router
        .oneshot(request(
            Method::GET,
            &format!("/store.json?{}", "q".repeat(MAX_URI_BYTES)),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::URI_TOO_LONG);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

fn document_request(path: &str, site: &str) -> Request {
    let mut req = request(Method::GET, path);
    for (name, value) in [
        ("sec-fetch-site", site),
        ("sec-fetch-mode", "navigate"),
        ("sec-fetch-dest", "document"),
        ("sec-fetch-user", "?1"),
    ] {
        req.headers_mut().insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
    }
    req
}

#[tokio::test]
async fn cross_port_user_document_navigation_opens_only_html_pages() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let router = spy(&f.config, count.clone(), BTreeSet::new());
    for site in ["same-site", "cross-site"] {
        for path in ["/db", "/backtest", "/"] {
            let response = router
                .clone()
                .oneshot(document_request(path, site))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{site}: {path}");
        }
        for path in [
            "/store.json",
            "/inspection.json",
            "/backtest/run",
            "/gaps.json",
            "/unlisted",
        ] {
            let response = router
                .clone()
                .oneshot(document_request(path, site))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{site}: {path}");
        }
    }
    assert_eq!(count.load(Ordering::SeqCst), 6);
}

#[tokio::test]
async fn document_exception_rejects_frames_background_requests_origins_bodies_and_wrong_hosts() {
    let f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let router = spy(&f.config, count.clone(), BTreeSet::new());
    let mut cases = Vec::new();
    for (name, value) in [
        ("sec-fetch-mode", "cors"),
        ("sec-fetch-dest", "iframe"),
        ("sec-fetch-user", "?0"),
        ("host", "127.0.0.1:8081"),
        ("origin", "http://127.0.0.1:8080"),
        ("origin", "http://127.0.0.1:8081"),
        ("content-length", "1"),
    ] {
        let mut req = document_request("/db", "same-site");
        req.headers_mut().insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
        cases.push(req);
    }
    for name in ["sec-fetch-user", "sec-fetch-mode", "sec-fetch-dest"] {
        let mut missing = document_request("/db", "same-site");
        missing.headers_mut().remove(name);
        cases.push(missing);
        let mut duplicated = document_request("/db", "same-site");
        let value = duplicated.headers().get(name).unwrap().clone();
        duplicated.headers_mut().append(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value,
        );
        cases.push(duplicated);
    }
    let mut post = document_request("/db", "same-site");
    *post.method_mut() = Method::POST;
    cases.push(post);
    let mut head = document_request("/db", "same-site");
    *head.method_mut() = Method::HEAD;
    cases.push(head);
    for req in cases {
        assert!(
            router
                .clone()
                .oneshot(req)
                .await
                .unwrap()
                .status()
                .is_client_error()
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
