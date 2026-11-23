# tc-error

Shared error classification and format-neutral codecs for TinyChain. `TCError`
carries an `ErrorKind`, diagnostic context, and optional structured pressure data;
`TCResult<T>` is `Result<T, TCError>`.

```rust
use tc_error::{TCError, TCResult};

fn expect_true(value: bool) -> TCResult<()> {
    if value {
        Ok(())
    } else {
        Err(TCError::bad_request("expected true"))
    }
}

assert!(expect_true(true).is_ok());
assert!(expect_true(false).is_err());
```

Use `consume` to retain diagnostic context and `pressure()` to inspect typed
admission information. Values implement `destream` codecs; the process or
transport boundary selects the wire format.

## Local recovery provenance

`requires_recovery()` records locally detected storage uncertainty. Context and
cloning retain this marker; serialization omits it. Decoding an error does not
confer local recovery authority. Resource owners decide rollback, recovery, and
process termination; this crate performs none of them.

## Optional conversions

The default build supplies core error types, codecs, and standard I/O conversions.
Optional features delegate classification to the corresponding dependency:

- `pathlink`: Link/path and `hr-id` parsing failures become BadRequest errors.
- `txn_lock`: transaction ordering, conflict, and interruption errors.
- `txfs`: transactional filesystem errors and local corruption provenance.
- `ha-ndarray`: numerical backend errors.
- `fensor`: Tensor errors and delegated numerical/I/O causes; enables the required
  `ha-ndarray` and `freqfs` support.
- `rjwt`: token/authentication errors.
- `freqfs`: native storage-integrity cause recognition in I/O errors.

Conversions use variants and retained causes rather than diagnostic wording.

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup and validation and [AGENTS.md](AGENTS.md)
for implementation invariants. Licensed under [Apache-2.0](LICENSE).
