//! `CLAUDE.md` §8 on the pull path that actually ships.
//!
//! *"This repository never mints a token. A stale token is re-read; if the
//! re-read returns the same dead value, the pull halts loudly."*
//!
//! # What was wrong
//!
//! `pull::secret::CredentialReader::reread_after_rejection` states that rule as
//! a function, and nothing outside `cfg(test)` called it. The shipped pull read
//! Parameter Store through `pull::ssm::get_parameter` on every instrument and
//! never compared a re-read with the value the vendor had just rejected, so a
//! dead token produced a 401 per instrument for the whole universe, and the F&O
//! walks built one source and sent its dead token to every remaining cell
//! (GAP2-36). The autopilot then printed *"re-reading it returned the same
//! value"* about a comparison nobody had made, and did it for any reason whose
//! text contained the word `credential`, including an unreachable Parameter
//! Store (GAP2-37). D-0948.
//!
//! # What this module is
//!
//! * [`Credentials`]: where a run's credential comes from. In production that
//!   is Parameter Store. Under `cfg(test)` a scripted source stands in, so the
//!   shipped loops can be driven against a fake vendor with no AWS and no
//!   network beyond loopback.
//! * [`Watch`]: one run's memory of which credential it last sent and which it
//!   knows to be dead, and the one verdict that stops the run.
//! * [`Watch::reread`]: the re-read itself. It is called once per rejection,
//!   never in a loop. It compares fingerprints, so the secret is never held
//!   twice and never logged ([`pull::http::CredentialPrint`]).
//!
//! # Cost
//!
//! O(1) per instrument or cell: one fingerprint comparison on the send path.
//! A rejection adds one Parameter Store read and one SHA-256. Neither grows
//! with the universe, the window or the cross product.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use pull::http::{CredentialPrint, HttpSource};

/// Why a credential could not be read, as a class an operator acts on.
///
/// # Two classes, because they go to two different places
///
/// A wrong or missing configuration does not fix itself between attempts:
/// HOME unset, `credentials.toml` unusable, no AWS identity, a parameter path
/// that will not build, Parameter Store answering `AccessDenied` or
/// `ParameterNotFound`. Retrying spends attempts on a fault only an operator can
/// fix, so the run stops and says which configuration to look at.
///
/// An unreachable Parameter Store is different: a timeout, a throttle, or a
/// 5xx from AWS. That is transport and is retried like transport. Halting a
/// feed for the life of the process over a network blip, and calling it a dead
/// token, is the GAP2-37 defect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unread {
    /// The configuration or the AWS identity is wrong. Retrying cannot fix it.
    Configuration,
    /// Parameter Store was not reached or did not answer. Worth retrying.
    Transport,
}

/// A credential read that produced no credential, with its class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// Which of the two places this sends an operator.
    pub class: Unread,
    /// The reason, which never contains the value or the assembled path.
    pub why: String,
}

impl Unreadable {
    /// A configuration fault.
    #[must_use]
    pub const fn configuration(why: String) -> Self {
        Self {
            class: Unread::Configuration,
            why,
        }
    }

    /// A Parameter Store that was not reached.
    #[must_use]
    pub const fn transport(why: String) -> Self {
        Self {
            class: Unread::Transport,
            why,
        }
    }

    /// The class Parameter Store's own answer belongs to.
    ///
    /// `Unreachable` covers a socket that did not answer, a throttle and a
    /// 5xx, and is transport. `AccessDenied`, `NotFound` and `Empty` say the
    /// role or the path is wrong, and are configuration.
    #[must_use]
    pub const fn of_secret(kind: pull::secret::SecretError, why: String) -> Self {
        match kind {
            pull::secret::SecretError::Unreachable => Self::transport(why),
            _ => Self::configuration(why),
        }
    }
}

impl core::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.why)
    }
}

