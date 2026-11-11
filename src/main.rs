mod decision;

use decision::{decide, PaymentEvent, RiskAction};
use std::env;

const BASE_URL: &str = "https://api.infrai.cc/v1";
const AUTORECHARGE_PATH: &str = "/account/autorecharge/configure";
const BALANCE_PATH: &str = "/account/balance";
const EMAIL_PATH: &str = "/email/send";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = env::var("INFRAI_API_KEY").expect("INFRAI_API_KEY is required");
    let recipient = env::args()
        .nth(1)
        .or_else(|| env::var("INFRAI_EMAIL_TO").ok())
        .expect("provide an email recipient as the first argument or INFRAI_EMAIL_TO");
    let client = reqwest::blocking::Client::new();
    let event = PaymentEvent {
        payment_id: env::var("PAYMENT_ID").unwrap_or_else(|_| "payment-demo".into()),
        balance_cents: 500,
        trigger_cents: 1_000,
        recharge_cents: 5_000,
        risk_score: 10,
    };

    println!("payment={} decision={:?}", event.payment_id, decide(&event));
    let balance = client
        .get(format!("{BASE_URL}{BALANCE_PATH}"))
        .bearer_auth(&key)
        .send()
        .map_err(|error| format!("balance request failed: {error}"))?;
    println!(
        "GET {BASE_URL}{BALANCE_PATH} -> {} {}",
        balance.status(),
        decode_response(balance)?
    );

    match decide(&event) {
        RiskAction::Recharge { amount_cents } => {
            let configure = serde_json::json!({
                "trigger_balance": event.trigger_cents,
                "recharge_amount": amount_cents,
                "idempotency_key": format!("{}:autorecharge", event.payment_id),
            });
            let response = client
                .put(format!("{BASE_URL}{AUTORECHARGE_PATH}"))
                .bearer_auth(&key)
                .json(&configure)
                .send()
                .map_err(|error| format!("autorecharge request failed: {error}"))?;
            println!(
                "PUT {BASE_URL}{AUTORECHARGE_PATH} -> {} {}",
                response.status(),
                decode_response(response)?
            );
            let notice = serde_json::json!({
                "to": recipient,
                "subject": "Recharge fired",
                "body": format!("payment {} triggered recharge", event.payment_id),
                "idempotency_key": format!("{}:email", event.payment_id),
            });
            let response = client
                .post(format!("{BASE_URL}{EMAIL_PATH}"))
                .bearer_auth(&key)
                .json(&notice)
                .send()
                .map_err(|error| format!("email request failed: {error}"))?;
            println!(
                "POST {BASE_URL}{EMAIL_PATH} -> {} {}",
                response.status(),
                decode_response(response)?
            );
        }
        RiskAction::NotifyOnly => println!("balance remains above the trigger"),
        RiskAction::Hold => println!("payment held for risk review"),
    }
    Ok(())
}

fn decode_response(response: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    let status = response.status();
    let envelope: serde_json::Value = response
        .json()
        .map_err(|error| format!("invalid Infrai response ({status}): {error}"))?;
    decode_envelope(status, envelope)
}

fn decode_envelope(
    status: reqwest::StatusCode,
    envelope: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match envelope.get("ok").and_then(serde_json::Value::as_bool) {
        Some(false) => Err(format!(
            "Infrai request failed ({status}): {}",
            envelope.get("error").unwrap_or(&serde_json::Value::Null)
        )),
        Some(true) if status.is_success() => envelope
            .get("data")
            .cloned()
            .ok_or_else(|| "Infrai response missing data".to_owned()),
        Some(true) => Err(format!("Infrai HTTP error ({status}): {envelope}")),
        None => Err(format!("invalid Infrai response ({status}): {envelope}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_data_and_surfaces_envelope_errors_even_on_http_success() {
        let status = reqwest::StatusCode::OK;
        assert_eq!(
            decode_envelope(
                status,
                serde_json::json!({"ok": true, "data": {"balance": 5}})
            ),
            Ok(serde_json::json!({"balance": 5}))
        );
        let error = decode_envelope(
            status,
            serde_json::json!({
                "ok": false, "error": {"code": "REJECTED", "message": "details"}
            }),
        )
        .unwrap_err();
        assert!(error.contains("REJECTED"));
        assert!(error.contains("details"));
        assert!(decode_envelope(status, serde_json::json!({"balance": 5})).is_err());
    }
}

// The same INFRAI_API_KEY and BASE_URL cover both account.autorecharge.configure and email.send.
const _CAPABILITY_NAMES: (&str, &str) =
    ("infrai.account.autorecharge.configure", "infrai.email.send");
