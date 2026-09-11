#![forbid(unsafe_code)]

//! Provides common error types and associated convenience methods for
//! TinyChain.
//!
//! This crate is a part of TinyChain: [http://github.com/haydnv/tinychain](http://github.com/haydnv/tinychain)

use std::convert::Infallible;
use std::str::FromStr;
use std::{fmt, io};

use destream::{de, en};

/// A result of type `T`, or a [`TCError`]
pub type TCResult<T> = Result<T, TCError>;

#[derive(Clone)]
struct ErrorData {
    message: String,
    stack: Vec<String>,
    pressure: Option<Pressure>,
}

/// Why a resource owner denied or delayed admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PressureReason {
    AllocationFailed,
    QuotaExceeded,
    Saturated,
    StorageFull,
}

impl PressureReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::AllocationFailed => "allocation_failed",
            Self::QuotaExceeded => "quota_exceeded",
            Self::Saturated => "saturated",
            Self::StorageFull => "storage_full",
        }
    }
}

impl FromStr for PressureReason {
    type Err = TCError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "allocation_failed" => Ok(Self::AllocationFailed),
            "quota_exceeded" => Ok(Self::QuotaExceeded),
            "saturated" => Ok(Self::Saturated),
            "storage_full" => Ok(Self::StorageFull),
            _ => Err(TCError::bad_request(format!(
                "unrecognized pressure reason: {value}"
            ))),
        }
    }
}

impl fmt::Display for PressureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl de::FromStream for PressureReason {
    type Context = ();

    async fn from_stream<D: de::Decoder>(_: (), decoder: &mut D) -> Result<Self, D::Error> {
        let value = String::from_stream((), decoder).await?;
        value
            .parse()
            .map_err(|_| de::Error::invalid_value(value, "a pressure reason"))
    }
}

impl<'en> en::IntoStream<'en> for PressureReason {
    fn into_stream<E: en::Encoder<'en>>(self, encoder: E) -> Result<E::Ok, E::Error> {
        self.as_str().into_stream(encoder)
    }
}

impl<'en> en::ToStream<'en> for PressureReason {
    fn to_stream<E: en::Encoder<'en>>(&'en self, encoder: E) -> Result<E::Ok, E::Error> {
        encoder.encode_str(self.as_str())
    }
}

/// Whether overload handling may drop work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reliability {
    BestEffort,
    Lossless,
}

impl Reliability {
    fn as_str(self) -> &'static str {
        match self {
            Self::BestEffort => "best_effort",
            Self::Lossless => "lossless",
        }
    }
}

impl FromStr for Reliability {
    type Err = TCError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "best_effort" => Ok(Self::BestEffort),
            "lossless" => Ok(Self::Lossless),
            _ => Err(TCError::bad_request(format!(
                "unrecognized reliability: {value}"
            ))),
        }
    }
}

impl fmt::Display for Reliability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl de::FromStream for Reliability {
    type Context = ();

    async fn from_stream<D: de::Decoder>(_: (), decoder: &mut D) -> Result<Self, D::Error> {
        let value = String::from_stream((), decoder).await?;
        value
            .parse()
            .map_err(|_| de::Error::invalid_value(value, "a reliability class"))
    }
}

impl<'en> en::IntoStream<'en> for Reliability {
    fn into_stream<E: en::Encoder<'en>>(self, encoder: E) -> Result<E::Ok, E::Error> {
        self.as_str().into_stream(encoder)
    }
}

impl<'en> en::ToStream<'en> for Reliability {
    fn to_stream<E: en::Encoder<'en>>(&'en self, encoder: E) -> Result<E::Ok, E::Error> {
        encoder.encode_str(self.as_str())
    }
}

/// Machine-readable overload metadata carried by [`TCError`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pressure {
    resource: String,
    reason: PressureReason,
    retry_after_ms: Option<u64>,
    reliability: Reliability,
}

impl Pressure {
    pub fn new(resource: impl Into<String>, reason: PressureReason) -> Self {
        Self {
            resource: resource.into(),
            reason,
            retry_after_ms: None,
            reliability: Reliability::Lossless,
        }
    }

    pub fn with_retry_after_ms(mut self, retry_after_ms: u64) -> Self {
        self.retry_after_ms = Some(retry_after_ms);
        self
    }

    pub fn with_reliability(mut self, reliability: Reliability) -> Self {
        self.reliability = reliability;
        self
    }

    pub fn resource(&self) -> &str {
        &self.resource
    }

