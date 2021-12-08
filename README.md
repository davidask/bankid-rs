# bankid-rs

A [BankID](https://www.bankid.com) client for Rust based on [reqwest](https://github.com/seanmonstar/reqwest).

## Example

This package is uses [Tokio](https://tokio.rs). Add `bankid` to your `Cargo.toml` dependencies.

```toml
[dependencies]
bankid = { version = "0.1.0" }
```

```rust
use bankid::{
    Client, Endpoint, PersonalNumber,
    request::{AuthRequest}
};
use std::net::{IpAddr, Ipv4Addr};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(Endpoint::Test);

    let auth_response = client.auth(AuthRequest {
        end_user_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        personal_number: Some(PersonalNumber::parse("198710105080")?),
        requirement: None
    }).await?;

    let collect_response = client.collect(auth_response.order_ref).await?;

    client.cancel(auth_response.order_ref).await?;

    Ok(())
}
```
