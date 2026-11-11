#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskAction {
    Recharge { amount_cents: u64 },
    NotifyOnly,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentEvent {
    pub payment_id: String,
    pub balance_cents: u64,
    pub trigger_cents: u64,
    pub recharge_cents: u64,
    pub risk_score: u8,
}

pub fn decide(event: &PaymentEvent) -> RiskAction {
    if event.risk_score >= 80 {
        return RiskAction::Hold;
    }
    if event.balance_cents <= event.trigger_cents {
        return RiskAction::Recharge {
            amount_cents: event.recharge_cents,
        };
    }
    RiskAction::NotifyOnly
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_balance_recharges_but_high_risk_holds() {
        let low = PaymentEvent {
            payment_id: "pay-42".into(),
            balance_cents: 500,
            trigger_cents: 1_000,
            recharge_cents: 5_000,
            risk_score: 10,
        };
        assert_eq!(decide(&low), RiskAction::Recharge { amount_cents: 5_000 });

        let risky = PaymentEvent { risk_score: 90, ..low };
        assert_eq!(decide(&risky), RiskAction::Hold);
    }
}