    pub fn reason(&self) -> PressureReason {
        self.reason
    }

    pub fn retry_after_ms(&self) -> Option<u64> {
        self.retry_after_ms
    }

    pub fn reliability(&self) -> Reliability {
        self.reliability
    }
}

struct PressureVisitor;

impl de::Visitor for PressureVisitor {
    type Value = Pressure;

    fn expecting() -> &'static str {
        "structured resource pressure metadata"
    }

    async fn visit_map<A: de::MapAccess>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut resource = None;
        let mut reason = None;
        let mut retry_after_ms = None;
        let mut reliability = None;
        while let Some(key) = access.next_key::<String>(()).await? {
            match key.as_str() {
                "resource" => resource = Some(access.next_value::<String>(()).await?),
                "reason" => reason = Some(access.next_value::<PressureReason>(()).await?),
                "retry_after_ms" => retry_after_ms = access.next_value::<Option<u64>>(()).await?,
                "reliability" => reliability = Some(access.next_value::<Reliability>(()).await?),
                _ => return Err(de::Error::custom(format!("unknown pressure field {key}"))),
            }
        }

        Ok(Pressure {
            resource: resource.ok_or_else(|| de::Error::custom("missing pressure resource"))?,
            reason: reason.ok_or_else(|| de::Error::custom("missing pressure reason"))?,
            retry_after_ms,
            reliability: reliability
                .ok_or_else(|| de::Error::custom("missing pressure reliability"))?,
        })
    }
}

impl de::FromStream for Pressure {
    type Context = ();

    async fn from_stream<D: de::Decoder>(_: (), decoder: &mut D) -> Result<Self, D::Error> {
        decoder.decode_map(PressureVisitor).await
    }
}

impl<'en> en::IntoStream<'en> for Pressure {
    fn into_stream<E: en::Encoder<'en>>(self, encoder: E) -> Result<E::Ok, E::Error> {
        use en::EncodeMap;
        let size = if self.retry_after_ms.is_some() { 4 } else { 3 };
        let mut map = encoder.encode_map(Some(size))?;
        map.encode_entry("resource", self.resource)?;
        map.encode_entry("reason", self.reason)?;
        if let Some(retry_after_ms) = self.retry_after_ms {
            map.encode_entry("retry_after_ms", retry_after_ms)?;
        }
        map.encode_entry("reliability", self.reliability)?;
        map.end()
    }
}

impl<'en> en::ToStream<'en> for Pressure {
    fn to_stream<E: en::Encoder<'en>>(&'en self, encoder: E) -> Result<E::Ok, E::Error> {
        use en::EncodeMap;
        let size = if self.retry_after_ms.is_some() { 4 } else { 3 };
        let mut map = encoder.encode_map(Some(size))?;
        map.encode_entry("resource", &self.resource)?;
        map.encode_entry("reason", self.reason)?;
        if let Some(retry_after_ms) = self.retry_after_ms {
            map.encode_entry("retry_after_ms", retry_after_ms)?;
        }
        map.encode_entry("reliability", self.reliability)?;
        map.end()
    }
}

struct DataVisitor;

impl de::Visitor for DataVisitor {
    type Value = ErrorData;

    fn expecting() -> &'static str {
        "an error message and optional stacktrace"
    }

    async fn visit_map<A: de::MapAccess>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut message = None;
        let mut stack = None;
        let mut pressure = None;
        while let Some(key) = access.next_key::<String>(()).await? {
            match key.as_str() {
                "message" => message = Some(access.next_value::<String>(()).await?),
                "stack" => stack = Some(access.next_value::<Vec<String>>(()).await?),
                "pressure" => pressure = Some(access.next_value::<Pressure>(()).await?),
                _ => return Err(de::Error::custom(format!("unknown error field {key}"))),
            }
        }

        Ok(ErrorData {
            message: message.ok_or_else(|| de::Error::custom("missing error message"))?,
            stack: stack.unwrap_or_default(),
            pressure,
        })
    }
}

impl de::FromStream for ErrorData {
    type Context = ();

    async fn from_stream<D: de::Decoder>(_: (), decoder: &mut D) -> Result<Self, D::Error> {
        decoder.decode_any(DataVisitor).await
    }
}

