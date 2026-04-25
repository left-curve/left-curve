"""Exception classes raised by the SDK."""

from __future__ import annotations


class Error(Exception):
    """Base class for all SDK-raised exceptions."""


class ClientError(Error):
    """Raised on a 4xx HTTP response from the GraphQL endpoint."""


class ServerError(Error):
    """Raised on a 5xx HTTP response from the GraphQL endpoint."""


class GraphQLError(Error):
    """Raised when a GraphQL response carries a non-empty `errors` array."""


class TxFailed(Error):
    """Raised when broadcastTxSync returns an `err` result."""
