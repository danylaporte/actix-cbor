//! # Example
//! ```
//! use actix_cbor::Cbor;
//! use actix_web::get;
//!
//! #[derive(serde::Deserialize)]
//! struct User {
//!     name: String,
//! }
//!
//! #[derive(serde::Serialize)]
//! struct Greeting {
//!     inner: String,
//! }
//!
//! #[get("/users/hello")]
//! pub async fn greet_user(user: Cbor<User>) -> Cbor<Greeting> {
//!     let name: &str = &user.name;
//!     let inner: String = format!("Hello {}!", name);
//!     Cbor(Greeting { inner })
//! }
//! ```

#[cfg(test)]
#[macro_use]
extern crate serde;

use std::fmt;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "compress")]
use actix_web::dev::Decompress;
use actix_web::{
    dev::Payload, http::StatusCode, FromRequest, HttpRequest, HttpResponse, Responder,
};
use futures_util::future::LocalBoxFuture;
use futures_util::FutureExt;
use log::error;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub use body::*;
pub use config::*;
pub use error::*;
pub use http_response_builder_ext::*;

mod body;
mod config;
mod error;
mod http_response_builder_ext;

#[cfg(test)]
mod tests;

/// Extractor/Responder for CBOR encoded data.
///
/// This will encode data with the content-type `application/cbor`.
///
/// By default, it expects to receive data with that content-type as well.
///
/// # Example
/// ```
/// use actix_cbor::Cbor;
/// use actix_web::get;
///
/// #[derive(serde::Deserialize)]
/// struct User {
///     name: String,
/// }
///
/// #[derive(serde::Serialize)]
/// struct Greeting {
///     inner: String,
/// }
///
/// #[get("/users/hello")]
/// pub async fn greet_user(user: Cbor<User>) -> Cbor<Greeting> {
///     let name: &str = &user.name;
///     let inner: String = format!("Hello {}!", name);
///     Cbor(Greeting { inner })
/// }
/// ```
#[derive(Default, Clone)]
pub struct Cbor<T>(pub T);

impl<T> Cbor<T> {
    /// Deconstruct to an inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Cbor<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Cbor<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> fmt::Debug for Cbor<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cbor: {:?}", self.0)
    }
}

impl<T> Responder for Cbor<T>
where
    T: Serialize,
{
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        match serde_cbor::to_vec(&self.0) {
            Ok(body) => HttpResponse::build(StatusCode::OK)
                .content_type("application/cbor")
                .body(body),
            Err(e) => {
                error!("cbor serialization error: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

impl<T> FromRequest for Cbor<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = CborConfig;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let config = CborConfig::from_req(req);

        let limit = config.limit;
        let ctype = config.content_type.clone();
        let err_handler = config.err_handler.clone();

        CborBody::new(req, payload, ctype)
            .limit(limit)
            .map(move |res| match res {
                Err(e) => {
                    log::debug!(
                        "Failed to deserialize CBOR from payload. \
                         Request path: {}",
                        req2.path()
                    );

                    if let Some(err) = err_handler {
                        Err((*err)(e, &req2))
                    } else {
                        Err(e.into())
                    }
                }
                Ok(data) => Ok(Cbor(data)),
            })
            .boxed_local()
    }
}
