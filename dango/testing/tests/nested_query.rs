use {
    dango_testing::setup_test_naive,
    grug_types::{Querier, Query, ResultExt},
};

/// Security regression: a deeply nested `Query::Multi` must not overflow the
/// node's stack.
///
/// `grug_app::process_query` recurses into every child of a `Query::Multi`
/// without a depth limit, so a query shaped like `Multi([Multi([Multi([...])])])`
/// used to abort the whole process via stack overflow. The overflow is not a
/// catchable panic — there is no `catch_unwind` on the query path — so the node
/// process drops dead and does not recover. The fix rejects any `Multi` child
/// that itself contains a `Multi`, capping nesting at two levels and returning
/// `AppError::NestedMultiQuery` ("multi query can't be nested") instead of
/// recursing.
///
/// The query runs on a worker thread with a fixed, pinned stack so the behavior
/// is deterministic regardless of `RUST_MIN_STACK` or the ambient libtest/tokio
/// thread stack size. Measured at this 1 MiB stack (debug build):
///
/// - Fix REMOVED: `process_query` overflows at ~70 levels of nesting (each level
///   is a ~15 KiB stack frame); the process aborts with SIGABRT (exit 134).
/// - Fix in place: the guard short-circuits before recursing, so the only deep
///   stack use is dropping the nested `Query` value (~220 B per level), which
///   overflows only beyond ~4500 levels.
///
/// `DEPTH = 1000` therefore sits ~14x above the unpatched crash depth (so
/// removing the fix reliably reproduces the DoS) and ~4.5x below the drop limit
/// (so the patched path always passes).
///
/// cargo test --package dango-testing --test nested_query --all-features -- nested_multi_query_is_rejected --exact --show-output --nocapture
#[test]
fn nested_multi_query_is_rejected() {
    const STACK_SIZE: usize = 1024 * 1024;
    const DEPTH: usize = 1000;

    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(|| {
            let (suite, ..) = setup_test_naive(Default::default());

            // Build `Multi([Multi([ ... Status ... ])])`, `DEPTH` levels deep.
            let mut query = Query::status();
            for _ in 0..DEPTH {
                query = Query::Multi(vec![query]);
            }

            // The outer Multi returns Ok; its single child carries the embedded
            // rejection because it is a `Multi` that contains a `Multi`.
            let response = suite.query_chain(query).should_succeed();
            response
                .into_multi()
                .into_iter()
                .next()
                .expect("multi response must contain one child result")
                .should_fail_with_error("multi query can't be nested");
        })
        .expect("failed to spawn worker thread")
        .join()
        .expect("worker thread panicked");
}
