## Implementation:

This program uses the csv and Serde crates for serialization and deserialization. The csv crate handles read and write buffering of csv data. The decimal crate is used for precision and easy integration with Serde.

My goal was to keep it simple to read and review. There is a single `process_csv` entrypoint in `src/lib.rs`.

Unit tests are added in the lib folder. Additional integration tests are added in the `tests` directory.

## Usage

```
cargo run -- transactions.csv > accounts.csv
```

## Testing:

Tests are written with cargo and can be run with:

```
cargo test
```

## Validations:

The following cases are checked. If validation fails the transactions are skipped:

* Insufficient available funds during withdrawals and disputes.
* Insufficient held funds during chargebacks.
* Invalid transaction ID for disputes, resolves, and withdrawals
* Resolves and chargebacks for transactions that are not disputed
* All transactions for locked accounts
* Duplicate disputes, resolves, and chargebacks
* Duplicate transactions IDs for deposits and withdrawals
* Disputes, resolves, chargebacks that do not match the original client
* Chargebacks that reference non-deposit transactions
* Transactions containing invalid or negative amounts

In a real world scenario many of these cases should be flagged for review. For example, funds under dispute could already be withdrawn:

* Deposit,1,$500
* Withdraw,2,$500
* Dispute,1
* Chargeback,1

This causes a clash between processing the dispute and causing the account balance to become negative. In this implementation I chose to disallow negative held and available balances since the chargeback could potentially allow for the deposit to be withdrawn twice by a malicious actor.

## Resource usage:

This implementation uses in memory data structures for accounts and deposits:

Asset accounts are stored using a `HashMap<u16, Account>`.

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

Deposits are stored using a `HashMap<u32, Transaction>`.

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
