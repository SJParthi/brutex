//! Rows of `credential_law.rs` no test reached before BASE-COV-API repair 3
//! (shard AS1, D-8661): a re-read before anything was sent, a rejected value
//! the watch already remembers as dead, and a script with no answer at all.
//! Every token is invented; no request leaves the process (the scripted
//! source's address is never dialled by a re-read).

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a whole-file test module; a failed assertion is the test failing"
)]

use super::*;

/// The scripted source's vendor address. A re-read builds a source against
/// it and sends nothing.
const BASE: &str = "http://127.0.0.1:9";

/// A feed whose transport is HTTP and whose auth takes one token.
const FEED: pull::vendor::Feed = pull::vendor::Feed::Dhan;

fn credentials(answers: Vec<Result<String, Unreadable>>) -> Credentials {
    Credentials::Scripted(std::sync::Arc::new(Script::new(BASE, answers)))
}

/// The print of a source built from `value`.
async fn print_of(value: &str) -> CredentialPrint {
    credentials(vec![Ok(value.to_owned())])
        .source(FEED)
        .await
        .expect("a scripted source")
        .0
        .credential_print()
}

/// `watch.reread` over a script answering `value` once.
async fn reread(watch: &mut Watch, value: &str) -> Reread {
    watch
        .reread(&credentials(vec![Ok(value.to_owned())]), FEED)
        .await
}

/// **EACH DEAD VALUE IS REMEMBERED ONCE, WHETHER OR NOT ANYTHING WAS SENT
/// BEFORE THE RE-READ** (BASE-COV-API repair 3, D-8661). `Watch::reread`
/// over the states the pull loops build and two no public call can, written
/// here field by field. Extremes: a re-read before anything was sent
/// (rotated, nothing remembered dead, the fresh value now the one sent); a
/// rejected value the watch already remembers as dead (rotated, remembered
/// once, never twice); a rejected value it does not remember (remembered);
/// a re-read that returns the rejected value (halted, same value); one that
/// returns a value rejected earlier in the run (halted, already rejected);
/// one that fails at the transport (halted, unreadable, its class kept); an
/// empty script (refused as a configuration fault, never a token); a
/// script that ran out (its last answer repeated); and, at once, nothing
/// sent with the re-read returning a dead value (halted: a rotation is no
/// pass for a dead value), and a rejected value already dead with the
/// re-read returning another dead value (halted, the dead list unchanged);
/// out of the box, a re-read whose value differs from the rejected one in
/// its last byte alone (rotated: the print compares every byte).
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "every watch state beside the others one re-read is decided against"
)]
async fn each_dead_value_is_remembered_once_whether_or_not_anything_was_sent() {
    let a = print_of("invented-token-a").await;
    let b = print_of("invented-token-b").await;
    assert_ne!(a, b);

    // NOTHING SENT BEFORE THE RE-READ.
    let mut fresh = Watch::default();
    assert!(matches!(
        reread(&mut fresh, "invented-token-a").await,
        Reread::Rotated(_)
    ));
    assert!(fresh.dead.is_empty(), "nothing was rejected: {fresh:?}");
    assert_eq!(fresh.sent_with, Some(a));
    assert!(fresh.stop().is_none());

    // THE REJECTED VALUE ALREADY DEAD: REMEMBERED ONCE.
    let mut known = Watch {
        sent_with: Some(a),
        dead: vec![a],
        stop: None,
    };
    assert!(matches!(
        reread(&mut known, "invented-token-b").await,
        Reread::Rotated(_)
    ));
    assert_eq!(known.dead, [a], "once, never twice");
    assert_eq!(known.sent_with, Some(b));

    // THE REJECTED VALUE NOT YET DEAD: REMEMBERED.
    let mut sending = Watch {
        sent_with: Some(a),
        ..Watch::default()
    };
    assert!(matches!(
        reread(&mut sending, "invented-token-b").await,
        Reread::Rotated(_)
    ));
    assert_eq!(sending.dead, [a]);

    // THE SAME VALUE BACK, AND ONE REJECTED EARLIER.
    assert!(matches!(
        reread(&mut sending, "invented-token-b").await,
        Reread::Halted(CredentialStop::SameValue, _)
    ));
    let mut earlier = Watch {
        sent_with: Some(b),
        dead: vec![a],
        stop: None,
    };
    let Reread::Halted(CredentialStop::SameValue, why) =
        reread(&mut earlier, "invented-token-a").await
    else {
        panic!("a value rejected earlier is never sent again")
    };
    assert!(why.contains("already rejected in this run"), "{why}");

    // A RE-READ THAT FAILS, AN EMPTY SCRIPT, A SCRIPT THAT RAN OUT.
    let mut failing = Watch {
        sent_with: Some(a),
        ..Watch::default()
    };
    let failed = failing
        .reread(
            &credentials(vec![Err(Unreadable::transport(
                "the parameter store was not reached".to_owned(),
            ))]),
            FEED,
        )
        .await;
    assert!(matches!(
        failed,
        Reread::Halted(CredentialStop::Unreadable(Unread::Transport), _)
    ));
    let empty = credentials(Vec::new())
        .source(FEED)
        .await
        .expect_err("an empty script answers nothing");
    assert_eq!(empty.class, Unread::Configuration);
    assert_eq!(empty.why, "the script is empty");
    let once = credentials(vec![Ok("invented-token-a".to_owned())]);
    for _ in 0..2 {
        let (source, _) = once.source(FEED).await.expect("the last answer again");
        assert_eq!(source.credential_print(), a);
    }

    // AT ONCE: NOTHING SENT, AND A DEAD VALUE BACK; THE REJECTED VALUE DEAD,
    // AND ANOTHER DEAD VALUE BACK.
    let mut unsent = Watch {
        dead: vec![a],
        ..Watch::default()
    };
    assert!(matches!(
        reread(&mut unsent, "invented-token-a").await,
        Reread::Halted(CredentialStop::SameValue, _)
    ));
    assert_eq!(unsent.sent_with, None, "nothing was admitted");
    let mut both = Watch {
        sent_with: Some(a),
        dead: vec![a, b],
        stop: None,
    };
    assert!(matches!(
        reread(&mut both, "invented-token-b").await,
        Reread::Halted(CredentialStop::SameValue, _)
    ));
    assert_eq!(both.dead, [a, b], "unchanged");

    // OUT OF THE BOX: ONE BYTE APART.
    let near = print_of("invented-token-c").await;
    let mut close = Watch {
        sent_with: Some(print_of("invented-token-d").await),
        ..Watch::default()
    };
    assert!(matches!(
        reread(&mut close, "invented-token-c").await,
        Reread::Rotated(_)
    ));
    assert_eq!(close.sent_with, Some(near));
    assert_eq!(close.dead.len(), 1);
}
