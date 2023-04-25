use anyhow::Result;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_keygen::KeyGen;
use aptos_types::{
    account_address::{create_multisig_account_address, AccountAddress},
    transaction::authenticator::AuthenticationKey,
};
use clap::Parser;
use crossbeam::channel;
use std::thread::{self, available_parallelism};

#[derive(Parser, Debug)]
struct CliArgs {
    // Number of vanity address keypairs to generate
    #[arg(short, long)]
    number: u8,
    // Address prefix to search
    #[arg(short, long)]
    prefix: String,
    // Number of threads to use
    #[arg(short, long, default_value_t = available_parallelism().unwrap().get())]
    threads: usize,
    // Whether to search for multisig addresses
    #[arg(short, long)]
    multisig: bool,
    // Threshold for multisig addresses
    #[arg(long)]
    multisig_threshold: Option<u8>,
}

/// Derive authentication key bytes vector from a reference to a private key.
fn auth_key_bytes_vec(private_key_ref: &Ed25519PrivateKey) -> Vec<u8> {
    AuthenticationKey::ed25519(&Ed25519PublicKey::from(private_key_ref)).to_vec()
}

fn parse_args() -> Result<CliArgs> {
    let args = CliArgs::parse();
    if args.multisig && args.multisig_threshold.is_none() {
        anyhow::bail!("Multisig threshold must be specified");
    }

    if args.prefix.len() % 2 == 1 {
        hex::decode(format!("{}0", &args.prefix))?;
    } else {
        hex::decode(&args.prefix)?;
    }

    Ok(args)
}

fn generate_key(
    prefix: String,
    multisig: bool,
    res_tx: channel::Sender<(String, String)>,
    exit_rx: channel::Receiver<()>,
) -> Result<()> {
    let mut key_generator = KeyGen::from_os_rng();
    let prefix_bytes = hex::decode(&prefix)?;
    loop {
        if exit_rx.try_recv().is_ok() {
            return Ok(());
        }

        let private_key = key_generator.generate_ed25519_private_key();
        let account_address_bytes = auth_key_bytes_vec(&private_key);
        let search_bytes = if multisig {
            create_multisig_account_address(AccountAddress::from_bytes(account_address_bytes)?, 0)
                .to_vec()
        } else {
            // If standard account, check against account address.
            account_address_bytes
        };

        // If a match found, print diagnostics then keep searching.
        if search_bytes.starts_with(&prefix_bytes) {
            let search_str = hex::encode(search_bytes);
            if !search_str.starts_with(&prefix) {
                continue;
            }
            // Get standard account address. If multisig:
            let account_address_hex = if multisig {
                println!("Multisig account address: 0x{}", search_str);
                // Re-derive account address bytes here rather than cloning during search.
                hex::encode(auth_key_bytes_vec(&private_key))
            } else {
                // If not multisig, standard account address is from search.
                search_str
            };

            res_tx.send((
                account_address_hex.clone(),
                hex::encode(private_key.to_bytes()),
            ))?;
        }
    }
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let (res_tx, res_rx) = channel::unbounded::<(String, String)>();
    let (exit_tx, exit_rx) = channel::bounded::<()>(1);

    // Spawn parallel search threads.
    for _ in 0..args.threads {
        let res_tx = res_tx.clone();
        let exit_rx = exit_rx.clone();
        let prefix = args.prefix.clone();
        thread::spawn(move || generate_key(prefix, args.multisig, res_tx, exit_rx).unwrap());
    }

    for _ in 0..args.number {
        let (account_address_hex, private_key) = res_rx.recv()?;
        println!("Standard account address: 0x{}", account_address_hex);
        println!("Private key:              0x{}\n", private_key);
    }

    exit_tx.send(())?;
    Ok(())
}
