## Implementation:

This program uses the csv and Serde crates for serialization and deserialization. The csv crate handles read and write buffering of csv data. The decimal crate is used for precision and easy integration with Serde.

## Resource usage:

Asset accounts are stored in memory using a `HashMap<u16, Account>`.

```
field       bytes
-----------------
client      2
available   16
held        16
total       16
locked      1
            ---
total       52
```

This could grow up to: `65535 * 56 = ~3.5mb`

Deposits are stored in memory using a `HashMap<u32, Transaction>`.

```
field       bytes
-----------------
tx_type     1
client      2
tx          4
amount      16
disputed    1
            ---
total       24
```

1MB of memory can hold: `1024**2 / 24 = ~43000` transactions.

## Testing:

Tests are written with cargo and can be run with:

```
cargo test
```

## Validations:

The following cases are checked and ignored:

* `InsufficientFunds` for withdrawals
* `InvalidTransaction` for disputes, resolves, and withdrawals 
* `NotDisputed` for resolves and chargebacks
* `AccountLocked` for all transactions on locked accounts
* `DuplicateTransaction` for disputes, resolves, and chargebacks

The following cases are assumed to be out of scope:

* `DuplicateRecord` for deposits and withdrawals
* `ClientMismatch` for disputes, resolves, chargebacks
* `InvalidChargeback` for chargebacks that reference non-deposit transactions
* `InvalidAmount` for transactions that contain invalid or negative amounts