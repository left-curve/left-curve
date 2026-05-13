"""Verify the dango.utils.error class hierarchy."""

from dango.utils.error import (
    ClientError,
    Error,
    GraphQLError,
    ServerError,
    TxFailed,
)


def test_all_subclass_error() -> None:
    for cls in (ClientError, ServerError, GraphQLError, TxFailed):
        assert issubclass(cls, Error)


def test_error_subclasses_exception() -> None:
    assert issubclass(Error, Exception)


def test_each_class_can_be_raised() -> None:
    for cls in (Error, ClientError, ServerError, GraphQLError, TxFailed):
        try:
            raise cls("oops")
        except Error as e:
            assert str(e) == "oops"
