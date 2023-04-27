use anyhow::Result;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_keygen::KeyGen;
use aptos_types::{
    account_address::{create_multisig_account_address, AccountAddress},
    transaction::authenticator::AuthenticationKey,
};
use chrono::prelude::{DateTime, Local};
use clap::Parser;
use crossbeam::channel;
use std::thread::{self, available_parallelism};
use std::time::{Instant, SystemTime};

/// Multisig account generation is assumed to take place in first transaction of standard account.
const SEQUENCE_NUMBER_MULTISIG: u64 = 0;

/// Optivanity: hyper-parallelized vanity address generator for the Aptos blockchain, brought to you
/// by Econia Labs
#[derive(Parser, Debug)]
struct CliArgs {
    /// Address prefix to match (no leading `0x`). Each additional character slows search by 16x.
    #[arg(short, long)]
    prefix: String,
    /// Use this flag if you want to search for multisig address(es)
    #[arg(short, long)]
    multisig: bool,
    /// Number of vanity accounts to generate
    #[arg(short, long, default_value_t = 1)]
    count: u64,
    /// Number of threads to use. Only specify if you want to use fewer cores than available.
    #[arg(short, long, default_value_t = available_parallelism().unwrap().get())]
    threads: usize,
}

/// Derive authentication key bytes vector from a reference to a private key.
fn auth_key_bytes_vec(private_key_ref: &Ed25519PrivateKey) -> Vec<u8> {
    AuthenticationKey::ed25519(&Ed25519PublicKey::from(private_key_ref)).to_vec()
}

/// Parse command line arguments, verifying hex characters and specified thread count.
fn parse_args() -> Result<CliArgs> {
    let args = CliArgs::parse();

    // Verify prefix has valid hex characters, appending a 0 as needed for an odd character count.
    if has_odd_character_count(&args.prefix) {
        hex::decode(format!("{}0", &args.prefix))?;
    } else {
        hex::decode(&args.prefix)?;
    }

    Ok(args)
}

/// Return `true` if string slice reference indicates a string with an odd number of characters.
fn has_odd_character_count(string_ref: &str) -> bool {
    string_ref.len() % 2 == 1
}

/// Generate a private key corresponding to a vanity prefix, while search is ongoing.
///
/// Once a match is found, a match message is transmitted to the main thread. Once the main thread
/// has received sufficient match transactions, it broadcasts an exit transaction to search threads.
///
/// # Arguments
///
/// * `prefix` - The vanity prefix to search against
/// * `multisig` - If `true` search for a multisig address
/// * `match_tx` - Transmit channel for match message sent to main thread when a match is found
/// * `exit_rx` - Receive channel for exit message broadcast from main thread when search is done
fn generate_key(
    prefix: String,
    multisig: bool,
    match_tx: channel::Sender<()>,
    exit_rx: channel::Receiver<()>,
) -> Result<()> {
    // Clone prefix for getting bytes to check against.
    let mut check_str = prefix.clone();
    // Pop last character of check string if an odd length.
    if has_odd_character_count(&check_str) {
        check_str.pop();
    };

    // Convert resulting string to bytes for checking against. For odd-length prefixes this approach
    // truncates the final character, which must be checked via string compare after a bytes match.
    // This is done for performance (to minimize string operations during the loop).
    let prefix_bytes = hex::decode(check_str)?;

    // Randomly generate private keys in a loop and check match against prefix bytes.
    let mut key_generator = KeyGen::from_os_rng();
    loop {
        // Exit search loop if exit message has been broadcast.
        if exit_rx.try_recv().is_ok() {
            return Ok(());
        }
        // Generate a private key and from it, bytes to compare against prefix bytes.
        let private_key = key_generator.generate_ed25519_private_key();
        let account_address_bytes = auth_key_bytes_vec(&private_key);
        let search_bytes = if multisig {
            create_multisig_account_address(
                AccountAddress::from_bytes(account_address_bytes)?,
                SEQUENCE_NUMBER_MULTISIG,
            )
            .to_vec()
        } else {
            account_address_bytes
        };

        // If a match found, print diagnostics then keep searching.
        if search_bytes.starts_with(&prefix_bytes) {
            let search_str = hex::encode(search_bytes);
            // Verify full string match only if prefix has an odd character count.
            if has_odd_character_count(&prefix) && !search_str.starts_with(&prefix) {
                continue;
            }
            // Get standard account address. If multisig:
            let account_address_hex = if multisig {
                println!("Multisig account address: 0x{}", search_str);
                // Re-derive account address here rather than cloning during search.
                hex::encode(auth_key_bytes_vec(&private_key))
            } else {
                // If not multisig, standard account address is from search.
                search_str
            };
            println!("Standard account address: 0x{}", account_address_hex);
            println!(
                "Private key:              0x{}\n",
                hex::encode(private_key.to_bytes())
            );
            // Transmit match message back to main thread.
            match_tx.send(())?;
        }
    }
}

/// Parses arguments, starts a timer, then spawns parallel search threads. Once search threads have
/// transmitted back enough match messages, broadcasts an exit transaction and prints elapsed time.
fn main() -> Result<()> {
    let start_time = Instant::now();
    let args = parse_args()?;
    println!(
        "\nStarting search at {:#?}\n",
        DateTime::<Local>::from(SystemTime::now())
    );

    // Initialize message channels for match and exit messages.
    let (match_tx, match_rx) = channel::unbounded::<()>();
    let (exit_tx, exit_rx) = channel::bounded::<()>(1);

    // Spawn parallel search threads.
    for _ in 0..args.threads {
        // Locally clone arguments not implementing copy trait so they can be moved into closure.
        let match_tx = match_tx.clone();
        let exit_rx = exit_rx.clone();
        let prefix = args.prefix.clone();
        thread::spawn(move || {
            if let Err(e) = generate_key(prefix, args.multisig, match_tx, exit_rx) {
                println!("Error: {}, in thread: {:?}", e, thread::current().id());
            }
        });
    }

    // Stop search after the desired number of addresses have been generated.
    for _ in 0..args.count {
        match_rx.recv()?;
    }
    exit_tx.send(())?;

    println!("Elapsed time: {:#?}\n", start_time.elapsed());
    Ok(())
}
