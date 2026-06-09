#!/usr/bin/env python3
import argparse
import datetime as dt
import json
import os
import signal
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


GRAPHQL_URL = "https://api.cloudflare.com/client/v4/graphql"
API_BASE_URL = "https://api.cloudflare.com/client/v4"
UTC = dt.timezone.utc
HTTP_EVENTS_FIELDS = """
  count
  dimensions {
    datetime
    clientIP
    clientRequestHTTPHost
    clientRequestHTTPMethodName
    clientRequestPath
    coloCode
    edgeResponseStatus
    originResponseStatus
    originIP
    cacheStatus
  }
  avg {
    edgeTimeToFirstByteMs
    originResponseDurationMs
  }
"""
LOAD_BALANCER_FIELDS = """
  datetime
  coloCode
  lbName
  selectedPoolName
  selectedPoolId
  selectedOriginIndex
  selectedPoolHealthy
  selectedPoolHealthChecksEnabled
  sessionAffinity
  sessionAffinityStatus
  steeringPolicy
  origins {
    originName
    fqdn
    health
    ipv4
    ipv6
    selected
  }
  pools {
    poolName
    healthy
    healthCheckEnabled
    avgRttMs
  }
"""


class CloudflareError(RuntimeError):
    pass


class Metrics:
    def __init__(self):
        self._lock = threading.Lock()
        self._values = {}

    def inc(self, name, labels=None, amount=1):
        key = (name, tuple(sorted((labels or {}).items())))
        with self._lock:
            self._values[key] = self._values.get(key, 0) + amount

    def set(self, name, value, labels=None):
        key = (name, tuple(sorted((labels or {}).items())))
        with self._lock:
            self._values[key] = value

    def render(self):
        with self._lock:
            lines = []
            for (name, labels), value in sorted(self._values.items()):
                label_text = ""
                if labels:
                    parts = [f'{k}="{escape_label(v)}"' for k, v in labels]
                    label_text = "{" + ",".join(parts) + "}"
                lines.append(f"{name}{label_text} {value}")
            return "\n".join(lines) + "\n"


def escape_label(value):
    return str(value).replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


def log(level, message, **fields):
    payload = {
        "level": level,
        "message": message,
        "timestamp": now().isoformat().replace("+00:00", "Z"),
        **fields,
    }
    print(json.dumps(payload, separators=(",", ":")), flush=True)


def now():
    return dt.datetime.now(UTC)


def parse_time(value):
    if not value:
        return None
    if value.endswith("Z"):
        value = value[:-1] + "+00:00"
    return dt.datetime.fromisoformat(value).astimezone(UTC)