/// The run's credential verdict. It is decided by a comparison, never by
/// reading prose.
///
/// Carried out of `broker_run` on `BrokerRun::credential_stop` and into
/// `autopilot::TickOutcome::credential`, where it alone can halt a feed as a
/// dead credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStop {
    /// The vendor rejected the credential and the one re-read returned the
    /// same value, by fingerprint. Nothing in this process can fix it.
    SameValue,
    /// A credential read failed, and the run stopped because of it.
    ///
    /// The failure was either the re-read after a rejection or a
    /// configuration fault that would fail every instrument the same way.
    /// No equality was established, so nothing may claim the value was
    /// unchanged.
    Unreadable(Unread),
}

/// Where a run's credential is read from.
///
/// One production arm. The `Scripted` arm exists only under `cfg(test)`, so a
/// shipped binary cannot take its credential from anywhere but Parameter Store.
#[derive(Debug, Clone, Default)]
pub enum Credentials {
    /// AWS Systems Manager Parameter Store, through
    /// `server::credentialed_source`. The only production source.
    #[default]
    Ssm,
    /// A test double, answering from a script.
    #[cfg(test)]
    Scripted(std::sync::Arc<Script>),
}

impl Credentials {
    /// One credential read and the source built from it, for one feed.
    ///
    /// # Errors
    ///
    /// [`Unreadable`], classed. A feed with no HTTP transport has no
    /// credential to read, and that is a configuration fault.
    pub async fn source(
        &self,
        feed: pull::vendor::Feed,
    ) -> Result<(HttpSource, brutex_core::vendor::Vendor), Unreadable> {
        let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
            return Err(Unreadable::configuration(format!(
                "{} is a local archive and has no credential to read",
                feed.display()
            )));
        };
        match self {
            Self::Ssm => crate::server::credentialed_source(feed, &spec).await,
            #[cfg(test)]
            Self::Scripted(script) => script.source(feed, &spec),
        }
    }
}