impl<'en> en::IntoStream<'en> for ErrorData {
    fn into_stream<E: en::Encoder<'en>>(self, encoder: E) -> Result<E::Ok, E::Error> {
        use en::EncodeMap;
        let size = 2 + usize::from(self.pressure.is_some());
        let mut map = encoder.encode_map(Some(size))?;
        map.encode_entry("message", self.message)?;
        map.encode_entry("stack", self.stack)?;
        if let Some(pressure) = self.pressure {
            map.encode_entry("pressure", pressure)?;
        }
        map.end()
    }
}

impl<'en> en::ToStream<'en> for ErrorData {
    fn to_stream<E: en::Encoder<'en>>(&'en self, encoder: E) -> Result<E::Ok, E::Error> {
        use en::EncodeMap;
        let size = 2 + usize::from(self.pressure.is_some());
        let mut map = encoder.encode_map(Some(size))?;
        map.encode_entry("message", &self.message)?;
        map.encode_entry("stack", &self.stack)?;
        if let Some(pressure) = &self.pressure {
            map.encode_entry("pressure", pressure)?;
        }
        map.end()
    }
}

impl<T> From<T> for ErrorData
where
    T: fmt::Display,
{
    fn from(message: T) -> Self {
        Self {
            message: message.to_string(),
            stack: vec![],
            pressure: None,
        }
    }
}

/// The category of a `TCError`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ErrorKind {
    BadGateway,
    BadRequest,
    Conflict,
    Forbidden,
    Internal,
    MethodNotAllowed,
    NotFound,
    NotImplemented,
    PayloadTooLarge,
    Timeout,
    TooManyRequests,
    Unauthorized,
    Unavailable,
}

impl ErrorKind {
    pub fn is_conflict(&self) -> bool {
        *self == Self::Conflict
    }

    pub fn is_retriable(&self) -> bool {
        [Self::Timeout, Self::Unavailable]
            .into_iter()
            .any(|code| *self == code)
    }
}

impl FromStr for ErrorKind {
    type Err = TCError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bad_gateway" => Ok(Self::BadGateway),
            "bad_request" => Ok(Self::BadRequest),
            "conflict" => Ok(Self::Conflict),
            "forbidden" => Ok(Self::Forbidden),
            "internal_error" => Ok(Self::Internal),
            "method_not_allowed" => Ok(Self::MethodNotAllowed),
            "not_found" => Ok(Self::NotFound),
            "not_implemented" => Ok(Self::NotImplemented),
            "payload_too_large" => Ok(Self::PayloadTooLarge),
            "request_timeout" => Ok(Self::Timeout),
            "too_many_requests" => Ok(Self::TooManyRequests),
            "unauthorized" => Ok(Self::Unauthorized),
            "temporarily_unavailable" => Ok(Self::Unavailable),
            other => Err(bad_request!("unrecognized error code: {other}")),
        }
    }
}

impl de::FromStream for ErrorKind {
    type Context = ();

    async fn from_stream<D: de::Decoder>(cxt: (), decoder: &mut D) -> Result<Self, D::Error> {
        let code = String::from_stream(cxt, decoder).await?;
        code.parse()
            .map_err(|_| de::Error::invalid_value(code, "an error code"))
    }
}

impl<'en> en::IntoStream<'en> for ErrorKind {
    fn into_stream<E: en::Encoder<'en>>(self, encoder: E) -> Result<E::Ok, E::Error> {
        self.to_string().into_stream(encoder)
    }
}

impl fmt::Debug for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string().replace("_", " "))
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::BadGateway => "bad_gateway",
            Self::BadRequest => "bad_request",
            Self::Conflict => "conflict",
            Self::Forbidden => "forbidden",
            Self::Internal => "internal_error",
            Self::MethodNotAllowed => "method_not_allowed",
            Self::NotFound => "not_found",
            Self::NotImplemented => "not_implemented",
            Self::PayloadTooLarge => "payload_too_large",
            Self::Timeout => "request_timeout",
            Self::TooManyRequests => "too_many_requests",
            Self::Unauthorized => "unauthorized",
            Self::Unavailable => "temporarily_unavailable",
        })
    }
}

/// A general error description.
#[derive(Clone)]
pub struct TCError {
    kind: ErrorKind,
    data: ErrorData,
}

impl TCError {
    /// Returns a new error with the given code and message.
    pub fn new<I: fmt::Display>(code: ErrorKind, message: I) -> Self {
        log::warn!("{code}: {message}");

        Self {
            kind: code,
            data: message.into(),
        }
    }

