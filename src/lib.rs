use rust_decimal::prelude::Decimal;
use std::error::Error;

type Transactions = std::collections::HashMap<u32, Transaction>;

#[derive(Debug, serde::Deserialize)]
enum TransactionType {
    #[serde(rename = "chargeback")]
    Chargeback,
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "withdrawal")]
    Withdrawal,
}

#[derive(Debug, serde::Deserialize)]
struct Transaction {
    #[serde(rename = "type")]
    tx_type: TransactionType,
    client: u16,
    tx: u32,
    amount: Decimal,
    #[serde(skip)]
    disputed: bool,
}

pub type Accounts = std::collections::HashMap<u16, Account>;

#[derive(Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

fn process_transaction(
    transaction: Transaction,
    accounts: &mut Accounts,
    deposits: &mut Transactions,
) -> Result<(), Box<dyn Error>> {
    let account = accounts.entry(transaction.client).or_insert(Account {
        client: transaction.client,
        ..Default::default()
    });

    if account.locked {
        return Ok(());
    }

    match transaction.tx_type {
        TransactionType::Chargeback => {
            if let Some(deposit) = deposits.get_mut(&transaction.tx) {
                if deposit.client == account.client
                    && deposit.disputed
                    && account.held >= deposit.amount
                {
                    account.held -= deposit.amount;
                    account.locked = true;
                }
            }
        }
        TransactionType::Deposit => {
            if !deposits.contains_key(&transaction.tx) && transaction.amount > 0.into() {
                account.available += transaction.amount;
                deposits.insert(transaction.tx, transaction);
            }
        }
        TransactionType::Dispute => {
            if let Some(deposit) = deposits.get_mut(&transaction.tx) {
                if deposit.client == account.client
                    && !deposit.disputed
                    && account.available >= deposit.amount
                {
                    account.available -= deposit.amount;
                    account.held += deposit.amount;
                    deposit.disputed = true;
                }
            }
        }
        TransactionType::Resolve => {
            if let Some(deposit) = deposits.get_mut(&transaction.tx) {
                if deposit.client == account.client
                    && deposit.disputed
                    && account.held >= deposit.amount
                {
                    account.available += deposit.amount;
                    account.held -= deposit.amount;
                    deposit.disputed = false
                }
            }
        }
        TransactionType::Withdrawal => {
            if account.available >= transaction.amount && transaction.amount > 0.into() {
                account.available -= transaction.amount;
            }
        }
    }

    account.total = account.available + account.held;

    Ok(())
}

fn process_reader<R>(mut rdr: csv::Reader<R>) -> Result<Accounts, Box<dyn Error>>
where
    R: std::io::Read,
{
    let mut accounts = Accounts::new();
    let mut transactions = Transactions::new();
    for result in rdr.deserialize() {
        let transaction: Transaction = result?;
        process_transaction(transaction, &mut accounts, &mut transactions)?;
    }
    Ok(accounts)
}

/// Read a transaction CSV file, process each row, and write the account summary to stdout.
pub fn process_csv<P: AsRef<std::path::Path>>(path: P) -> Result<Accounts, Box<dyn Error>> {
    let rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;
    let accounts = process_reader(rdr)?;

    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    for account in accounts.values() {
        wtr.serialize(account)?;
    }
    wtr.flush()?;

    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use crate::*;

    fn compare(input: &str, exp_in: &str) -> Result<(), Box<dyn Error>> {
        let rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(input.as_bytes());
        let got = process_reader(rdr)?;

        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_reader(exp_in.as_bytes());

        let mut exp = Accounts::new();
        for result in rdr.deserialize() {
            let acct: Account = result?;
            exp.insert(acct.client, acct);
        }

        assert_eq!(got.len(), exp.len(), "\nGot: {:?} != \nExp: {:?}", got, exp,);

        for (client, acct1) in &got {
            match exp.get(client) {
                Some(acct2) => {
                    assert_eq!(acct1, acct2);
                }
                None => panic!("Client {client} not found in expected output"),
            }
        }

        Ok(())
    }

    #[test]
    fn chargeback() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1, 0
            chargeback, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 0.0, 0.0, 0.0, true"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn chargeback_duplicate() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1, 0
            chargeback, 1, 1, 0
            chargeback, 1, 1, 0
            chargeback, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 0.0, 0.0, 0.0, true"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn chargeback_invalid_tx() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1, 0
            chargeback, 1, 2, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 0.0, 10.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn chargeback_loop() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1, 0
            chargeback, 1, 1, 0
            dispute, 1, 1, 0
            chargeback, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 0.0, 0.0, 0.0, true"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn chargeback_lock() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 10.0
            dispute, 1, 1, 0
            chargeback, 1, 1, 0
            deposit, 1, 3, 10.0
            deposit, 1, 4, 10.0
            deposit, 1, 5, 10.0
            withdrawal, 1, 6, 5.0
            withdrawal, 1, 7, 5.0
            dispute, 1, 2, 0
            dispute, 1, 3, 0
            resolve, 1, 3, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 10.0, 0.0, 10.0, true"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn chargeback_nondisputed() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            chargeback, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 10.0, 0.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn deposit() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 2, 2, 5.0
            deposit, 2, 3, 5.0
            deposit, 1, 4, 10.0"#;
        let exp = r#"client, available, held, total, locked
            1, 20.0, 0.0, 20.0, false
            2, 10.0, 0.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn dispute() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            dispute, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 5.0, 10.0, 15.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn dispute_duplicate() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            dispute, 1, 1, 0
            dispute, 1, 1, 0
            dispute, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 5.0, 10.0, 15.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn dispute_invalid_tx() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 2, 0
            dispute, 1, 3, 0
            dispute, 1, 4, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 10.0, 0.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn resolve() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            dispute, 1, 1, 0
            resolve, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 15.0, 0.0, 15.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn resolve_duplicate() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            dispute, 1, 1, 0
            resolve, 1, 1, 0
            resolve, 1, 1, 0
            resolve, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 15.0, 0.0, 15.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn resolve_invalid_tx() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            resolve, 1, 2, 0
            resolve, 1, 3, 0
            resolve, 1, 4, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 10.0, 0.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn resolve_loop() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1, 0
            resolve, 1, 1, 0
            dispute, 1, 1, 0
            resolve, 1, 1, 0"#;
        let exp = r#"client, available, held, total, locked
            1, 10.0, 0.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn withdraw() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 5.0"#;
        let exp = r#"client, available, held, total, locked
            1, 5.0, 0.0, 5.0, false"#;
        compare(input, exp).unwrap();
    }

    #[test]
    fn withdraw_insufficient_funds() {
        let input = r#"type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 15.0"#;
        let exp = r#"client, available, held, total, locked
            1, 10.0, 0.0, 10.0, false"#;
        compare(input, exp).unwrap();
    }
}
