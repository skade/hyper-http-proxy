extern crate futures;
extern crate hyper;

use futures::future;
use hyper::rt::{Future, Stream};
use hyper::service::{service_fn, Service};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::client;

/// We need to return different futures depending on the route matched,
/// and we can do that with an enum, such as `futures::Either`, or with
/// trait objects.
///
/// A boxed Future (trait object) is used as it is easier to understand
/// and extend with more types. Advanced users could switch to `Either`.
type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct Proxy;

impl Service for Proxy {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = BoxFut;
    /// This is our service handler. It receives a Request, routes on its
    /// path, and returns a Future of a Response.
    fn call(&mut self, req: Request<Body>) -> BoxFut {
        let (parts, body) = req.into_parts();

        match (parts.method, parts.uri.path()) {
            // Serve some instructions at /
            (Method::GET, "/") => {
                let mut response = Response::new(Body::empty());

                *response.body_mut() = Body::from("Try POSTing data to /echo");
                Box::new(future::ok(response))
            }

            // Reverse the entire body before sending back to the client.
            //
            // Since we don't know the end yet, we can't simply stream
            // the chunks as they arrive. So, this returns a different
            // future, waiting on concatenating the full body, so that
            // it can be reversed. Only then can we return a `Response`.
            (Method::POST, _) => {
                let url = format!("http://httpbin.org/post");
                let client = client::Client::new();

                let mut request_builder = hyper::Request::post(url);
                let length = parts.headers.get("Content-Length").unwrap().clone();

                request_builder.header("Content-Length", length);

                let request = request_builder.body(body).unwrap();

                let f = client.request(request);

                Box::new(f)
            }

            // The 404 Not Found route...
            _ => {
                let mut response = Response::new(Body::empty());

                *response.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(response))
            }
        }

    }
}

fn spawn_service() -> Result<Proxy, hyper::Error> {
    Ok(Proxy)
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| spawn_service() )
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);
    hyper::rt::run(server);
}
