use serde::{Deserialize, Serialize};

pub const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AccountInfo {
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
}

pub fn validate_address(address: &str) -> Result<&str, String> {
    if address.len() < 32
        || address.len() > 44
        || !address.chars().all(
            |c| matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z'),
        )
    {
        return Err(
            "Invalid address: enter a Solana public address (base58, 32–44 characters)."
                .to_string(),
        );
    }
    Ok(address)
}

pub fn format_sol(lamports: u64) -> String {
    let whole = lamports / LAMPORTS_PER_SOL;
    let rem = lamports % LAMPORTS_PER_SOL;
    if rem == 0 {
        whole.to_string()
    } else {
        let fraction = format!("{:09}", rem);
        let trimmed = fraction.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

pub fn parse_rpc_response(body: &serde_json::Value) -> Result<Option<AccountInfo>, String> {
    if let Some(err) = body.get("error") {
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("RPC error: {}", message));
    }

    let result = body
        .get("result")
        .ok_or_else(|| "RPC returned an unexpected response.".to_string())?;

    let value = result
        .get("value")
        .ok_or_else(|| "RPC returned an unexpected response.".to_string())?;

    if value.is_null() {
        return Ok(None);
    }

    serde_json::from_value::<AccountInfo>(value.clone())
        .map(Some)
        .map_err(|_| "RPC returned account data in an unexpected shape.".to_string())
}

pub fn parse_get_account_info_response(
    body: &serde_json::Value,
) -> Result<Option<AccountInfo>, String> {
    parse_rpc_response(body)
}

pub fn parse_get_multiple_accounts_response(
    body: &serde_json::Value,
) -> Result<Vec<Option<AccountInfo>>, String> {
    if let Some(err) = body.get("error") {
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("RPC error: {}", message));
    }

    let result = body
        .get("result")
        .ok_or_else(|| "RPC returned an unexpected response.".to_string())?;

    let value = result
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "RPC returned an unexpected response.".to_string())?;

    let mut accounts = Vec::with_capacity(value.len());
    for item in value {
        if item.is_null() {
            accounts.push(None);
        } else {
            let account: AccountInfo = serde_json::from_value(item.clone())
                .map_err(|_| "RPC returned account data in an unexpected shape.".to_string())?;
            accounts.push(Some(account));
        }
    }
    Ok(accounts)
}

pub fn get_account_info(address: &str, rpc_url: &str) -> Result<Option<AccountInfo>, String> {
    validate_address(address)?;

    let client = ureq::Agent::new_with_config(ureq::config::Config::default());
    let mut response = client
        .post(rpc_url)
        .header("content-type", "application/json")
        .send_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [address, { "encoding": "base64", "commitment": "confirmed" }]
        }))
        .map_err(|e| format!("RPC error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "RPC HTTP {}. Check your connection or try again later.",
            response.status().as_u16()
        ));
    }

    let body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("Failed to parse RPC response: {}", e))?;

    parse_rpc_response(&body)
}

pub fn get_multiple_accounts(
    addresses: &[&str],
    rpc_url: &str,
) -> Result<Vec<Option<AccountInfo>>, String> {
    if addresses.is_empty() {
        return Ok(Vec::new());
    }

    for address in addresses {
        validate_address(address)?;
    }

    let client = ureq::Agent::new_with_config(ureq::config::Config::default());
    let mut response = client
        .post(rpc_url)
        .header("content-type", "application/json")
        .send_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getMultipleAccounts",
            "params": [addresses, { "encoding": "base64", "commitment": "confirmed" }]
        }))
        .map_err(|e| format!("RPC error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "RPC HTTP {}. Check your connection or try again later.",
            response.status().as_u16()
        ));
    }

    let body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("Failed to parse RPC response: {}", e))?;

    parse_get_multiple_accounts_response(&body)
}

