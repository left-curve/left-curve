use {
    actix_web::{HttpRequest, http::header::HeaderMap},
    async_graphql::SimpleObject,
    grug_types::HttpRequestDetails,
    serde::Serialize,
    std::net::{IpAddr, SocketAddr},
};

#[derive(Clone, Debug, Serialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct RequesterIp {
    pub remote_ip: Option<String>,
    pub peer_ip: Option<String>,
    pub x_forwarded_for: Option<String>,
    pub forwarded: Option<String>,
    pub cf_connecting_ip: Option<String>,
    pub true_client_ip: Option<String>,
    pub x_real_ip: Option<String>,
}

impl RequesterIp {
    pub(crate) fn from_request(req: &HttpRequest) -> Self {
        let x_forwarded_for = header_value(req.headers(), "x-forwarded-for").map(ToOwned::to_owned);
        let forwarded = header_value(req.headers(), "forwarded").map(ToOwned::to_owned);
        let cf_connecting_ip =
            header_value(req.headers(), "cf-connecting-ip").map(ToOwned::to_owned);
        let true_client_ip = header_value(req.headers(), "true-client-ip").map(ToOwned::to_owned);
        let x_real_ip = header_value(req.headers(), "x-real-ip").map(ToOwned::to_owned);

        let real_ip = req
            .connection_info()
            .realip_remote_addr()
            .and_then(parse_ip_candidate);

        let remote_ip = if real_ip.as_ref().is_some_and(|ip| !is_proxy_hop_ip(ip)) {
            real_ip
        } else {
            original_client_ip_from_headers(req.headers())
                .or(real_ip)
                .or_else(|| {
                    req.connection_info()
                        .peer_addr()
                        .and_then(parse_ip_candidate)
                })
        };

        Self {
            remote_ip,
            peer_ip: req
                .connection_info()
                .peer_addr()
                .and_then(parse_ip_candidate),
            x_forwarded_for,
            forwarded,
            cf_connecting_ip,
            true_client_ip,
            x_real_ip,
        }
    }

    pub(crate) fn into_http_request_details(self) -> HttpRequestDetails {
        HttpRequestDetails::new(self.remote_ip, self.peer_ip)
    }
}

fn original_client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    [
        header_value(headers, "x-forwarded-for").and_then(parse_ip_list_header),
        header_value(headers, "forwarded").and_then(parse_forwarded_header),
        header_value(headers, "cf-connecting-ip").and_then(parse_ip_candidate),
        header_value(headers, "true-client-ip").and_then(parse_ip_candidate),
        header_value(headers, "x-real-ip").and_then(parse_ip_candidate),
    ]
    .into_iter()
    .flatten()
    .next()
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn parse_ip_list_header(value: &str) -> Option<String> {
    value.split(',').find_map(parse_ip_candidate)
}

fn parse_forwarded_header(value: &str) -> Option<String> {
    value
        .split(',')
        .flat_map(|segment| segment.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, raw_value)| {
            if key.trim().eq_ignore_ascii_case("for") {
                parse_ip_candidate(raw_value)
            } else {
                None
            }
        })
}

fn parse_ip_candidate(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"');

    if value.is_empty() || value.eq_ignore_ascii_case("unknown") {
        return None;
    }

    let bracketless = value
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']').map(|(ip, _)| ip))
        .unwrap_or(value);

    bracketless
        .parse::<IpAddr>()
        .map(|ip| ip.to_string())
        .ok()
        .or_else(|| {
            bracketless
                .parse::<SocketAddr>()
                .map(|addr| addr.ip().to_string())
                .ok()
        })
}

fn is_proxy_hop_ip(value: &str) -> bool {
    let Some(ip) = parse_ip_candidate(value).and_then(|ip| ip.parse::<IpAddr>().ok()) else {
        return false;
    };

    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_unspecified()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        http::header::{HeaderName, HeaderValue},
        test::TestRequest,
    };

    #[test]
    fn parses_forwarded_for_lists() {
        assert_eq!(
            parse_ip_list_header("198.51.100.10, 172.22.0.2"),
            Some("198.51.100.10".to_string())
        );
    }

    #[test]
    fn parses_forwarded_header() {
        assert_eq!(
            parse_forwarded_header("for=198.51.100.10;proto=https, for=172.22.0.2"),
            Some("198.51.100.10".to_string())
        );
    }

    #[test]
    fn parses_cf_connecting_ip_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("cf-connecting-ip"),
            HeaderValue::from_static("203.0.113.9"),
        );

        assert_eq!(
            original_client_ip_from_headers(&headers),
            Some("203.0.113.9".to_string())
        );
    }

    #[test]
    fn treats_private_addresses_as_proxy_hops() {
        assert!(is_proxy_hop_ip("172.22.0.2"));
        assert!(is_proxy_hop_ip("127.0.0.1"));
        assert!(!is_proxy_hop_ip("203.0.113.9"));
    }

    #[test]
    fn requester_ip_prefers_forwarded_headers_when_real_ip_is_proxy_hop() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "198.51.100.10, 172.22.0.2"))
            .peer_addr("172.22.0.2:1234".parse().unwrap())
            .to_http_request();

        let requester_ip = RequesterIp::from_request(&req);

        assert_eq!(requester_ip.remote_ip, Some("198.51.100.10".to_string()));
        assert_eq!(requester_ip.peer_ip, Some("172.22.0.2".to_string()));
    }
}
