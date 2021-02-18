// The main function used with lambda

#[cfg(not(feature = "with-lambda"))]
use clap::{App, Arg};

#[cfg(feature = "with-lambda")]
use lambda_http::{lambda, IntoResponse};

#[cfg(feature = "with-lambda")]
fn main() {
    fn lambda_wrapper(
        request: lambda_http::Request,
        _context: lambda_runtime::Context,
    ) -> Result<impl IntoResponse, lambda_runtime::error::HandlerError> {
        let response_builder = simple_server::ResponseBuilder::new();
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let resp = rt.block_on(handle(
            request.map(|b| b.as_ref().to_vec()),
            response_builder,
            "",
            "https://gw.dfinity.network",
        ));
        resp.or_else(|e| {
            println!("Error: {}", e);
            Ok(simple_server::ResponseBuilder::new()
                .status(500)
                .body(format!("Error:\n{}", e).as_bytes().to_vec())
                .unwrap())
        })
    }
    lambda!(lambda_wrapper)
}

// The main function used with simple_server

#[cfg(not(feature = "with-lambda"))]
fn main() {
    // env_logger::init().unwrap();

    let matches = App::new("ic-http-lambda")
        .args(&[Arg::new("force-canister-id")
            .about("Sets the canister id to use instead of parsing from the url.")
            .takes_value(true)
            .short('c')
            .long("force-canister-id")
            .default_value("")])
        .args(&[Arg::new("replica-url")
            .about("Sets the url for the running replica to forward requests to.")
            .takes_value(true)
            .short('r')
            .long("replica-url")
            .default_value("https://gw.dfinity.network")])
        .get_matches();
    let force_canister_id = matches.value_of("force-canister-id").unwrap().to_string();
    let replica_url = matches.value_of("replica-url").unwrap().to_string();

    let host = "127.0.0.1";
    let port = "7878";

    let server = simple_server::Server::new(move |request, response| {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(handle(request, response, &force_canister_id, &replica_url))
            .or_else(|e| {
                Ok(simple_server::ResponseBuilder::new()
                    .body(format!("{}", e).as_bytes().to_vec())
                    .unwrap())
            })
    });

    println!("Running on http://{}:{}/", host, port);

    server.listen(host, port);
}

// Common code

// The candid interface

use candid::{CandidType, Decode, Encode};
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
struct HTTPRequest {
    method: String,
    headers: Vec<(Vec<u8>, Vec<u8>)>,
    uri: String,
    body: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
struct HTTPResult {
    status: u16,
    headers: Vec<(Vec<u8>, Vec<u8>)>,
    body: Vec<u8>,
    upgrade: bool,
}

// The handler

async fn handle(
    request: http::Request<Vec<u8>>,
    mut response: simple_server::ResponseBuilder,
    force_canister_id: &str,
    replica_url: &str,
) -> Result<http::Response<Vec<u8>>, Box<dyn Send + Sync + std::error::Error>> {
    println!("Uri: {}", request.uri());
    println!("Request: {:?}", String::from_utf8_lossy(request.body()));

    let cid: ic_types::Principal;
    if !force_canister_id.is_empty() {
        cid = ic_types::Principal::from_text(force_canister_id).unwrap();
    } else {
        cid = match request
            .uri()
            .host()
            .and_then(|h| h.strip_suffix(".ic.nomeata.de").map(|x| x.to_owned()))
            .and_then(|c| ic_types::Principal::from_text(c).ok())
        {
            Some(c) => c,
            None => {
                return Err(
                    format!("Use https://<cid>ic.nomeata.de/!\n(got: {})", request.uri()).into(),
                )
            }
        };
    }

    let agent = ic_agent::Agent::builder()
        .with_url(replica_url)
        .build()
        .map_err(|e| Box::new(e))?;
    let req = HTTPRequest {
        method: request.method().to_string(),
        headers: request
            .headers()
            .iter()
            .map(|(h, v)| (h.as_str().into(), v.as_bytes().into()))
            .collect(),
        uri: request
            .uri()
            .path_and_query()
            .map_or(",".to_string(), |x| x.to_string()),
        body: request.body().to_vec(),
    };

    let result_blob = agent
        .query(&cid, "http_query")
        .with_arg(&Encode!(&req)?)
        .call()
        .await?;

    let result = Decode!(result_blob.as_slice(), HTTPResult)?;
    println!(
        "Response (query, upgrade = {}): {:?}",
        result.upgrade,
        String::from_utf8_lossy(&result.body)
    );

    let result = if result.upgrade {
        // Re-do the request as an update call
        agent.fetch_root_key().await?;
        let waiter = delay::Delay::builder()
            .throttle(std::time::Duration::from_millis(500))
            .timeout(std::time::Duration::from_secs(45))
            .build();

        let result_blob = agent
            .update(&cid, "http_update")
            .with_arg(&Encode!(&req)?)
            .call_and_wait(waiter)
            .await?;
        let result = Decode!(result_blob.as_slice(), HTTPResult)?;
        println!(
            "Response (update): {:?}",
            String::from_utf8_lossy(&result.body)
        );
        result
    } else {
        result
    };

    let mut res = response.status(result.status);
    for (h, v) in result.headers.iter() {
        res = res.header(
            http::header::HeaderName::from_bytes(h)?,
            http::header::HeaderValue::from_bytes(v)?,
        )
    }
    return Ok(res.body(result.body)?);
}