def format_time(value):
    return value.astimezone(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def gql_string(value):
    return json.dumps(str(value))


class Observer:
    def __init__(self, config, token, metrics):
        self.config = config
        self.token = token
        self.metrics = metrics
        self.state_path = config.get("state_path", "/data/state.json")
        self.state = self.load_state()
        self.stop_event = threading.Event()

    def load_state(self):
        try:
            with open(self.state_path, "r", encoding="utf-8") as handle:
                state = json.load(handle)
                if not isinstance(state, dict):
                    raise ValueError("state root is not a JSON object")
                if not isinstance(state.get("seen"), dict):
                    state["seen"] = {}
                state.setdefault("zone_id", "")
                return state
        except FileNotFoundError:
            pass
        except Exception as exc:
            log("error", "failed to read state; quarantining", error=str(exc), path=self.state_path)
            self.quarantine_state()
        return {"seen": {}, "zone_id": ""}

    def quarantine_state(self):
        corrupt_path = f"{self.state_path}.corrupt.{int(time.time())}"
        try:
            os.replace(self.state_path, corrupt_path)
            log("warning", "quarantined unreadable state", path=self.state_path, corrupt_path=corrupt_path)
        except FileNotFoundError:
            pass
        except Exception as exc:
            log("warning", "failed to quarantine unreadable state", error=str(exc), path=self.state_path)

    def save_state(self):
        directory = os.path.dirname(self.state_path)
        if directory:
            os.makedirs(directory, exist_ok=True)
        tmp_path = self.state_path + ".tmp"
        with open(tmp_path, "w", encoding="utf-8") as handle:
            json.dump(self.state, handle, sort_keys=True)
        os.replace(tmp_path, self.state_path)

    def run(self):
        self.metrics.set("cloudflare_edge_observer_configured_targets", len(self.config.get("targets", [])))
        while not self.stop_event.is_set():
            started = time.time()
            try:
                target_errors = self.poll_once()
                if target_errors == 0:
                    self.metrics.set("cloudflare_edge_observer_last_success_timestamp_seconds", int(time.time()))
            except Exception as exc:
                self.metrics.inc("cloudflare_edge_observer_poll_errors_total")
                log("error", "poll failed", error=str(exc))

            interval = int(self.config.get("poll_interval_seconds", 60))
            elapsed = time.time() - started
            self.stop_event.wait(max(1, interval - elapsed))

    def poll_once(self):
        zone_id = self.zone_id()
        end = now()
        lookback = int(self.config.get("lookback_seconds", 300))
        start = end - dt.timedelta(seconds=lookback)
        all_events = []
        target_errors = 0

        for target in self.config.get("targets", []):
            labels = {
                "network": target.get("network", "unknown"),
                "service": target.get("service", "unknown"),
                "mode": target.get("mode", "unknown"),
            }
            try:
                rows = self.query_http_events(zone_id, target, start, end)
            except Exception as exc:
                target_errors += 1
                self.metrics.inc("cloudflare_edge_observer_target_poll_errors_total", labels)
                log("error", "target poll failed", target=target.get("host"), error=str(exc))
                continue

            self.metrics.set("cloudflare_edge_observer_last_target_rows", len(rows), labels)
            for row in rows:
                event = self.event_from_http_row(target, row)
                if not event:
                    continue
                if self.is_seen(event):
                    continue
                if target.get("mode") == "load_balanced":
                    event.update(self.correlate_load_balancer(zone_id, target, event))
                all_events.append(event)

        if all_events:
            self.push_loki(all_events)
            seen_at = int(time.time())
            for event in all_events:
                self.mark_seen(event, seen_at)
                event_labels = {
                    "network": event.get("network", "unknown"),
                    "service": event.get("service", "unknown"),
                    "edge_status": str(event.get("edge_status", "unknown")),
                    "colo": event.get("colo", "unknown"),
                }
                self.metrics.inc("cloudflare_edge_observer_events_total", event_labels)
            log("info", "pushed cloudflare events", count=len(all_events))

        self.prune_seen(lookback * 4)
        self.save_state()
        return target_errors

    def zone_id(self):
        configured = self.config.get("zone_id") or self.state.get("zone_id")
        if configured:
            return configured

        zone_name = self.config["zone_name"]
        query = urllib.parse.urlencode({"name": zone_name})
        data = self.cloudflare_rest("GET", f"/zones?{query}")
        result = data.get("result") or []
        if not result:
            raise CloudflareError(f"zone not found: {zone_name}")
        zone_id = result[0]["id"]
        self.state["zone_id"] = zone_id
        self.save_state()
        return zone_id

    def query_http_events(self, zone_id, target, start, end):
        query = self.http_query(zone_id, target, start, end, HTTP_EVENTS_FIELDS)
        return self.extract_zone_rows(self.cloudflare_graphql(query), "httpRequestsAdaptiveGroups")

    def http_query(self, zone_id, target, start, end, fields):
        status_min = int(self.config.get("status_min", 520))
        status_lt = int(self.config.get("status_lt", 600))
        return f"""
        {{
          viewer {{
            zones(filter: {{ zoneTag: {gql_string(zone_id)} }}) {{
              httpRequestsAdaptiveGroups(
                limit: 1000
                filter: {{
                  datetime_geq: {gql_string(format_time(start))}
                  datetime_lt: {gql_string(format_time(end))}
                  clientRequestHTTPHost: {gql_string(target["host"])}
                  edgeResponseStatus_geq: {status_min}
                  edgeResponseStatus_lt: {status_lt}
                  requestSource: "eyeball"
                }}
                orderBy: [datetime_ASC]
              ) {{
                {fields}
              }}
            }}
          }}
        }}
        """

    def event_from_http_row(self, target, row):
        dimensions = row.get("dimensions") or {}
        event_time = dimensions.get("datetime")
        edge_status = dimensions.get("edgeResponseStatus")
        host = dimensions.get("clientRequestHTTPHost") or target.get("host")
        colo = dimensions.get("coloCode") or "unknown"

        if not event_time or not edge_status:
            return None

        avg = row.get("avg") or {}
        return {
            "source": "cloudflare",
            "zone": self.config.get("zone_name"),
            "datetime": event_time,
            "ray_id": "",
            "ray_name": "",
            "network": target.get("network"),
            "service": target.get("service"),
            "mode": target.get("mode"),
            "host": host,
            "path": dimensions.get("clientRequestPath"),
            "method": dimensions.get("clientRequestHTTPMethodName"),
            "client_ip": dimensions.get("clientIP"),
            "colo": colo,
            "edge_status": edge_status,
            "origin_status": dimensions.get("originResponseStatus"),
            "origin_ip": dimensions.get("originIP"),
            "cache_status": dimensions.get("cacheStatus"),
            "edge_time_to_first_byte_ms": avg.get("edgeTimeToFirstByteMs"),
            "origin_response_duration_ms": avg.get("originResponseDurationMs"),
            "sample_count": row.get("count"),
            "expected_origins": target.get("expected_origins", []),
            "direct_hosts": target.get("direct_hosts", []),
        }

    def correlate_load_balancer(self, zone_id, target, event):
        try:
            rows = self.query_load_balancer(zone_id, target, event)
        except CloudflareError as exc:
            return {"lb_correlation": "error", "lb_correlation_error": str(exc)}
        return self.lb_event_fields(event, rows)

    def query_load_balancer(self, zone_id, target, event):
        event_time = parse_time(event.get("datetime"))
        if not event_time:
            return []
        start = event_time - dt.timedelta(seconds=5)
        end = event_time + dt.timedelta(seconds=5)
        lb_name = target.get("lb_name")
        if not lb_name:
            raise CloudflareError(f"load-balanced target is missing lb_name: {target.get('host')}")
        filter_parts = [
            f"datetime_geq: {gql_string(format_time(start))}",
            f"datetime_leq: {gql_string(format_time(end))}",
            f"lbName: {gql_string(lb_name)}",
        ]
        if event.get("colo") and event.get("colo") != "unknown":
            filter_parts.append(f"coloCode: {gql_string(event['colo'])}")

        query = f"""
        {{
          viewer {{
            zones(filter: {{ zoneTag: {gql_string(zone_id)} }}) {{
              loadBalancingRequestsAdaptive(
                limit: 20
                filter: {{ {" ".join(filter_parts)} }}
                orderBy: [datetime_DESC]
              ) {{
                {LOAD_BALANCER_FIELDS}
              }}
            }}
          }}
        }}
        """
        return self.extract_zone_rows(self.cloudflare_graphql(query), "loadBalancingRequestsAdaptive")

    def lb_event_fields(self, event, rows):
        if not rows:
            return {"lb_correlation": "none"}

        event_time = parse_time(event.get("datetime"))
        if event_time:
            rows = sorted(
                rows,
                key=lambda row: abs((parse_time(row.get("datetime")) or event_time) - event_time),
            )
        row = rows[0]
        selected_origin = None
        selected_index = row.get("selectedOriginIndex")
        origins = row.get("origins") or []
        # Some LB analytics rows expose only selectedOriginIndex plus origins[].
        if selected_origin is None and isinstance(selected_index, int) and 0 <= selected_index < len(origins):
            selected = origins[selected_index]
            selected_origin = selected.get("originName")
        if selected_origin is None:
            selected = next((origin for origin in origins if origin.get("selected") is True), None)
            if selected:
                selected_origin = selected.get("originName")

        return {
            "lb_correlation": "matched",
            "lb_datetime": row.get("datetime"),
            "lb_name": row.get("lbName"),
            "selected_pool": row.get("selectedPoolName"),
            "selected_pool_id": row.get("selectedPoolId"),
            "selected_origin": selected_origin,
            "selected_origin_index": selected_index,
            "selected_pool_healthy": row.get("selectedPoolHealthy"),
            "selected_pool_health_checks_enabled": row.get("selectedPoolHealthChecksEnabled"),
            "session_affinity": row.get("sessionAffinity"),
            "session_affinity_status": row.get("sessionAffinityStatus"),
            "steering_policy": row.get("steeringPolicy"),
            "lb_origins": origins,
            "lb_pools": row.get("pools") or [],
        }

    def extract_zone_rows(self, data, field):
        zones = (((data or {}).get("viewer") or {}).get("zones")) or []
        if not zones:
            return []
        return zones[0].get(field) or []

    def cloudflare_graphql(self, query):
        payload = self.http_json(
            GRAPHQL_URL,
            "POST",
            {"query": query},
            {
                "Authorization": f"Bearer {self.token}",
                "Content-Type": "application/json",
            },
        )
        if payload.get("errors"):
            raise CloudflareError(json.dumps(payload["errors"], separators=(",", ":")))
        return payload.get("data") or {}

    def cloudflare_rest(self, method, path):
        payload = self.http_json(
            API_BASE_URL + path,
            method,
            None,
            {"Authorization": f"Bearer {self.token}"},
        )
        if not payload.get("success", False):
            raise CloudflareError(json.dumps(payload.get("errors") or payload, separators=(",", ":")))
        return payload

    def http_json(self, url, method, body, headers):
        data = None
        if body is not None:
            data = json.dumps(body).encode("utf-8")
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                raw = resp.read().decode("utf-8")
                return json.loads(raw) if raw else {}
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode("utf-8", errors="replace")
            raise CloudflareError(f"HTTP {exc.code}: {detail}") from exc

    def push_loki(self, events):
        streams = {}
        for event in events:
            labels = {
                "job": "cloudflare-edge-observer",
                "source": "cloudflare",
                "zone": event.get("zone") or "unknown",
                "host": event.get("host") or "unknown",
                "network": event.get("network") or "unknown",
                "service": event.get("service") or "unknown",
                "mode": event.get("mode") or "unknown",
                "edge_status": str(event.get("edge_status") or "unknown"),
                "colo": event.get("colo") or "unknown",
            }
            if event.get("selected_origin"):
                labels["selected_origin"] = event["selected_origin"]
            key = tuple(sorted(labels.items()))
            streams.setdefault(key, {"stream": labels, "values": []})
            event_time = parse_time(event.get("datetime")) or now()
            line = json.dumps(event, sort_keys=True, separators=(",", ":"))
            streams[key]["values"].append([str(int(event_time.timestamp() * 1_000_000_000)), line])

        payload = {"streams": list(streams.values())}
        try:
            self.http_json(
                self.config["loki_url"],
                "POST",
                payload,
                {"Content-Type": "application/json"},
            )
        except Exception as exc:
            self.metrics.inc("cloudflare_edge_observer_loki_push_errors_total")
            raise RuntimeError(f"failed to push events to Loki: {exc}") from exc

    def seen_key(self, event):
        return "|".join(
            [
                str(event.get("ray_id") or ""),
                str(event.get("datetime") or ""),
                str(event.get("host") or ""),
                str(event.get("path") or ""),
                str(event.get("method") or ""),
                str(event.get("edge_status") or ""),
                str(event.get("origin_status") or ""),
                str(event.get("colo") or ""),
                str(event.get("client_ip") or ""),
                str(event.get("origin_ip") or ""),
                str(event.get("cache_status") or ""),
            ]
        )

    def is_seen(self, event):
        return self.seen_key(event) in self.state["seen"]

    def mark_seen(self, event, seen_at):
        self.state["seen"][self.seen_key(event)] = seen_at

    def prune_seen(self, max_age_seconds):
        cutoff = int(time.time()) - max_age_seconds
        self.state["seen"] = {key: value for key, value in self.state["seen"].items() if value >= cutoff}


def start_metrics_server(metrics, port):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/health":
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b"ok\n")
                return
            if self.path != "/metrics":
                self.send_response(404)
                self.end_headers()
                return

            payload = metrics.render().encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; version=0.0.4")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, _format, *_args):
            return

    server = ThreadingHTTPServer(("0.0.0.0", port), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", required=True)
    args = parser.parse_args()

    with open(args.config, "r", encoding="utf-8") as handle:
        config = json.load(handle)

    token = os.environ.get("CF_API_TOKEN", "")
    if not token:
        print("CF_API_TOKEN is required", file=sys.stderr)
        return 2

    metrics = Metrics()
    observer = Observer(config, token, metrics)
    server = start_metrics_server(metrics, int(config.get("metrics_port", 9210)))

    def stop(_signum, _frame):
        observer.stop_event.set()
        server.shutdown()

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)

    log("info", "cloudflare edge observer started", targets=len(config.get("targets", [])))
    observer.run()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