/// A scripted credential source, for tests only.
///
/// Answers are given in order. Once the script runs out, the last answer is
/// repeated, which is what a dead token in Parameter Store looks like: the same
/// value on every read until somebody rotates it. Each answer builds a real
/// [`HttpSource`] against `base_url`, so the request reaches a loopback fake
/// vendor carrying exactly the token the script handed out.
#[cfg(test)]
#[derive(Debug)]
pub struct Script {
    answers: std::sync::Mutex<std::collections::VecDeque<Result<String, Unreadable>>>,
    last: std::sync::Mutex<Option<Result<String, Unreadable>>>,
    base_url: &'static str,
    reads: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl Script {
    /// A script, against a vendor at `base_url`.
    #[must_use]
    pub fn new(base_url: &'static str, answers: Vec<Result<String, Unreadable>>) -> Self {
        Self {
            answers: std::sync::Mutex::new(answers.into()),
            last: std::sync::Mutex::new(None),
            base_url,
            reads: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// How many times the credential has been read.
    #[must_use]
    pub fn reads(&self) -> usize {
        self.reads.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn source(
        &self,
        feed: pull::vendor::Feed,
        spec: &pull::vendor::HttpSpec,
    ) -> Result<(HttpSource, brutex_core::vendor::Vendor), Unreadable> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let next = self
            .answers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front();
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let answer = match next {
            Some(answer) => {
                *last = Some(answer.clone());
                answer
            }
            None => last.clone().unwrap_or_else(|| {
                Err(Unreadable::configuration("the script is empty".to_owned()))
            }),
        };
        let token = answer?;
        let spec = pull::vendor::HttpSpec {
            base_url: self.base_url,
            ..*spec
        };
        let vendor = feed
            .store_vendor()
            .ok_or_else(|| Unreadable::configuration("no store prefix".to_owned()))?;
        let source = HttpSource::new(spec, pull::http::Credential::token(token))
            .map_err(|why| Unreadable::configuration(why.to_string()))?;
        Ok((source, vendor))
    }
}

/// What one rejection's re-read decided.
#[derive(Debug)]
pub enum Reread {
    /// The value changed. The run continues, and an F&O walk sends the
    /// remaining cells with this source.
    Rotated(Box<HttpSource>),
    /// The run stops, with the verdict and the reason. [`Watch::stop`] holds
    /// the same pair.
    Halted(CredentialStop, String),
}

/// One run's credential memory. Constructed per run and dropped with it.
///
/// # What it remembers, and why each one is needed
///
/// * `sent_with`: the fingerprint of the credential the last request carried.
///   That is the value a rejection rejected, and the re-read is compared with
///   it.
/// * `dead`: EVERY fingerprint the vendor rejected and a later read replaced,
///   in this run. If a later read hands any of them back, for example because
///   a rotation was undone, it is refused before a socket is opened -- by
///   [`Self::admit`] on the spot loop and by [`Self::reread`] itself on the F&O
///   walks, which send a rotated source without admitting it. Sending a
///   credential already known to be dead is the request this module exists to
///   stop. It held ONE fingerprint and only `admit` read it, so on a walk the
///   sequence A rejected, B read, B rejected, A read sent A again (v3a-1,
///   D-1482). A list, not a set: the print has a constant-time `PartialEq` and
///   deliberately no `Hash`, so a membership test is a walk over the dead
///   values, O(d) where d is the number of rotations this run has seen, each
///   bought by one vendor rejection. **UNVERIFIED as a measurement**:
///   no test times it; `docs/06-limits.md` names it.
/// * `stop`: the verdict, once there is one. Every loop checks it after each
///   instrument or cell and breaks.
#[derive(Debug, Default)]
pub struct Watch {
    sent_with: Option<CredentialPrint>,
    dead: Vec<CredentialPrint>,
    stop: Option<(CredentialStop, String)>,
}

impl Watch {
    /// A watch for a walk that already holds the source it will send with.
    #[must_use]
    pub fn sending(source: &HttpSource) -> Self {
        Self {
            sent_with: Some(source.credential_print()),
            ..Self::default()
        }
    }

    /// Admits a freshly read credential onto the wire, or refuses one the
    /// vendor already rejected in this run.
    ///
    /// # Errors
    ///
    /// The halt reason, already recorded on [`Self::stop`], when `source`
    /// carries a value this run knows to be dead.
    pub fn admit(&mut self, feed: pull::vendor::Feed, source: &HttpSource) -> Result<(), String> {
        let print = source.credential_print();
        if self.is_dead(print) {
            let why = format!(
                "{}: a credential read returned the access-token the vendor already rejected \
                 in this run, so it was not sent. {}",
                feed.display(),
                NEVER_MINTS
            );
            self.stop = Some((CredentialStop::SameValue, why.clone()));
            return Err(why);
        }
        self.sent_with = Some(print);
        Ok(())
    }

    /// Whether the vendor already rejected `print` in this run. O(d), see the
    /// type's docs.
    fn is_dead(&self, print: CredentialPrint) -> bool {
        self.dead.iter().any(|dead| *dead == print)
    }

    /// Notes a credential read that produced nothing.
    ///
    /// A configuration fault stops the run, because every later instrument
    /// would fail the same way. A transport fault does not: that instrument is
    /// refused and the next one reads again.
    pub fn unreadable(&mut self, feed: pull::vendor::Feed, failed: &Unreadable) {
        if failed.class == Unread::Configuration {
            self.stop = Some((
                CredentialStop::Unreadable(Unread::Configuration),
                format!(
                    "{}: the credential configuration is not usable, so the pull stopped \
                     rather than fail every remaining instrument the same way. No vendor \
                     rejected anything and no re-read was made. Fix the configuration it \
                     names: {}",
                    feed.display(),
                    failed.why
                ),
            ));
        }
    }

    /// The re-read after a vendor rejected the credential this run last sent.
    ///
    /// Called ONCE per rejection, by the loop that saw it. It does not loop,
    /// and it never mints. Each arm emits one telemetry line naming the vendor
    /// and the field and never the value:
    ///
    /// * the re-read fails: the run halts with that reason;
    /// * the re-read returns the rejected value: the run halts, because the
    ///   token is dead and this repository cannot replace it;
    /// * the re-read returns a value the vendor rejected EARLIER in this run:
    ///   the run halts the same way, because that token is dead too (v3a-1);
    /// * the re-read returns a different value: the rejected print is
    ///   remembered as dead and the new source is handed back.
    pub async fn reread(&mut self, credentials: &Credentials, feed: pull::vendor::Feed) -> Reread {
        let rejected = self.sent_with;
        match credentials.source(feed).await {
            Err(failed) => {
                note(feed, "the re-read after a rejection failed", None);
                self.halt(
                    CredentialStop::Unreadable(failed.class),
                    format!(
                        "{}: the vendor rejected the access-token and the one re-read after it \
                         failed, so the pull halted rather than send the rejected value again. \
                         The re-read said: {}",
                        feed.display(),
                        failed.why
                    ),
                )
            }
            Ok((source, _)) => {
                let fresh = source.credential_print();
                let rotated = rejected != Some(fresh);
                if rotated && self.is_dead(fresh) {
                    note(
                        feed,
                        "re-read returned a value the vendor already rejected in this run; \
                         the pull halted",
                        Some(false),
                    );
                    return self.halt(
                        CredentialStop::SameValue,
                        format!(
                            "{}: the vendor rejected the access-token and the one re-read \
                             returned a value the vendor already rejected in this run, \
                             compared by fingerprint, so it was not sent. {}",
                            feed.display(),
                            NEVER_MINTS
                        ),
                    );
                }
                note(
                    feed,
                    if rotated {
                        "re-read returned a different value; continuing with it"
                    } else {
                        "re-read returned the SAME value; the token is dead and the pull halted"
                    },
                    Some(rotated),
                );
                if rotated {
                    if let Some(rejected) = rejected
                        && !self.is_dead(rejected)
                    {
                        self.dead.push(rejected);
                    }
                    self.sent_with = Some(fresh);
                    return Reread::Rotated(Box::new(source));
                }
                self.halt(
                    CredentialStop::SameValue,
                    format!(
                        "{}: the vendor rejected the access-token and the one re-read returned \
                         the same value, compared by fingerprint. {}",
                        feed.display(),
                        NEVER_MINTS
                    ),
                )
            }
        }
    }

    /// Records the verdict and hands it back.
    fn halt(&mut self, stop: CredentialStop, why: String) -> Reread {
        self.stop = Some((stop, why.clone()));
        Reread::Halted(stop, why)
    }

    /// The verdict, if the run must stop.
    #[must_use]
    pub fn stop(&self) -> Option<&(CredentialStop, String)> {
        self.stop.as_ref()
    }
}

/// The half of every same-value halt that says what can and cannot fix it.
const NEVER_MINTS: &str = "CLAUDE.md §8: this repository never mints a token, so nothing \
     further was sent. Rotate it where it is minted, in AWS Parameter Store, then pull again.";

/// One line on the rolling log per re-read. The vendor and the field name are
/// logged, and the value never is.
fn note(feed: pull::vendor::Feed, what: &str, rotated: Option<bool>) {
    let event = telemetry::Event::new(
        if rotated == Some(true) {
            telemetry::Level::Info
        } else {
            telemetry::Level::Warn
        },
        "pull.credential",
        what,
    )
    .with("feed", telemetry::Value::Str(feed.wire()))
    .with("field", telemetry::Value::Str("access-token"));
    let event = match rotated {
        Some(rotated) => event.with("rotated", telemetry::Value::Bool(rotated)),
        None => event,
    };
    let _dropped_when_filtered = telemetry::emit(&event);
}