    /// Reconstruct a [`TCError`] from its [`ErrorKind`] and data.
    pub fn with_stack<I, S, SI>(code: ErrorKind, message: I, stack: S) -> Self
    where
        I: fmt::Display,
        SI: fmt::Display,
        S: IntoIterator<Item = SI>,
    {
        let stack = stack.into_iter().map(|msg| msg.to_string()).collect();

        log::warn!("{code}: {message} (cause: {stack:?})");

        Self {
            kind: code,
            data: ErrorData {
                message: message.to_string(),
                stack,
                pressure: None,
            },
        }
    }

    /// Error to indicate a malformed or nonsensical request
    pub fn bad_request<I: fmt::Display>(info: I) -> Self {
        Self::new(ErrorKind::BadRequest, info)
    }

    /// Error to convey an upstream problem
    pub fn bad_gateway<I: fmt::Display>(locator: I) -> Self {
        Self::new(ErrorKind::BadGateway, locator)
    }

    /// Error to indicate that the requested resource is already locked
    pub fn conflict<I: fmt::Display>(info: I) -> Self {
        Self::new(ErrorKind::Conflict, info)
    }

    /// An internal error which should never occur.
    pub fn internal<I: fmt::Display>(info: I) -> Self {
        Self::new(ErrorKind::Internal, info)
    }

    /// Error to indicate that the requested resource exists but does not
    /// support the request method
    pub fn method_not_allowed<M: fmt::Debug, P: fmt::Display>(method: M, path: P) -> Self {
        let message = format!("endpoint {} does not support {:?}", path, method);
        Self::new(ErrorKind::MethodNotAllowed, message)
    }

    /// Error to indicate that the requested resource does not exist at the
    /// specified location
    pub fn not_found<I: fmt::Display>(locator: I) -> Self {
        Self::new(ErrorKind::NotFound, locator)
    }

    /// Error to indicate that the end-user is not authorized to perform the
    /// requested action
    pub fn unauthorized<I: fmt::Display>(info: I) -> Self {
        Self::new(ErrorKind::Unauthorized, info)
    }

    /// Error to indicate an unexpected input value or type
    pub fn unexpected<V: fmt::Debug>(value: V, expected: &str) -> Self {
        Self::bad_request(format!("invalid value {value:?}: expected {expected}"))
    }

    /// Error to indicate that the requested action cannot be performed due to
    /// technical limitations
    pub fn unsupported<I: fmt::Display>(info: I) -> Self {
        Self::bad_request(info)
    }

    /// The [`ErrorKind`] of this error
    pub fn code(&self) -> ErrorKind {
        self.kind
    }

    /// The error message of this error
    pub fn message(&self) -> &str {
        &self.data.message
    }

    /// Machine-readable pressure metadata, if this is a resource error.
    pub fn pressure(&self) -> Option<&Pressure> {
        self.data.pressure.as_ref()
    }

    /// Construct a structured temporary resource-exhaustion error.
    pub fn resource_unavailable<I: fmt::Display>(message: I, pressure: Pressure) -> Self {
        let mut err = Self::new(ErrorKind::Unavailable, message);
        err.data.pressure = Some(pressure);
        err
    }

    /// Construct an error for a payload which exceeds its transport budget.
    pub fn payload_too_large<I: fmt::Display>(message: I, pressure: Pressure) -> Self {
        let mut err = Self::new(ErrorKind::PayloadTooLarge, message);
        err.data.pressure = Some(pressure);
        err
    }

    /// Construct an error for an explicit caller or rate quota.
    pub fn too_many_requests<I: fmt::Display>(message: I, pressure: Pressure) -> Self {
        let mut err = Self::new(ErrorKind::TooManyRequests, message);
        err.data.pressure = Some(pressure);
        err
    }

    /// Construct a new error with the given `cause`
    pub fn consume<I: fmt::Debug>(mut self, cause: I) -> Self {
        self.data.stack.push(format!("{:?}", cause));
        self
    }
}

impl std::error::Error for TCError {}

#[cfg(feature = "pathlink")]
impl From<pathlink::ParseError> for TCError {
    fn from(err: pathlink::ParseError) -> Self {
        Self::bad_request(err)
    }
}

#[cfg(feature = "ha-ndarray")]
impl From<ha_ndarray::Error> for TCError {
    fn from(err: ha_ndarray::Error) -> Self {
        Self::new(ErrorKind::Internal, err)
    }
}