pub fn describe_account_entry(address: &str, result: Result<Option<&AccountInfo>, &str>) -> String {
    let mut lines = vec![
        "Cluster: devnet".to_string(),
        format!("Address: {}", address),
        format!(
            "Explorer: https://explorer.solana.com/address/{}?cluster=devnet",
            address
        ),
    ];

    match result {
        Ok(Some(account)) => {
            lines.push(format!(
                "Balance: {} lamports ({} SOL)",
                account.lamports,
                format_sol(account.lamports)
            ));
            lines.push(format!("Owner program: {}", account.owner));
            lines.push(format!(
                "Executable: {}",
                if account.executable { "yes" } else { "no" }
            ));
        }
        Ok(None) => {
            lines.push("Account: no onchain data found yet".to_string());
        }
        Err(err) => {
            lines.push(format!("Account: invalid address ({})", err));
        }
    }

    lines.join("\n")
}

pub fn describe_account(address: &str, account: Option<&AccountInfo>) -> String {
    describe_account_entry(address, Ok(account))
}

pub fn format_comparison_summary(
    results: &[(&str, &Result<Option<AccountInfo>, String>)],
) -> String {
    let mut programs = Vec::new();
    let mut data_accounts = Vec::new();
    let mut unallocated = Vec::new();
    let mut invalid = Vec::new();

    for (addr, res) in results {
        match res {
            Ok(Some(acc)) => {
                if acc.executable {
                    programs.push((*addr, acc));
                } else {
                    data_accounts.push((*addr, acc));
                }
            }
            Ok(None) => unallocated.push(*addr),
            Err(e) => invalid.push((*addr, e.as_str())),
        }
    }

    let mut lines = Vec::new();
    lines.push("Account Comparison Summary:".to_string());
    lines.push(format!("- Executable programs: {}", programs.len()));
    for (addr, acc) in &programs {
        lines.push(format!("  • {} (Owner: {}, executable)", addr, acc.owner));
    }
    lines.push(format!("- Data accounts: {}", data_accounts.len()));
    for (addr, acc) in &data_accounts {
        lines.push(format!(
            "  • {} (Balance: {} SOL, Owner: {})",
            addr,
            format_sol(acc.lamports),
            acc.owner
        ));
    }
    if !unallocated.is_empty() {
        lines.push(format!("- Unallocated / No data: {}", unallocated.len()));
        for addr in &unallocated {
            lines.push(format!("  • {}", addr));
        }
    }
    if !invalid.is_empty() {
        lines.push(format!("- Invalid addresses: {}", invalid.len()));
        for (addr, err) in &invalid {
            lines.push(format!("  • {}: {}", addr, err));
        }
    }
    lines.join("\n")
}

pub fn parse_account_ids_from_csv(content: &str) -> Vec<String> {
    let mut lines = content.lines().map(str::trim).filter(|l| !l.is_empty());
    let first_line = match lines.next() {
        Some(line) => line,
        None => return Vec::new(),
    };

    let first_row: Vec<&str> = first_line
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
        .collect();

    let header_idx = first_row.iter().position(|col| {
        let lower = col.to_ascii_lowercase();
        matches!(
            lower.as_str(),
            "account_id"
                | "account id"
                | "account"
                | "accounts"
                | "account_ids"
                | "address"
                | "addresses"
                | "pubkey"
                | "pubkeys"
                | "public_key"
                | "public key"
                | "id"
        )
    });

    let mut ids = Vec::new();

    if let Some(target_col) = header_idx {
        for line in lines {
            let cols: Vec<&str> = line
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
                .collect();
            if let Some(&val) = cols.get(target_col) {
                if !val.is_empty() {
                    ids.push(val.to_string());
                }
            }
        }
    } else {
        for val in first_row {
            if !val.is_empty() {
                ids.push(val.to_string());
            }
        }
        for line in lines {
            for val in line
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
            {
                if !val.is_empty() {
                    ids.push(val.to_string());
                }
            }
        }
    }

    ids
}

pub fn load_account_ids_from_file(path: &str) -> Result<Vec<String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
    Ok(parse_account_ids_from_csv(&content))
}
