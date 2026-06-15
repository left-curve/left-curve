use {
    dango_primitives::{Querier, Query, ResultExt},
    dango_testing::setup_test_naive,
};

/// Multi queries cannot be nested at all: a `Query::Multi` may only contain
/// non-multi children. A flat multi query is valid; a multi nested inside
/// another multi is rejected with `AppError::NestedMultiQuery` ("multi query
/// can't be nested"), no matter how shallow.
///
/// This pins the boundary precisely — the largest legal case (a flat `Multi`)
/// and the smallest illegal case (`Multi([Multi([..])])`).
///
/// cargo test --package dango-testing --test nested_query --all-features -- multi_query_rejects_any_nesting --exact --show-output --nocapture
#[test]
fn multi_query_rejects_any_nesting() {
    let (suite, ..) = setup_test_naive(Default::default());

    // A flat multi query — `Multi([Status])` — is valid: its child is not a
    // multi query, so it is evaluated normally.
    suite
        .query_chain(Query::Multi(vec![Query::status()]))
        .should_succeed()
        .into_multi()
        .into_iter()
        .next()
        .expect("multi response must contain one child result")
        .should_succeed();

    // A multi nested one level deep — `Multi([Multi([Status])])` — is rejected,
    // even though it is tiny: nesting is not allowed at any depth.
    suite
        .query_chain(Query::Multi(vec![Query::Multi(vec![Query::status()])]))
        .should_succeed()
        .into_multi()
        .into_iter()
        .next()
        .expect("multi response must contain one child result")
        .should_fail_with_error("multi query can't be nested");
}

/// Security regression: a deeply nested `Query::Multi` must not overflow the
/// node's stack.
///
/// `dango_app::process_query` recurses into every child of a `Query::Multi`
/// without a depth limit, so a query shaped like `Multi([Multi([Multi([...])])])`
/// used to abort the whole process via stack overflow. The overflow is not a
/// catchable panic — there is no `catch_unwind` on the query path — so the node
/// process drops dead and does not recover. The fix rejects any `Multi` nested
/// inside another `Multi` before recursing, so the recursion can never deepen.
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
/// cargo test --package dango-testing --test nested_query --all-features -- deeply_nested_multi_query_does_not_overflow --exact --show-output --nocapture
#[test]
fn deeply_nested_multi_query_does_not_overflow() {
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
            // rejection because it is itself a `Multi`.
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