#[cfg(feature = "rjwt")]
impl From<rjwt::Error> for TCError {
    fn from(err: rjwt::Error) -> Self {
        #[cfg(debug_assertions)]
        panic!("rjwt error: {err}");

        #[cfg(not(debug_assertions))]
        match err.into_inner() {
            (rjwt::ErrorKind::Auth | rjwt::ErrorKind::Time, msg) => Self::unauthorized(msg),
            (rjwt::ErrorKind::Base64 | rjwt::ErrorKind::Format | rjwt::ErrorKind::Json, msg) => {
                Self::bad_request(msg)
            }
            (rjwt::ErrorKind::Fetch, msg) => Self::bad_gateway(msg),
        }
    }
}

#[cfg(feature = "txn_lock")]
impl From<txn_lock::Error> for TCError {
    fn from(err: txn_lock::Error) -> Self {
        match err {
            txn_lock::Error::Saturated => Self::resource_unavailable(
                err,
                Pressure::new("/host/resource/queue", PressureReason::Saturated)
                    .with_retry_after_ms(1000),
            ),
            err => Self::conflict(err),
        }
    }
}

#[cfg(feature = "txfs")]
impl From<txfs::Error> for TCError {
    fn from(err: txfs::Error) -> Self {
        match err {
            txfs::Error::Conflict(err) => Self::conflict(err),
            txfs::Error::Corrupt(message) => Self::internal(message),
            txfs::Error::IO(err) => err.into(),
            txfs::Error::NotFound(path) => Self::not_found(path),
            txfs::Error::Parse(err) => Self::bad_request(err),
        }
    }
}

impl From<io::Error> for TCError {
    fn from(cause: io::Error) -> Self {
        match cause.kind() {
            io::ErrorKind::AlreadyExists => {
                #[cfg(debug_assertions)]
                panic!("tried to create an entry that already exists: {}", cause);

                #[cfg(not(debug_assertions))]
                bad_request!("tried to create an entry that already exists").consume(cause)
            }
            io::ErrorKind::InvalidInput => bad_request!("{}", cause),
            io::ErrorKind::NotFound => TCError::not_found(cause),
            io::ErrorKind::PermissionDenied => {
                bad_gateway!("host filesystem permission denied").consume(cause)
            }
            io::ErrorKind::StorageFull => TCError::resource_unavailable(
                "host storage reserve exhausted",
                Pressure::new("/host/resource/storage", PressureReason::StorageFull),
            )
            .consume(cause),
            io::ErrorKind::OutOfMemory => TCError::resource_unavailable(
                "host memory allocation failed",
                Pressure::new("/host/resource/memory", PressureReason::AllocationFailed),
            )
            .consume(cause),
            io::ErrorKind::ResourceBusy => TCError::resource_unavailable(
                "host file-handle capacity exhausted",
                Pressure::new(
                    "/host/resource/storage/file-handle",
                    PressureReason::Saturated,
                )
                .with_retry_after_ms(1000),
            )
            .consume(cause),
            io::ErrorKind::WouldBlock => {
                conflict!("synchronous filesystem access failed").consume(cause)
            }
            kind => internal!("host filesystem error: {:?}", kind).consume(cause),
        }
    }
}

impl From<Infallible> for TCError {
    fn from(_: Infallible) -> Self {
        internal!("an unanticipated error occurred--please file a bug report")
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures::{stream, TryStreamExt};

    use super::*;

    #[test]
    fn bad_request_has_its_own_error_kind() {
        assert_eq!(
            TCError::bad_request("invalid").code(),
            ErrorKind::BadRequest
        );
    }

    #[cfg(feature = "txfs")]
    #[test]
    fn txfs_errors_map_by_kind() {
        assert_eq!(
            TCError::from(txfs::Error::Corrupt("invalid pending storage".into())).code(),
            ErrorKind::Internal
        );
        assert_eq!(
            TCError::from(txfs::Error::NotFound("missing".into())).code(),
            ErrorKind::NotFound
        );
        assert_eq!(
            TCError::from(txfs::Error::IO(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid path",
            )))
            .code(),
            ErrorKind::BadRequest
        );
    }

