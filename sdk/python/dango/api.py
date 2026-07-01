"""GraphQL HTTP client; base class for Info and Exchange."""

from __future__ import annotations

from typing import Any, TypeVar, cast

import requests

from dango.utils.error import ClientError, GraphQLError, ServerError

T = TypeVar("T")


class API:
    """Sync GraphQL POST client. Reads `<base_url>/graphql`; raises on HTTP and GraphQL errors."""

    def __init__(self, base_url: str, *, timeout: float | None = None) -> None:
        self.base_url: str = base_url.rstrip("/")
        self.timeout: float | None = timeout
        self._session: requests.Session = requests.Session()
        self._session.headers["Content-Type"] = "application/json"

    def query(
        self,
        document: str,
        variables: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        """POST a GraphQL query/mutation; return the `data` field.

        Raises:
            ServerError: on network-level failures (connection refused, DNS failure,
                timeout, SSL issues, etc), 5xx HTTP responses, or non-JSON bodies.
            ClientError: on 4xx HTTP responses.
            GraphQLError: when the response carries an `errors` array, or is missing
                both `data` and `errors`.
        """

        url = f"{self.base_url}/graphql"
        body = {"query": document, "variables": variables or {}}

        try:
            response = self._session.post(url, json=body, timeout=self.timeout)
        except requests.RequestException as exc:
            # Wrap network-level failures (connection / DNS / timeout / SSL)
            # so callers never see a raw `requests` exception.
            raise ServerError(f"request to {self.base_url} failed: {exc}") from exc

        # Status branches must run before `response.json()`: a 4xx may carry a
        # GraphQL-shaped body, but we want it mapped to ClientError, not parsed
        # as a successful response.
        if 400 <= response.status_code < 500:
            raise ClientError(f"HTTP {response.status_code}: {response.text[:500]}")
        if response.status_code >= 500:
            raise ServerError(f"HTTP {response.status_code}: {response.text[:500]}")

        try:
            payload = response.json()
        except ValueError as exc:
            raise ServerError(f"non-JSON response: {response.text[:500]!r}") from exc

        # The GraphQL spec mandates a JSON object envelope; an array or scalar
        # is a server-side malformation, not a query-level error.
        if not isinstance(payload, dict):
            raise ServerError(f"GraphQL response was not a JSON object: {payload!r}")

        errors = payload.get("errors")
        if errors:
            raise GraphQLError(_format_graphql_errors(errors))

        # The GraphQL spec requires `data` or `errors` (or both) on every
        # response. If both are absent the envelope is malformed.
        data = payload.get("data")
        if data is None:
            raise GraphQLError(f"GraphQL response missing both `data` and `errors`: {payload!r}")

        return cast("dict[str, Any]", data)

    def query_typed(
        self,
        document: str,
        variables: dict[str, Any] | None = None,
        *,
        response_type: type[T],
    ) -> T:
        """Same as `query` but cast the result to `response_type`. No runtime validation."""

        return cast(T, self.query(document, variables))


def _format_graphql_errors(errors: list[dict[str, Any]]) -> str:
    parts = []
    for err in errors:
        msg = err.get("message", "<no message>")
        path = err.get("path")
        if path:
            parts.append(f"{msg} (path={path})")
        else:
            parts.append(msg)

    return "; ".join(parts) if parts else "GraphQL error"
