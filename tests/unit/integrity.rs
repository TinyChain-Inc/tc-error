use std::io;

use bytes::Bytes;
use futures::{stream, TryStreamExt};

use super::TCError;

#[tokio::test]
async fn integrity_provenance_is_local_and_survives_native_conversion() {
    let truncated = freqfs::IntegrityError::from_read(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "truncated stored header",
    ));

    assert!(TCError::from(truncated).requires_recovery());
    let native = freqfs::IntegrityError::invalid_data("stored payload is inconsistent");
    assert_eq!(native.kind(), io::ErrorKind::InvalidData);
    assert!(std::error::Error::source(native.get_ref().unwrap()).is_some());
    let error = TCError::from(native);
    assert!(error.requires_recovery());
    assert!(error
        .clone()
        .consume("additional context")
        .requires_recovery());

    let message = error.to_string();
    let encoded = destream_json::encode(error)
        .unwrap()
        .try_fold(Vec::new(), |mut bytes, chunk| async move {
            bytes.extend_from_slice(&chunk);
            Ok(bytes)
        })
        .await
        .unwrap();

    let decoded: TCError =
        destream_json::try_decode((), stream::iter([Ok::<_, io::Error>(Bytes::from(encoded))]))
            .await
            .unwrap();

    assert_eq!(decoded.to_string(), message);
    assert!(
        !decoded.requires_recovery(),
        "a remote error cannot authorize local termination"
    );

    for kind in [
        io::ErrorKind::InvalidData,
        io::ErrorKind::UnexpectedEof,
        io::ErrorKind::Interrupted,
        io::ErrorKind::NotFound,
        io::ErrorKind::StorageFull,
        io::ErrorKind::PermissionDenied,
    ] {
        assert!(!TCError::from(io::Error::new(kind, "ordinary failure")).requires_recovery());
    }
}