    #[tokio::test]
    async fn structured_pressure_round_trips() {
        let err = TCError::resource_unavailable(
            "request capacity exhausted",
            Pressure::new("/host/resource/request", PressureReason::Saturated)
                .with_retry_after_ms(250),
        );
        let encoded = destream_json::encode(err)
            .unwrap()
            .try_fold(Vec::new(), |mut bytes, chunk| async move {
                bytes.extend_from_slice(&chunk);
                Ok(bytes)
            })
            .await
            .unwrap();
        let decoded: TCError = destream_json::try_decode(
            (),
            stream::iter([Ok::<_, std::io::Error>(Bytes::from(encoded))]),
        )
        .await
        .unwrap();

        assert_eq!(decoded.code(), ErrorKind::Unavailable);
        assert_eq!(
            decoded.pressure(),
            Some(
                &Pressure::new("/host/resource/request", PressureReason::Saturated)
                    .with_retry_after_ms(250)
            )
        );
    }
}

struct ErrorVisitor;

impl de::Visitor for ErrorVisitor {
    type Value = TCError;

    fn expecting() -> &'static str {
        "an error code, message, and optional stacktrace"
    }

    async fn visit_map<A: de::MapAccess>(self, mut access: A) -> Result<Self::Value, A::Error> {
        if let Some(kind) = access.next_key(()).await? {
            let data = access.next_value(()).await?;
            Ok(TCError { kind, data })
        } else {
            Err(de::Error::invalid_length(0, Self::expecting()))
        }
    }
}

impl de::FromStream for TCError {
    type Context = ();

    async fn from_stream<D: de::Decoder>(_: (), decoder: &mut D) -> Result<Self, D::Error> {
        decoder.decode_map(ErrorVisitor).await
    }
}

impl<'en> en::IntoStream<'en> for TCError {
    fn into_stream<E: en::Encoder<'en>>(self, encoder: E) -> Result<E::Ok, E::Error> {
        use en::EncodeMap;
        let mut map = encoder.encode_map(Some(1))?;
        map.encode_entry(self.kind, self.data)?;
        map.end()
    }
}

impl<'en> en::ToStream<'en> for TCError {
    fn to_stream<E: en::Encoder<'en>>(&'en self, encoder: E) -> Result<E::Ok, E::Error> {
        use en::EncodeMap;
        let mut map = encoder.encode_map(Some(1))?;
        map.encode_entry(self.kind, &self.data)?;
        map.end()
    }
}

impl fmt::Debug for TCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for TCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.data.message)
    }
}

/// Error to convey an upstream problem
#[macro_export]
macro_rules! bad_gateway {
    ($($t:tt)*) => {{
        $crate::TCError::new($crate::ErrorKind::BadGateway, format!($($t)*))
    }}
}

/// Error to indicate that the request is badly-constructed or nonsensical
#[macro_export]
macro_rules! bad_request {
    ($($t:tt)*) => {{
        $crate::TCError::bad_request(format!($($t)*))
    }}
}

/// Error to indicate that the request cannot be fulfilled due to a conflict
/// with another request.
#[macro_export]
macro_rules! conflict {
    ($($t:tt)*) => {{
        $crate::TCError::conflict(format!($($t)*))
    }}
}

/// Error to indicate that the requestor's credentials do not authorize the
/// request to be fulfilled
#[macro_export]
macro_rules! forbidden {
    ($($t:tt)*) => {{
        $crate::TCError::new($crate::ErrorKind::Unauthorized, format!($($t)*))
    }}
}

/// Error to indicate that no resource exists at the requested path or key.
#[macro_export]
macro_rules! not_found {
    ($($t:tt)*) => {{
        $crate::TCError::not_found(format!($($t)*))
    }}
}

/// Error to indicate that a required feature is not yet implemented.
#[macro_export]
macro_rules! not_implemented {
    ($($t:tt)*) => {{
        $crate::TCError::new($crate::ErrorKind::NotImplemented, format!($($t)*))
    }}
}

/// Error to indicate that the request failed to complete in the allotted time.
#[macro_export]
macro_rules! timeout {
    ($($t:tt)*) => {{
        $crate::TCError::new($crate::ErrorKind::Timeout, format!($($t)*))
    }}
}

/// A truly unexpected error, for which no handling behavior can be defined
#[macro_export]
macro_rules! internal {
    ($($t:tt)*) => {{
        $crate::TCError::internal(format!($($t)*))
    }}
}

/// Error to indicate that the user's credentials are missing or nonsensical.
#[macro_export]
macro_rules! unauthorized {
    ($($t:tt)*) => {{
        $crate::TCError::unauthorized(format!($($t)*))
    }}
}

/// Error to indicate that this host is currently overloaded
#[macro_export]
macro_rules! unavailable {
    ($($t:tt)*) => {{
        $crate::TCError::new($crate::ErrorKind::Unavailable, format!($($t)*))
    }}
}
