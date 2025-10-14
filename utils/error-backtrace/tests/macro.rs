#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    #[backtrace(new)]
    NonBacktraceable(NonBacktraceableError),

    #[error("hi {x}")]
    Named { x: u32 },

    #[error(transparent)]
    Unnamed(InnerError),

    #[error("unit")]
    Unit,
}

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
enum InnerError {
    #[error("my error: {x}")]
    #[backtrace(private_constructor)]
    MyError { x: u32, y: u64 },
}

#[derive(Debug, thiserror::Error)]
enum NonBacktraceableError {
    #[error("my error: {x}")]
    MyError { x: u32 },
}

#[test]
fn test_macro() {
    let inner = NonBacktraceableError::MyError { x: 1 };

    let _: Error = inner.into();
    InnerError::_my_error(1, 2);
}
