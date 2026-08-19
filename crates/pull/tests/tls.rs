//! **The TLS provider is installed, and installing it twice is not an error.**
//!
//! `rustls` 0.23 carries no cryptography of its own, so a provider must be
//! chosen before the first client is built. Reaching
//! [`reqwest::Client::builder`] without one compiles cleanly and panics at run
//! time with `No provider set` — a failure the compiler cannot see, and one that
//! a test only finds if it happens to drive that exact path.
//!
//! # What is asserted, and what deliberately is not
//!
//! **Asserted:** the provider installs, a second install is a no-op rather than
//! a panic, and a client builds. Building the client is the assertion that
//! matters: it is the precise operation that panics when no provider is set, so
//! this test fails for the real reason rather than by inspecting a flag.
//!
//! **Not asserted:** that a handshake completes with any host. That needs a
//! network, and a test which reaches the network fails for reasons that are not
//! about this workspace — a proxy, an outage, a runner without egress. The
//! handshake was verified by hand against three production stacks before this
//! change landed, including `aws.amazon.com`, which is the stack
//! [`pull::ssm`](../src/ssm.rs) reads Parameter Store over. D-0211 records it.

/// Installing twice is safe, and a client builds afterwards.
///
/// One test rather than two: the second call has to happen *before* the client
/// is built for the no-op to be worth anything, and splitting them would let a
/// runner order them the other way round.
#[test]
fn the_provider_installs_idempotently_and_a_client_builds() {
    // Twice, deliberately. `install_default` refuses a second call, so a
    // `ensure_tls_provider` that forwarded the error would panic here.
    pull::ensure_tls_provider();
    pull::ensure_tls_provider();

    let built = reqwest::Client::builder()
        .timeout(core::time::Duration::from_secs(5))
        .build();

    assert!(
        built.is_ok(),
        "a client must build once the provider is installed; without one this \
         call panics with `No provider set` rather than returning an error"
    );
}

/// The guard runs even when nothing else in the process has touched `pull`.
///
/// The function is `pub` and the three call sites are inside this crate, so a
/// consumer that builds its own client is expected to call it. This test drives
/// it from outside the crate — an integration test, not a unit one — so the
/// path a consumer takes is the path measured.
#[test]
fn a_caller_outside_the_crate_can_install_the_provider() {
    pull::ensure_tls_provider();
    let built = reqwest::Client::builder().build();
    assert!(built.is_ok(), "an external caller gets a usable client");
}
