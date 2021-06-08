use actix_web::{http::header::ContentType, HttpResponse, HttpResponseBuilder};
use log::error;
use serde::Serialize;

/// Allow to serialize in cbor on the `HttpResponseBuilder`.
pub trait HttpResponseBuilderExt {
    /// Set a cbor body and generate `Response`
    ///
    /// `ResponseBuilder` can not be used after this call.
    fn cbor<T: Serialize>(&mut self, value: T) -> HttpResponse;

    /// Set a cbor body and generate `Response`
    ///
    /// `ResponseBuilder` can not be used after this call.
    fn cbor2<T: Serialize>(&mut self, value: &T) -> HttpResponse;
}

impl HttpResponseBuilderExt for HttpResponseBuilder {
    fn cbor<T: Serialize>(&mut self, value: T) -> HttpResponse {
        self.cbor2(&value)
    }

    fn cbor2<T: Serialize>(&mut self, value: &T) -> HttpResponse {
        match serde_cbor::to_vec(value) {
            Ok(body) => {
                self.insert_header(ContentType("application/cbor".parse().unwrap()));
                self.body(actix_web::dev::Body::from(body)).into()
            }
            Err(e) => {
                error!("Serialize error: {}", e);
                HttpResponse::InternalServerError()
                    .reason("unable to serialize cbor.")
                    .finish()
            }
        }
    }
}
