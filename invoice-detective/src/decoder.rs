use anyhow::{anyhow, bail, Result};
use lightning::offers::offer::Offer;
use lightning::offers::refund::Refund;
use lightning_invoice::Bolt11Invoice;
use lnurl::lightning_address::LightningAddress;
use lnurl::lnurl::LnUrl;
use lnurl::pay::LnURLPayInvoice;
use lnurl::{decode_ln_url_response, LnUrlResponse};
use std::io;
use std::io::Write;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DecodedData {
    Invoice(Bolt11Invoice),
    Offer(Offer),
	Refund(Refund),
    LightningAddress(LightningAddress),
    LnUrl(LnUrl),
}

pub fn decode(input: &str) -> Result<DecodedData> {
    println!("    Input: {input}");
    let input = input.trim().to_lowercase();
    let input = input.strip_prefix("lightning:").unwrap_or(&input);
    println!("Sanitized: {input}");
    let filtered_input: String = input
        .chars()
        .filter(|c| *c != '+' && !c.is_whitespace())
        .collect();

    let decoded_data = if input.contains('@') {
        println!("Decoding as a lightning address");
        let address = LightningAddress::from_str(input)?;
        DecodedData::LightningAddress(address)
    } else if input.starts_with("lnurl") {
        // TODO: Support LUD-17: Protocol schemes and raw (non bech32-encoded) URLs.
        println!("Decoding as LNURL");
        let lnurl = LnUrl::from_str(input)?;
        DecodedData::LnUrl(lnurl)
    } else if filtered_input.starts_with("lno") {
        println!("Decoding as BOLT12 offer");
        let offer = Offer::from_str(input).map_err(|e| anyhow!("{e:?}"))?;
        DecodedData::Offer(offer)
    } else if filtered_input.starts_with("lnr") {
        println!("Decoding as BOLT12 refund (naked invoice request)");
		let refund = Refund::from_str(input).map_err(|e| anyhow!("{e:?}"))?;
		DecodedData::Refund(refund)
    } else if input.starts_with("ln") {
        println!("Decoding as BOLT11 invoice");
        let invoice = input.parse::<Bolt11Invoice>()?;
        DecodedData::Invoice(invoice)
    } else {
        // TODO: Support BIP-21.
        bail!("Input is not recognized");
    };
    Ok(decoded_data)
}

pub async fn resolve_lnurl(lnurl: LnUrl) -> Result<String> {
    println!("Quering {}", lnurl.url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client.get(&lnurl.url).send().await?;
    let text = response.error_for_status()?.text().await?;
    println!("Response: {text}");
    print!("Decoding as JSON: ");
    let _ = io::stdout().flush();
    let response = decode_ln_url_response(&text)?;
    println!("OK");

    let pay = match response {
        LnUrlResponse::LnUrlPayResponse(pay_response) => pay_response,
        LnUrlResponse::LnUrlWithdrawResponse(_) => bail!("LNURL Withdraw"),
        LnUrlResponse::LnUrlChannelResponse(_) => bail!("LNURL channel request"),
    };

    let symbol = if pay.callback.contains('?') { '&' } else { '?' };
    let url = format!("{}{symbol}amount={}", pay.callback, pay.min_sendable);
    println!("Quering {url}");
    let response = client.get(&url).send().await?;
    let text = response.error_for_status()?.text().await?;
    println!("Response: {text}");
    print!("Decoding as JSON: ");
    let _ = io::stdout().flush();
    let json: serde_json::Value = serde_json::from_str(&text)?;
    println!("OK");
    print!("Decoding as LNURL pay invoice response: ");
    let _ = io::stdout().flush();
    let reponse: LnURLPayInvoice = serde_json::from_value(json)?;
    println!("OK");
    Ok(reponse.pr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let d = decode("satoshi@bitcoin.org").unwrap();
        println!("{d:?}");

        let d = decode("LNURL1DP68GURN8GHJ7MRWW4EXCTNXD9SHG6NPVCHXXMMD9AKXUATJDSKHQCTE8AEK2UMND9HKU0FJ89JXXCT989JRGVE3XVMK2ERZXPJX2DECXP3KXV33XQCKVE3C8QMXXD3CVVUXXEPNV3NRWE3HXVUKZWP3XSEX2V3CXEJXGCNRXGUKGUQ0868").unwrap();
        println!("{d:?}");

        let d = decode("lntb10u1pjkvq6mpp5zszjfrehd5y8sq4w47jegjy5xglw3smcfelfkqud56vtq9c48kmsdqqcqzzsxqyz5vqsp5kgjy259sn4t24er4hawcsr9zl9u7vrkdk7a9kcs9ffury0kf50cq9qyyssqept74lw02kkng3cpzqhyrwt542ct6dtfcz7mtesfggt57r5j7djyz7z5de4cyaupehhwyv7ql6yatqe3e4hvnp2lvpvdwxstpy2rnwqq89p90d").unwrap();
        println!("{d:?}");
    }
}
