//! HTTP-boundary tests for capped font downloads.

use super::{fetch_font_data_from, to_lower_hex};
use rstest::rstest;
use sha2::{Digest, Sha256};
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::thread;

fn serve_http_body(body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local HTTP listener");
    let address = listener.local_addr().expect("read listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept HTTP request");
        let mut request = [0; 1024];
        stream.read(&mut request).expect("read HTTP request");
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write HTTP response headers");
        stream.write_all(&body).expect("write HTTP response body");
    });
    (format!("http://{address}/font.ttf"), server)
}

#[rstest]
fn fetch_font_data_accepts_an_exact_limit_http_response() {
    let body = b"font".to_vec();
    let expected_sha256 = to_lower_hex(&Sha256::digest(&body));
    let (url, server) = serve_http_body(body.clone());

    let result = fetch_font_data_from(&url, &expected_sha256, body.len() as u64);
    server.join().expect("HTTP server completed");

    assert_eq!(result.expect("response at the cap is accepted"), body);
}

#[rstest]
fn fetch_font_data_rejects_an_oversized_http_response_before_checksum() {
    let (url, server) = serve_http_body(b"font!".to_vec());

    let result = fetch_font_data_from(&url, "unused checksum", 4);
    server.join().expect("HTTP server completed");

    let error = result.expect_err("response above the cap is rejected");
    assert!(
        format!("{error:#}").contains("body exceeds the maximum of 4 bytes"),
        "unexpected error: {error:#}"
    );
}
