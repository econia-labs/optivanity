use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_keygen::KeyGen;
use aptos_types::{
    account_address::{create_multisig_account_address, AccountAddress},
    transaction::authenticator::AuthenticationKey,
};
use std::env;
use std::thread;
use std::time::Duration;

const PREFIX_ARG: usize = 1;
const N_THREADS_ARG: usize = 2;
const MULTISIG_ARG: usize = 3;
const MULTISIG_FLAG: &str = "-m";
const SEQUENCE_NUMBER: u64 = 0;

/// Derive authentication key bytes vector from a reference to a private key.
fn auth_key_bytes_vec(private_key_ref: &Ed25519PrivateKey) -> Vec<u8> {
    AuthenticationKey::ed25519(&Ed25519PublicKey::from(private_key_ref)).to_vec()
}

fn main() {
    // Get command arguments.
    let args: Vec<_> = env::args().collect();
    let mut check_str = args[PREFIX_ARG].clone(); // Get prefix string for format check.
    let is_odd_length = check_str.len() % 2 != 0; // Check if string has odd character count.
    if is_odd_length {
        // Append a 0 to check string if an odd number of hex characters.
        check_str.push('0')
    };
    if hex::decode(check_str).is_err() {
        println!("Vanity prefix must contain valid hex characters (0123456789abcdef).");
        return; // Exit if vanity prefix isn't specified in valid hex characters.
    }
    // Check if should search for multisig vanity address.
    let multisig = args.len() > MULTISIG_ARG && args[MULTISIG_ARG].as_str() == MULTISIG_FLAG;
    // Spawn parallel search threads.
    for _ in 0..args[N_THREADS_ARG].parse::<u64>().unwrap() {
        thread::spawn(move || {
            // Get random key generator.
            let mut key_generator = KeyGen::from_os_rng();
            // Get vanity prefix as string.
            let prefix_str = env::args().collect::<Vec<_>>()[PREFIX_ARG].clone();
            // Clone prefix for getting bytes to check against.
            let mut check_str = prefix_str.clone();
            // Pop last character if an odd length.
            if is_odd_length {
                check_str.pop();
            };
            // Convert resulting prefix to bytes for checking against. For odd-length prefixes this
            // approach truncates the final character, which must be checked via string compare
            // after a bytes match. This is done for performance (to minimize string operations).
            let prefix_bytes = hex::decode(check_str).unwrap();
            // Generate private keys, reporting for each match against vanity prefix.
            loop {
                let private_key = key_generator.generate_ed25519_private_key();
                // Assume account address is identical to authentication key (no rotation).
                let account_address_bytes = auth_key_bytes_vec(&private_key);
                // Get bytes to check against: if multisig, derive multisig account address.
                let search_bytes = if multisig {
                    // Note that for performance this moves (rather than clones) address bytes, such
                    // that account address must be re-derived from private key if a match is found.
                    create_multisig_account_address(
                        AccountAddress::from_bytes(account_address_bytes).unwrap(),
                        SEQUENCE_NUMBER,
                    )
                    .to_vec()
                } else {
                    account_address_bytes // If standard account, check against account address.
                };
                // If a match found, print diagnostics then keep searching.
                if search_bytes.starts_with(&prefix_bytes) {
                    // Get search bytes as a string.
                    let search_str = hex::encode(search_bytes);
                    // Verify full string match if prefix is an odd number of characters.
                    if is_odd_length && !search_str.starts_with(&prefix_str) {
                        continue;
                    }
                    // Get standard account address. If multisig:
                    let account_address_hex = if multisig {
                        println!("Multisig account address: 0x{}", search_str);
                        // Re-derive account address bytes here rather than cloning during search.
                        hex::encode(auth_key_bytes_vec(&private_key))
                    } else {
                        search_str // If not multisig, standard account address is from search.
                    };
                    println!("Standard account address: 0x{}", account_address_hex);
                    println!(
                        "Private key:              0x{}\n",
                        hex::encode(private_key.to_bytes())
                    );
                }
            }
        });
    }
    // Run infinite loop to prevent main thread from returning and closing search threads.
    loop {
        thread::sleep(Duration::from_secs(u64::MAX)); // Sleep to preserve resources.
    }
}
