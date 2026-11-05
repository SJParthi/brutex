//! The two rows of the single-stop reader its page tests leave: a pinned ask under a second store root while the
//! first root's reader is cached, and a page kind no query can name, refused by the projection itself.
use super::*;

/// Two saved catalogs of the same bytes under two roots. An unpinned read caches the first root's reader and the
/// first root is then removed: a pinned ask under the second root must compare the roots, reopen there and answer,
/// where a reader kept from the first root would refuse. A parsed ask then given a page kind no query can name is
/// refused by the projection by name, after its pin and its setting pass.
#[test]
fn a_second_root_reopens_and_a_page_kind_no_query_names_is_refused_by_name() -> Result<(), String> {
    projection_tests::with_saved_stop_catalog(|first, id| {
        projection_tests::with_saved_stop_catalog(|second, same| {
            assert_eq!(id, same, "both catalogs save one identity");
            let base = format!("identity={}", hex(id));
            let opened = render(first, &Asked::parse(&base)?)?;
            let pin = opened["completion"]
                .as_str()
                .ok_or("the first read names its completion")?;
            std::fs::remove_dir_all(first).map_err(|why| why.to_string())?;
            let pinned = format!("{base}&completion={pin}");
            let moved = render(second, &Asked::parse(&pinned)?)?;
            assert_eq!(
                (&moved["completion"], &moved["total"]),
                (&opened["completion"], &json!("2")),
                "the second root answers its own saved catalog"
            );
            let mut unnamed = Asked::parse(&format!("{pinned}&kind=trades&setting=1"))?;
            unnamed.kind = "grid".into();
            assert_eq!(
                render(second, &unnamed),
                Err("single-stop unsupported page".into())
            );
            Ok(())
        })
    })
}
