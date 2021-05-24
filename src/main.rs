use std::{env, io};
use csv;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

fn help() {
    println!("
    This program parses one CSV file as input.
    Eg: cargo run -- transactions.csv
    OR
    transactions_parser transactions.csv
    The output is text in the CSV format as well
    Hist, you can save that as a csv file:
    cargo run -- input.csv > output.csv
    For more info check the README file
    ");
}
fn main() {
    let args: Vec<String> = env::args().collect();

    let input_file:String;

    // keep all the accounts here
    let mut accounts: HashMap<u16, Account> = HashMap::new();

    // keep relevant transactions here (deposit and withdarawals)
    let mut transactions: HashMap<u32, Transaction> = HashMap::new();

    // checks correct number of arguments is received
    // TODO: check if valid CSV file, and fail gracefully
    match args.len() {
        1 => {
            eprintln!("No arguments passed!");
            help();
            return;
        },
        2 => {
            match args[1].parse::<String>() {
                Ok(inp) => {
                    input_file = inp;
                },
                _ => {
                    eprintln!("Sorry, could not parse the input file!");
                    return;
                }
            }
        },
        _ => {
            help();
            return;
        }

    }

    // create the CSV reader
    let mut reader = match csv::Reader::from_path(input_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Could not create CSV reader for input.
             Error: {}", e );
            return;
        }
    };

    // go through input transactions
    for result in reader.deserialize() {
        match result {
            Ok(r) => {
                let transaction: Transaction = r;

                // get or create a new client account
                let mut client_account = match accounts.get(&transaction.client) {
                    Some(c) => c.to_owned(),
                    None =>  {
                        accounts.insert(transaction.client, Account::new(&transaction.client));
                        match accounts.get(&transaction.client) {
                            Some(nc) => nc.to_owned(),
                            None => {
                                panic!("Could not add a new account!")
                            }
                        }
                    }
                    
                };
                
                let transaction_type = transaction.r#type.as_str();

                // depending on operation, do the needfull :)

                match transaction_type {
                    "deposit" => {
                        if client_account.deposit(transaction.get_amount()) {
                            transactions.insert(transaction.tx, transaction);                            
                            }                        
                    },
                    "withdrawal" => {
                        if client_account.withdraw(transaction.get_amount()) {
                            transactions.insert(transaction.tx, transaction);
                        }
                    },
                    "dispute" => {
                        let t = transactions.get(&transaction.tx);

                        match t {
                            Some(tr) => {
                                if !tr.disputed {
                                    if client_account.dispute(tr) {
                                        let mut newt = tr.clone();
                                        newt.disputed = true;
                                        transactions.insert(newt.tx, newt);
                                    }
                                }
                            },
                            None => {
                                // no transaction can be found to be disputed
                                // TODO: maybe let the user know
                            }
                        }                        
                    },
                    "resolve" => {
                        let t = transactions.get(&transaction.tx);

                        match t {
                            Some(tr) => {
                                if tr.disputed {
                                    if client_account.resolve(tr) {
                                        let mut newt = tr.clone();
                                        newt.disputed = false;
                                        transactions.insert(newt.tx, newt);
                                    }
                                }
                            },
                            None => {
                                // no transaction can be found to be resolved
                                // TODO: maybe let the user know
                            }
                        }  
                    },
                    "chargeback" => {
                        let t = transactions.get(&transaction.tx);

                        match t {
                            Some(tr) => {
                                if tr.disputed {
                                    if client_account.chargeback(tr) {
                                        let mut newt = tr.clone();
                                        newt.disputed = false;
                                        transactions.insert(newt.tx, newt);
                                    }
                                }
                            },
                            None => {
                                // no transaction can be found to be chargedback
                                // TODO: maybe let the user know
                            }
                        }
                    },
                    _ => {
                        eprintln!("Unsuported operation: {}, skipping", transaction_type)
                    }
                }
                
                // update the account in the hashmap
                
                accounts.insert(client_account.client, client_account);

            },
            Err(e) => {
                eprintln!("Could not get line!. Error: {}", e)
            }
        }
    }

    // create the CSV Writer
    let mut writer = csv::Writer::from_writer(io::stdout());

    // start serializing output
    for (_client_id, acc) in &accounts {

        if let Err(err) = writer.serialize(acc){
            eprintln!("Cannot write line ! Err: {}", err);
        }
    }

    // flush
    // TODO: check if we should maybe just output directly every line (save memory, etc)
    if let Err(err) = writer.flush() {
        eprintln!("Could not flush the CSV data to output. Err: {}", err);
    }


}

// The transaction data representation
#[derive(Debug, Clone, Default, Deserialize)]
struct Transaction {
    
    r#type: String,

    client: u16,

    tx: u32,
    
    amount: Option<f32>,

    #[serde(skip_deserializing)]
    disputed: bool

}

impl Transaction {
    fn get_amount(&self) -> f32 {
        // default the amount to zero
        let amount = match self.amount {
            Some(a) => a,
            None => 0.0
        };
        amount
    }
}

// Account representation
#[derive(Debug, Clone, Serialize)]
struct Account {
    client: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool
}

