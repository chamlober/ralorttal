#[cfg(test)]
mod integration {
    const CMD: &str = "./target/debug/transactions";
    use std::error::Error;
    use std::process::Command;
    use transactions::{Account, Accounts};

    fn compare(name: &str) -> Result<(), Box<dyn Error>> {
        let inpath = format!("tests/{}.csv", name);
        let outpath = format!("tests/{}.out.csv", name);

        let output = Command::new(CMD).arg(inpath).output().unwrap();
        let s = String::from_utf8_lossy(&output.stdout);

        let mut got = Accounts::new();
        {
            let mut rdr = csv::ReaderBuilder::new()
                .trim(csv::Trim::All)
                .from_reader(s.as_bytes());
            for result in rdr.deserialize() {
                let acct: Account = result?;
                got.insert(acct.client, acct);
            }
        }

        let mut exp = Accounts::new();
        {
            let mut rdr = csv::ReaderBuilder::new()
                .trim(csv::Trim::All)
                .from_path(outpath)?;
            for result in rdr.deserialize() {
                let acct: Account = result?;
                exp.insert(acct.client, acct);
            }
        }

        assert_eq!(
            got.len(),
            exp.len(),
            "\nGot: {:?} != \nExp: {:?}",
            got.len(),
            exp.len()
        );

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
    fn call_empty() {
        let output = Command::new(CMD).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Error: No CSV path provided\n"
        );
    }

    #[test]
    fn missing_file() {
        let output = Command::new(CMD).arg("missing.csv").output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "Error: No such file or directory (os error 2)\n"
        );
    }

    #[test]
    fn chargeback() {
        compare("chargeback").unwrap();
    }

    #[test]
    fn complex() {
        compare("complex").unwrap();
    }

    #[test]
    fn deposits() {
        compare("deposits").unwrap();
    }

    #[test]
    fn deposit_duplicate_tx() {
        compare("deposit_duplicate_tx").unwrap();
    }

    #[test]
    fn deposit_negative() {
        compare("deposit_negative").unwrap();
    }

    #[test]
    fn dispute() {
        compare("dispute").unwrap();
    }

    #[test]
    fn large() {
        compare("large").unwrap();
    }

    #[test]
    fn resolve() {
        compare("resolve").unwrap();
    }

    #[test]
    fn withdrawal() {
        compare("withdrawal").unwrap();
    }

    #[test]
    fn withdrawal_negative() {
        compare("withdrawal_negative").unwrap();
    }
}
