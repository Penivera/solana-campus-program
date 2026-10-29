use account_viewer::{
    describe_account, describe_account_entry, format_comparison_summary, format_sol,
    load_account_ids_from_file, parse_account_ids_from_csv, parse_rpc_response, validate_address,
    AccountInfo,
};
use serde_json::json;

const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

#[test]
fn formats_lamports_without_floating_point_rounding() {
    assert_eq!(format_sol(1_500_000_001), "1.500000001");
    assert_eq!(format_sol(0), "0");
    assert_eq!(format_sol(1_000_000_000), "1");
    assert_eq!(format_sol(1_050_000_000), "1.05");
    assert_eq!(format_sol(10), "0.00000001");
    assert_eq!(format_sol(17_773_281_124), "17.773281124");
}

#[test]
fn rejects_malformed_addresses_before_making_rpc_request() {
    let err = validate_address("not-an-address").unwrap_err();
    assert!(err.contains("Invalid address"));

    assert!(validate_address("123").is_err());
    assert!(validate_address("").is_err());
    // Characters '0', 'O', 'I', 'l' are not part of base58
    assert!(validate_address("00000000000000000000000000000000").is_err());
    assert!(validate_address("OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO").is_err());
    assert!(validate_address("IIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII").is_err());
    assert!(validate_address("llllllllllllllllllllllllllllllll").is_err());
    // Exceeding 44 characters
    assert!(validate_address("111111111111111111111111111111111111111111111").is_err());

    // Valid base58 addresses of lengths 32 and 44
    assert!(validate_address(SYSTEM_PROGRAM).is_ok());
    assert!(validate_address("5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd").is_ok());
}

#[test]
fn reads_a_valid_rpc_account_response() {
    let raw_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "context": { "slot": 100 },
            "value": {
                "lamports": 1_500_000_001u64,
                "owner": SYSTEM_PROGRAM,
                "executable": false
            }
        }
    });

    let account = parse_rpc_response(&raw_response).expect("Failed to parse valid RPC response");
    assert_eq!(
        account,
        Some(AccountInfo {
            lamports: 1_500_000_001,
            owner: SYSTEM_PROGRAM.to_string(),
            executable: false,
        })
    );

    let output = describe_account(SYSTEM_PROGRAM, account.as_ref());
    assert!(output.contains("1.500000001 SOL"));
    assert!(output.contains("Owner program: 11111111111111111111111111111111"));
    assert!(output.contains("Executable: no"));
    assert!(output.contains("Cluster: devnet"));
    assert!(output.contains(
        "https://explorer.solana.com/address/11111111111111111111111111111111?cluster=devnet"
    ));
}

#[test]
fn explains_an_account_with_no_onchain_data() {
    let raw_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "context": { "slot": 100 },
            "value": null
        }
    });

    let account = parse_rpc_response(&raw_response).expect("Parsing null value should succeed");
    assert_eq!(account, None);

    let output = describe_account(SYSTEM_PROGRAM, account.as_ref());
    assert!(output.contains("Account: no onchain data found yet"));
    assert!(output.contains("Cluster: devnet"));
}

#[test]
fn describes_invalid_account_entry_like_valid_except_flagged_invalid() {
    let output = describe_account_entry(
        "not-an-address",
        Err("Invalid address: enter a Solana public address (base58, 32–44 characters)."),
    );
    assert!(output.contains("Cluster: devnet"));
    assert!(output.contains("Address: not-an-address"));
    assert!(output.contains("https://explorer.solana.com/address/not-an-address?cluster=devnet"));
    assert!(output.contains("Account: invalid address (Invalid address: enter a Solana public address (base58, 32–44 characters).)"));
}

#[test]
fn handles_rpc_error_response() {
    let raw_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32600,
            "message": "Invalid request"
        }
    });

    let result = parse_rpc_response(&raw_response);
    assert_eq!(result, Err("RPC error: Invalid request".to_string()));
}

#[test]
fn formats_comparison_summary_properly() {
    let program_info = AccountInfo {
        lamports: 1,
        owner: "NativeLoader1111111111111111111111111111111".to_string(),
        executable: true,
    };
    let wallet_info = AccountInfo {
        lamports: 5_000_000_000,
        owner: SYSTEM_PROGRAM.to_string(),
        executable: false,
    };

    let program_res: Result<Option<AccountInfo>, String> = Ok(Some(program_info));
    let wallet_res: Result<Option<AccountInfo>, String> = Ok(Some(wallet_info));
    let unallocated_res: Result<Option<AccountInfo>, String> = Ok(None);
    let error_res: Result<Option<AccountInfo>, String> = Err("Invalid address".to_string());

    let results = vec![
        ("11111111111111111111111111111111", &program_res),
        ("5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd", &wallet_res),
        (
            "6wE29RaZgANhAr24QKntqQf2jmrvbH9WzmQ8hwdLHmas",
            &unallocated_res,
        ),
        ("invalid-addr", &error_res),
    ];

    let summary = format_comparison_summary(&results);
    assert!(summary.contains("Executable programs: 1"));
    assert!(summary.contains("Data accounts: 1"));
    assert!(summary.contains("Unallocated / No data: 1"));
    assert!(summary.contains("Invalid addresses: 1"));
}

#[test]
fn parses_account_ids_from_csv_without_header() {
    let csv = "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd\n11111111111111111111111111111111\n";
    let ids = parse_account_ids_from_csv(csv);
    assert_eq!(
        ids,
        vec![
            "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd",
            "11111111111111111111111111111111"
        ]
    );
}

#[test]
fn parses_account_ids_from_csv_with_header() {
    let csv = "account_id\n5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd\n11111111111111111111111111111111";
    let ids = parse_account_ids_from_csv(csv);
    assert_eq!(
        ids,
        vec![
            "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd",
            "11111111111111111111111111111111"
        ]
    );
}

#[test]
fn parses_account_ids_from_csv_with_quotes_and_spaces() {
    let csv = " \"account_id\" \n \"5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd\" \n '11111111111111111111111111111111' \n\n";
    let ids = parse_account_ids_from_csv(csv);
    assert_eq!(
        ids,
        vec![
            "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd",
            "11111111111111111111111111111111"
        ]
    );
}

#[test]
fn parses_account_ids_from_csv_comma_separated() {
    let csv = "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd, 11111111111111111111111111111111";
    let ids = parse_account_ids_from_csv(csv);
    assert_eq!(
        ids,
        vec![
            "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd",
            "11111111111111111111111111111111"
        ]
    );
}

#[test]
fn parses_account_ids_from_multi_column_csv() {
    let csv = "label,account_id,active\nwallet,5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd,true\nsystem,11111111111111111111111111111111,true\n";
    let ids = parse_account_ids_from_csv(csv);
    assert_eq!(
        ids,
        vec![
            "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd",
            "11111111111111111111111111111111"
        ]
    );
}

#[test]
fn loads_account_ids_from_file_and_handles_missing_file() {
    let temp_file = std::env::temp_dir().join("test_accounts_fixture.csv");
    let content = "account_id\n5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd\n11111111111111111111111111111111\n";
    std::fs::write(&temp_file, content).unwrap();

    let loaded = load_account_ids_from_file(temp_file.to_str().unwrap()).unwrap();
    assert_eq!(
        loaded,
        vec![
            "5ggTuwYT92FqEdBJqDKPMgq6QfAp7ihsvUpgFmq47KHd",
            "11111111111111111111111111111111"
        ]
    );

    let _ = std::fs::remove_file(temp_file);

    let err = load_account_ids_from_file("non_existent_file_path_12345.csv").unwrap_err();
    assert!(err.contains("Failed to read file"));
}
