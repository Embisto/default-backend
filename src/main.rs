#![allow(unused_variables)]
#![cfg_attr(feature="cargo-clippy", allow(needless_pass_by_value))]

extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate futures;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prometheus;

use std::{env};
use actix_web::{fs, server, App, HttpRequest, HttpResponse, Result};
use actix_web::http::{StatusCode};
use actix_web::middleware::{self};

use prometheus::{CounterVec, HistogramVec, TextEncoder, Encoder };

const NAMESPACE: &'static str = "default_http_backend";
const SUBSYSTEM: &'static str = "http";

lazy_static! {


    static ref REQUEST_COUNT: CounterVec = register_counter_vec!(
        opts!(
            "request_count_total",
            "Counter of HTTP requests made."
        )
        .subsystem(NAMESPACE)
        .namespace(SUBSYSTEM),
        &["proto"]
    ).unwrap();

    static ref REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        histogram_opts!(
            "request_duration_milliseconds",
            "Histogram of the time (in milliseconds) each request took.",
            vec![0.001, 0.003]
        )
        .subsystem(NAMESPACE)
        .namespace(SUBSYSTEM),
        &["proto"]
    ).unwrap();
}

/// notfound
fn notfound(req: HttpRequest) -> Result<HttpResponse> {

    let timer = REQUEST_DURATION.with_label_values(&[&format!("{:?}", req.version())]).start_timer();
    REQUEST_COUNT.with_label_values(&[&format!("{:?}", req.version())]).inc();
    timer.observe_duration();

    Ok(HttpResponse::build(StatusCode::NOT_FOUND)
       .content_type("text/html; charset=utf-8")
       .body(include_str!("../static/index.html")))
}

fn healthz(req: HttpRequest) -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
       .content_type("text/plain; charset=utf-8")
       .body("ok"))
}

fn metrics(req: HttpRequest) -> Result<HttpResponse> {

    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_familys = prometheus::gather();
    encoder.encode(&metric_familys, &mut buffer).unwrap();

    Ok(HttpResponse::build(StatusCode::OK)
       .content_type("text/plain; charset=utf-8")
       .body(String::from_utf8(buffer).unwrap()))
}


fn main() {
    env::set_var("RUST_LOG", "actix_web=debug");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
    let sys = actix::System::new("default-backend");

    let addr = server::new(
        || App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            // healthz
            .resource("/healthz", |r| r.f(healthz))
            // static files
            .handler("/static", fs::StaticFiles::new("static"))
            // metrics
            .resource("/metrics", |r| r.f(metrics))
            // root
            .resource("/", |r| r.f(notfound))
            )

        .bind("127.0.0.1:8080").expect("Can not bind to 127.0.0.1:8080")
        .shutdown_timeout(0)
        .start();

    println!("Starting http server: 127.0.0.1:8080");
    let _ = sys.run();
}