impl Account {
    // Some sane defaults for a new account
    fn new(client_id: &u16) -> Account {
        Account{
            client: client_id.to_owned(),
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false
        }
    }

    fn round_amounts(&mut self) {
        self.available = (self.available * 10000.0).round() / 10000.0;
        self.held = (self.held * 10000.0).round() / 10000.0;
        self.total = (self.total * 10000.0).round() / 10000.0;
    }
    
    fn deposit(&mut self, amount: f32) -> bool{
        match self.locked {
            false => {
                self.available += amount;
                self.total += amount;
                self.round_amounts();
                },
            true => {
                eprintln!("Account {} locked, cannot deposit {}", self.client, amount);
                return false;
            }
        }
        true      
    }

    fn withdraw(&mut self, amount: f32) -> bool{
        match self.locked {
            false => {
                if self.available - amount > 0.0  {
                    self.available -= amount;
                    self.total -= amount;
                    self.round_amounts();
                } else {
                    eprintln!("Could not withdraw {} from {}. Not enough funds!", amount, self.client);
                    return false;
                }
            },
            true => {
                eprintln!("Account {} locked, cannot withdraw {}", self.client, amount);
                return false;
            }
        }
        true
    }

    fn dispute(&mut self, tx: &Transaction) -> bool{
        match self.locked {
            false => {
                let amt = tx.get_amount();
                self.held += amt;
                self.available -= amt;
                self.round_amounts();
            },
            true => {
                eprintln!("Account {} locked, cannot dispute!", self.client);
                return false;             
            }
        }
        true
    }

    fn resolve(&mut self, tx: &Transaction) -> bool{
        match self.locked {
            false => {
                let amt = tx.get_amount();
                self.held -= amt;
                self.available += amt;
                self.round_amounts();
            },
            true => {
                eprintln!("Account {} locked, cannot resolve!", self.client);
                return false;               
            }
        }
        true
    }

    fn chargeback(&mut self, tx: &Transaction) -> bool {
        match self.locked {
            false => {
                let amt = tx.get_amount();
                self.held -= amt;
                self.total -= amt;
                self.locked = true;
                self.round_amounts();
            },
            true => {
                eprintln!("Account {} locked, cannot chargeback!", self.client);
                return false;               
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_deposit() {
        let mut acc = Account::new(&123);
        acc.deposit(3.1415);
        assert_eq!(acc.available, 3.1415);
        assert_eq!(acc.total, 3.1415);
    }

    #[test]
    fn test_withdraw() {
        let mut acc = Account::new(&123);
        acc.deposit(3.1415);
        acc.withdraw(2.1414);
        assert_eq!(acc.available, 1.0001);
        assert_eq!(acc.total, 1.0001);
        // test withdraw more than available
        assert_eq!(acc.withdraw(12.1414), false);
    }

    #[test]
    fn test_dispute() {
        let mut acc = Account::new(&123);
        acc.deposit(10.1234);
        let tr = Transaction{
            r#type: String::from("withdraw"),
            client: 123,
            tx: 5001,
            amount: Some(5.1002),
            disputed: false
        };
        acc.withdraw(5.1002);

        acc.dispute(&tr);
        assert_eq!(acc.available, -0.077);
        assert_eq!(acc.held, 5.1002);
        assert_eq!(acc.total, 5.0232);
    }

    #[test]
    fn test_resolve() {
        let mut acc = Account::new(&123);
        acc.deposit(10.1234);
        let tr = Transaction{
            r#type: String::from("withdraw"),
            client: 123,
            tx: 5001,
            amount: Some(5.1002),
            disputed: false
        };
        acc.withdraw(5.1002);

        acc.dispute(&tr);

        acc.resolve(&tr);
        assert_eq!(acc.available, 5.0232);
        assert_eq!(acc.held, 0.0);
        assert_eq!(acc.total, 5.0232);
    }

    #[test]
    fn test_chargeback() {
        let mut acc = Account::new(&123);
        acc.deposit(10.1234);
        let tr = Transaction{
            r#type: String::from("withdraw"),
            client: 123,
            tx: 5001,
            amount: Some(5.1002),
            disputed: false
        };
        acc.withdraw(5.1002);

        acc.dispute(&tr);

        acc.chargeback(&tr);
        assert_eq!(acc.available, -0.077);
        assert_eq!(acc.held, 0.0);
        assert_eq!(acc.total, -0.077);
        assert_eq!(acc.locked, true);
    }

    #[test]
    fn test_locked_operations() {
        let mut acc = Account::new(&123);
        acc.deposit(10.1234);
        let tr = Transaction{
            r#type: String::from("withdraw"),
            client: 123,
            tx: 5001,
            amount: Some(5.1002),
            disputed: false
        };
        acc.withdraw(5.1002);

        acc.dispute(&tr);

        acc.chargeback(&tr);
        assert_eq!(acc.deposit(0.1), false);
        assert_eq!(acc.withdraw(0.1), false);
        assert_eq!(acc.dispute(&tr), false);
        assert_eq!(acc.resolve(&tr), false);
        assert_eq!(acc.chargeback(&tr), false);
    }


}
