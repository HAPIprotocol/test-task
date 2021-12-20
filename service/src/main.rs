use anyhow::{anyhow, bail, Context, Result};
use log::info;
use reqwest::{header, Client};
use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const CONFIGURATION_ENV: &'static str = "CFG_PATH";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub cmc_api_key: String,
    pub contract_id: String,
    pub signer_account_id: String,
}

impl Config {
    pub fn from_toml<T: std::clone::Clone + AsRef<Path>>(path: T) -> Result<Self> {
        info!("Loading configuration from toml file");
        let mut f = OpenOptions::new()
            .read(true)
            .open(path.clone())
            .context(format!("Path to toml file: {}", path.as_ref().display()))?;
        let mut buffer = vec![];
        f.read_to_end(&mut buffer)?;
        let config = toml::from_slice::<Self>(&buffer[..])
            .context("While parsing configuration from toml file.")?;
        config.is_valid()?;
        Ok(config)
    }

    pub fn is_valid(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResponseBody {
    data: DataBody,
}

impl ResponseBody {
    pub fn price(&self) -> &f64 {
        &self.data.quote.usd.price
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DataBody {
    quote: QuoteBody,
}

#[derive(Deserialize, Debug, Clone)]
pub struct QuoteBody {
    #[serde(rename = "USD")]
    usd: CurrencyBody,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrencyBody {
    price: f64,
}

fn near_login() -> Result<()> {
    let cmd_output = Command::new("near")
        .args(["login"])
        .output()
        .expect("failed to execute near-cli");

    if cmd_output.status.success() {
        std::io::stdout()
            .write_all(&cmd_output.stdout)
            .context("Error on trying write to stdout")
            .unwrap();
    } else {
        bail!("Error on command 'near login': {}", unsafe {
            std::str::from_utf8_unchecked(&cmd_output.stderr)
        })
    };
    Ok(())
}

async fn init_req_client(api_token: &str) -> Result<Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "X-CMC_PRO_API_KEY",
        header::HeaderValue::from_str(api_token)
            .context("Invalid X-CMC_PRO_API_KEY header value")?,
    );
    headers.insert(
        "Host",
        header::HeaderValue::from_static("pro-api.coinmarketcap.com"),
    );
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Accept-Encoding",
        header::HeaderValue::from_static("deflate, gzip"),
    );

    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .gzip(true)
        .deflate(true)
        .build()?)
}

async fn get_bitcoin_price(client: &Client) -> Result<f64> {
    let response = client
        .post("http://pro-api.coinmarketcap.com/v1/tools/price-conversion")
        .query(&[("symbol", "BTC"), ("amount", "1")])
        .send()
        .await?;
    if response.status().is_success() {
        let body: ResponseBody = response.json().await?;
        Ok(*body.price())
    } else {
        let err = anyhow!(
            "Error status: {} with body:\n{}",
            response.status(),
            response.json::<serde_json::Value>().await?
        );
        Err(err)?
    }
}

fn near_set_last_price(price: f64, contract_id: &str, signer_id: &str) -> Result<()> {
    let cmd_output = Command::new("near")
        .args([
            "call",
            contract_id,
            "set_last_price",
            &format!("'{{\"price\":{}}}'", price),
            "--accountId",
            signer_id,
        ])
        .output()
        .expect("failed to execute near-cli");

    if cmd_output.status.success() {
        unsafe { std::str::from_utf8_unchecked(&cmd_output.stdout) }
            .lines()
            .for_each(|line| info!("{}", line));
    } else {
        bail!("Error on command 'near call set_last_price': {}", unsafe {
            std::str::from_utf8_unchecked(&cmd_output.stderr)
        })
    };
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _logger_guard = flexi_logger::Logger::try_with_env_or_str("info")
        .unwrap()
        .start()
        .unwrap();
    let cfg_path = std::env::var(CONFIGURATION_ENV)
        .expect(&format!("Environment '{}' did not set", CONFIGURATION_ENV));
    let cfg = Config::from_toml(cfg_path).unwrap();
    near_login()?;
    let client = init_req_client(&cfg.cmc_api_key).await?;
    loop {
        let current_price = get_bitcoin_price(&client).await?;
        info!("Current BTC price = {}", &current_price);
        near_set_last_price(current_price, &cfg.contract_id, &cfg.signer_account_id).unwrap();
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}
