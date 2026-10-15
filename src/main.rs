use std::env;
use std::process::ExitCode;

use account_viewer::{
    describe_account_entry, format_comparison_summary, get_multiple_accounts, validate_address,
    AccountInfo, DEFAULT_RPC_URL,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("Usage: account-viewer <ADDRESS_1> <ADDRESS_2> [ADDITIONAL_ADDRESSES...]");
        println!();
        println!("Inspect two or more Solana devnet accounts by public address.");
        return ExitCode::SUCCESS;
    }

    if args.len() < 2 {
        eprintln!("Usage: account-viewer <ADDRESS_1> <ADDRESS_2> [ADDITIONAL_ADDRESSES...]");
        eprintln!("Error: At least two public addresses are required.");
        return ExitCode::FAILURE;
    }

    let rpc_url = env::var("SOLANA_DEVNET_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());

    // Step 1: Pre-validate all addresses before making RPC request
    let mut valid_indices = Vec::new();
    let mut valid_addresses = Vec::new();
    let mut results: Vec<Result<Option<AccountInfo>, String>> = Vec::with_capacity(args.len());

    for (i, address) in args.iter().enumerate() {
        match validate_address(address) {
            Ok(valid) => {
                valid_indices.push(i);
                valid_addresses.push(valid);
                results.push(Ok(None)); // Placeholder
            }
            Err(err) => {
                results.push(Err(err));
            }
        }
    }

    // Step 2: Fetch all valid addresses in a single batch RPC request
    if !valid_addresses.is_empty() {
        match get_multiple_accounts(&valid_addresses, &rpc_url) {
            Ok(accounts) => {
                for (val_idx, account_opt) in valid_indices.iter().zip(accounts) {
                    results[*val_idx] = Ok(account_opt);
                }
            }
            Err(err) => {
                for val_idx in valid_indices {
                    results[val_idx] = Err(format!("Batch RPC error: {}", err));
                }
            }
        }
    }

    // Step 3: Print valid accounts first
    let mut valid_entries = Vec::new();
    let mut invalid_entries = Vec::new();

    for (address, result) in args.iter().zip(&results) {
        if result.is_ok() {
            valid_entries.push((address.as_str(), result));
        } else {
            invalid_entries.push((address.as_str(), result));
        }
    }

    let mut is_first = true;

    for (address, result) in &valid_entries {
        if !is_first {
            println!();
        }
        let res_ref = result
            .as_ref()
            .map(|opt| opt.as_ref())
            .map_err(|e| e.as_str());
        println!("{}", describe_account_entry(address, res_ref));
        is_first = false;
    }

    // Step 4: Towards the end, print invalid accounts formatted like valid, except flagged as invalid
    for (address, result) in &invalid_entries {
        if !is_first {
            println!();
        }
        let res_ref = result
            .as_ref()
            .map(|opt| opt.as_ref())
            .map_err(|e| e.as_str());
        println!("{}", describe_account_entry(address, res_ref));
        is_first = false;
    }

    // Step 5: Print comparison summary between programs and data accounts
    println!();
    let summary_input: Vec<(&str, &Result<Option<AccountInfo>, String>)> =
        args.iter().map(|s| s.as_str()).zip(&results).collect();
    println!("{}", format_comparison_summary(&summary_input));

    if !invalid_entries.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
