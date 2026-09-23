fn main() {
    let full = vocab::ConditionMask::ZERO.with_bit(0).with_bit(63);
    let bars = [full, vocab::ConditionMask::ZERO];
    let result = std::panic::catch_unwind(|| {
        engine::Ladder::with_min_hits(1)
            .with_support_lanes(usize::MAX)
            .walk(&bars, &[0, 63])
    });
    assert!(result.is_ok(), "maximum worker request must not panic");
    let normal = engine::Ladder::with_min_hits(1)
        .with_support_lanes(1)
        .walk(&bars, &[0, 63]);
    assert_eq!(
        result.unwrap(),
        normal,
        "scheduling cannot change the answer"
    );
    println!("two bars, two live bits, usize::MAX lanes: no panic; same result as one worker");
}
