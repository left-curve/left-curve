use metrics::{describe_counter, describe_histogram};

pub fn init_httpd_metrics() {
    describe_counter!(
        "http.requests.total",
        "Total HTTP requests by method, path, and status"
    );
    describe_histogram!(
        "http.request.duration.seconds",
        "HTTP request duration in seconds by method, path, and status"
    );
}
