use anyhow::{bail, Result};
use clap::Parser;
use ed25519_dalek::SigningKey;
use num::{BigInt, FromPrimitive};
use regex::Regex;
use sha3::{Digest, Sha3_256};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering::Relaxed},
        Arc,
    },
    thread::{self, available_parallelism},
    time::{Duration, Instant},
};

/// Multisig account generation is assumed to take place in first transaction of standard account.
const SEQUENCE_NUMBER_MULTISIG: u64 = 0;

/// Optivanity: hyper-parallelized vanity address generator for the Aptos blockchain, brought to you
/// by Econia Labs
#[derive(Parser, Debug)]
struct CliArgs {
    /// Address prefix to match (no leading `0x`). Each additional character slows search by 16x.
    #[arg(short, long)]
    prefix: Option<String>,
    /// Address suffix to match. Each additional character slows search by 16x.
    #[arg(short, long)]
    suffix: Option<String>,
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
fn auth_key_bytes_vec(private_key_ref: &SigningKey) -> Vec<u8> {
    // Get public key from private key
    let mut public = Into::<ed25519_dalek::VerifyingKey>::into(private_key_ref)
        .to_bytes()
        .to_vec();
    // Push 0 which is the schema identifier for Ed25519 in the Aptos codebase
    public.push(0);
    // Hash the result and return the hash
    let mut h = Sha3_256::new();
    h.update(public);
    h.finalize().to_vec()
}

/// Creates a multisig account address.
/// Code inspired by the Aptos codebase.
/// This is reimplemented to avoid adding Aptos as a dependency which adds over 200 other crates.
/// See https://github.com/aptos-labs/aptos-core/blob/e2c8a6726a9bc4be464a755b47a113771a89e72c/types/src/account_address.rs#L239
pub fn create_multisig_account_address(mut creator: Vec<u8>, creator_nonce: u64) -> Vec<u8> {
    let mut full_seed = vec![];
    full_seed.extend(b"aptos_framework::multisig_account");
    full_seed.extend(creator_nonce.to_le_bytes());
    creator.extend(full_seed);
    creator.push(255);
    let mut h = Sha3_256::new();
    h.update(creator);
    h.finalize().to_vec()
}

/// Parse command line arguments, verifying hex characters and specified thread count.
fn parse_args() -> Result<CliArgs> {
    let mut args = CliArgs::parse();

    let r = Regex::new(r"^[0-9a-fA-F]+$").unwrap();

    // Verify prefix has valid hex characters.
    if !args
        .prefix
        .clone()
        .map_or(true, |prefix| r.is_match(&prefix))
    {
        bail!(
            "prefix '{}' is not a valid hex prefix",
            &args.prefix.unwrap()
        );
    }

    // Verify prefix has valid hex characters.
    if !args
        .suffix
        .clone()
        .map_or(true, |suffix| r.is_match(&suffix))
    {
        bail!(
            "suffix '{}' is not a valid hex suffix",
            &args.suffix.unwrap()
        );
    }

    args.prefix = args.prefix.map(|e| e.to_lowercase());
    args.suffix = args.suffix.map(|e| e.to_lowercase());

    Ok(args)
}

/// Return `true` if string slice reference indicates a string with an odd number of characters.
fn has_odd_character_count(string_ref: &str) -> bool {
    string_ref.len() % 2 == 1
}

/// Convert a char to what hex number it represents.
fn to_byte(c: char) -> u8 {
    match c {
        '0' => 0x0,
        '1' => 0x1,
        '2' => 0x2,
        '3' => 0x3,
        '4' => 0x4,
        '5' => 0x5,
        '6' => 0x6,
        '7' => 0x7,
        '8' => 0x8,
        '9' => 0x9,
        'a' => 0xa,
        'b' => 0xb,
        'c' => 0xc,
        'd' => 0xd,
        'e' => 0xe,
        'f' => 0xf,
        _ => panic!(),
    }
}

/// Generate a private key corresponding to a vanity prefix, while search is ongoing.
///
/// Once a match is found, a match message is transmitted to the main thread. Once the main thread
/// has received sufficient match transactions, it will exit making all the other threads stop.
///
/// # Arguments
///
/// * `prefix` - The vanity prefix to search against
/// * `prefix` - The vanity suffix to search against
/// * `multisig` - If `true` search for a multisig address
/// * `match_tx` - Transmit channel for match message sent to main thread when a match is found
/// * `counter` - Atomic integer that keeps track of the total number of addresses generated
fn generate_key(
    prefix: Option<String>,
    suffix: Option<String>,
    multisig: bool,
    match_tx: std::sync::mpsc::Sender<(String, String, Option<String>)>,
    counter: Arc<AtomicU64>,
) -> Result<()> {
    // Translate prefix string to bytes
    let prefix = if let Some(s) = prefix {
        Some(if has_odd_character_count(&s) {
            let c = s.chars().last().unwrap();
            (hex::decode(s[..s.len() - 1].to_string())?, Some(to_byte(c)))
        } else {
            (hex::decode(s.to_string())?, None)
        })
    } else {
        None
    };

    // Translate suffix string to bytes
    let suffix = if let Some(s) = suffix {
        Some(if has_odd_character_count(&s) {
            let c = s.chars().next().unwrap();
            (hex::decode(s[1..].to_string())?, Some(to_byte(c)))
        } else {
            (hex::decode(s.to_string())?, None)
        })
    } else {
        None
    };

    // Randomly generate private keys in a loop and check match against prefix bytes.
    let mut rng = rand::rngs::OsRng;
    loop {
        // Generate a private key and from it, bytes to compare against prefix bytes.
        let private_key = SigningKey::generate(&mut rng);
        let account_address_bytes = auth_key_bytes_vec(&private_key);
        let search_bytes = if multisig {
            create_multisig_account_address(account_address_bytes, SEQUENCE_NUMBER_MULTISIG)
        } else {
            account_address_bytes
        };

        // Increment generated addresses counter
        counter.fetch_add(1, Relaxed);

        // Check prefix match
        if let Some((pb, pc)) = &prefix {
            if !search_bytes.starts_with(pb) {
                continue;
            }
            if let Some(pc) = pc {
                if !(search_bytes[pb.len()] >> 4 == *pc) {
                    continue;
                }
            }
        }
        // Check suffix match
        if let Some((sb, sc)) = &suffix {
            if !search_bytes.ends_with(sb) {
                continue;
            }
            if let Some(sc) = sc {
                if !(search_bytes[search_bytes.len() - sb.len() - 1] & 0x0f == *sc) {
                    continue;
                }
            }
        }

        // Send match
        let str = hex::encode(search_bytes);
        let pk = hex::encode(private_key.to_bytes());
        if multisig {
            match_tx.send((hex::encode(auth_key_bytes_vec(&private_key)), pk, Some(str)))?;
        } else {
            match_tx.send((str, pk, None))?;
        }
    }
}

/// Parses arguments, starts a timer, then spawns parallel search threads. Once search threads have
/// transmitted back enough match messages, broadcasts an exit transaction and prints elapsed time.
fn main() -> Result<()> {
    let args = parse_args()?;
    let start_time = Instant::now();

    // Initialize message channels for match and exit messages.
    let (match_tx, match_rx) = std::sync::mpsc::channel::<(String, String, Option<String>)>();

    let count = Arc::new(AtomicU64::new(0));

    // Spawn parallel search threads.
    for _ in 0..args.threads {
        // Locally clone arguments not implementing copy trait so they can be moved into closure.
        let match_tx = match_tx.clone();
        let prefix = args.prefix.clone();
        let suffix = args.suffix.clone();
        let count = count.clone();
        thread::spawn(move || {
            if let Err(e) = generate_key(prefix, suffix, args.multisig, match_tx, count) {
                println!("Error: {}, in thread: {:?}", e, thread::current().id());
            }
        });
    }

    let bar = indicatif::ProgressBar::new_spinner();

    let bar2 = bar.clone();
    let count2 = count.clone();
    thread::spawn(move || {
        // Chance of getting the right address each time a guess is made
        let chance = BigInt::from_u8(16).unwrap();
        let chance = chance
            .pow((args.prefix.map_or(0, |e| e.len()) + args.suffix.map_or(0, |e| e.len())) as u32);

        // Number of addresses to generate
        let n_guesses_needed = args.count.clone();

        let mut buf = vec![];
        let mut first = true;
        let mut prev_count = 0;

        loop {
            thread::sleep(Duration::from_millis(100));
            let current_count = count2.load(Relaxed);
            let it_per_s = (current_count - prev_count) * 10;
            bar2.set_message(format!("Iterations per second: {} it/s", it_per_s));
            bar2.tick();

            // Store 5 it/s speeds, average that, then calculate the estimated amount of time
            if buf.len() < 5 {
                buf.push(it_per_s);
            } else if first {
                first = !first;
                let average = BigInt::from_u64(buf.iter().sum::<u64>() / 5).unwrap();
                let average_per_minute = average * 60;
                let estimate: BigInt = chance.clone() * n_guesses_needed / average_per_minute;
                bar2.suspend(|| {
                    println!("Estimate: {} minutes", estimate);
                    println!();
                })
            }

            prev_count = current_count;
        }
    });

    // Stop search after the desired number of addresses have been generated.
    for _ in 0..args.count {
        let (addr, pk, multi) = match_rx.recv()?;
        bar.suspend(|| {
            if let Some(multi) = multi {
                println!("Multisig account address: 0x{}", multi);
                println!("Standard account address: 0x{}", addr);
                println!("Private key:              0x{}", pk);
                println!();
            } else {
                println!("Standard account address: 0x{}", addr);
                println!("Private key:              0x{}", pk);
                println!();
            }
        });
    }

    bar.finish_and_clear();

    println!("Elapsed time: {:#?}", start_time.elapsed());
    println!("Total addresses generated: {}", count.load(Relaxed));
    Ok(())
}
