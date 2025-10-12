use chrono::{DateTime, Datelike, TimeZone, Utc};
use derive_builder::{Builder, UninitializedFieldError};
use rust_decimal::{Decimal, prelude::*};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[cfg(feature = "json-schema")]
use schemars::JsonSchema;

use crate::{
    EffectiveDate,
    ledger::{CreditFacilityBalanceSummary, CreditFacilityProposalBalanceSummary},
    primitives::{
        CVLPct, CollateralizationState, DisbursedReceivableAccountCategory, PriceOfOneBTC,
        Satoshis, UsdCents,
    },
};

use super::error::TermsError;

const NUMBER_OF_DAYS_IN_YEAR: u64 = 365;
const SHORT_TERM_DURATION_MONTHS_THRESHOLD: u32 = 12;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(transparent)]
pub struct AnnualRatePct(Decimal);
#[cfg(feature = "graphql")]
async_graphql::scalar!(AnnualRatePct);

impl AnnualRatePct {
    pub fn interest_for_time_period(&self, principal: UsdCents, days: u32) -> UsdCents {
        let cents = principal.to_usd() * Decimal::from(days) * self.0
            / Decimal::from(NUMBER_OF_DAYS_IN_YEAR);

        UsdCents::from(
            cents
                .round_dp_with_strategy(0, RoundingStrategy::AwayFromZero)
                .to_u64()
                .expect("should return a valid integer"),
        )
    }
}

impl From<Decimal> for AnnualRatePct {
    fn from(value: Decimal) -> Self {
        AnnualRatePct(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(transparent)]
pub struct OneTimeFeeRatePct(Decimal);
#[cfg(feature = "graphql")]
async_graphql::scalar!(OneTimeFeeRatePct);

impl OneTimeFeeRatePct {
    pub const ZERO: Self = Self(dec!(0));

    pub fn new(pct: u64) -> Self {
        OneTimeFeeRatePct(Decimal::from(pct))
    }

    pub fn apply(&self, amount: UsdCents) -> UsdCents {
        let fee_as_decimal = (amount.to_usd() * (self.0 / dec!(100)))
            .round_dp_with_strategy(2, RoundingStrategy::AwayFromZero);

        UsdCents::try_from_usd(fee_as_decimal).expect("Unexpected negative number")
    }
}

impl From<Decimal> for OneTimeFeeRatePct {
    fn from(value: Decimal) -> Self {
        OneTimeFeeRatePct(value)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum FacilityDuration {
    Months(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacilityDurationType {
    LongTerm,
    ShortTerm,
}

impl From<FacilityDurationType> for DisbursedReceivableAccountCategory {
    fn from(duration_type: FacilityDurationType) -> Self {
        match duration_type {
            FacilityDurationType::LongTerm => DisbursedReceivableAccountCategory::LongTerm,
            FacilityDurationType::ShortTerm => DisbursedReceivableAccountCategory::ShortTerm,
        }
    }
}

impl FacilityDuration {
    fn maturity_date(&self, start_date: DateTime<Utc>) -> EffectiveDate {
        match self {
            FacilityDuration::Months(months) => start_date
                .checked_add_months(chrono::Months::new(*months))
                .expect("should return a maturity date")
                .into(),
        }
    }

    pub fn duration_type(&self) -> FacilityDurationType {
        match self {
            FacilityDuration::Months(months) => {
                if *months > SHORT_TERM_DURATION_MONTHS_THRESHOLD {
                    FacilityDurationType::LongTerm
                } else {
                    FacilityDurationType::ShortTerm
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ObligationDuration {
    Days(u64),
}

impl ObligationDuration {
    pub fn end_date(&self, start_date: EffectiveDate) -> EffectiveDate {
        match self {
            Self::Days(days) => start_date
                .checked_add_days(chrono::Days::new(*days))
                .expect("should return an end date"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct InterestPeriod {
    pub interval: InterestInterval,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl InterestPeriod {
    fn new(interval: InterestInterval, start: DateTime<Utc>) -> Self {
        let end = interval.end_date_starting_at(start);
        Self {
            interval,
            start,
            end,
        }
    }

    pub fn next(&self) -> Self {
        Self::new(self.interval, self.end + chrono::Duration::seconds(1))
    }

    pub fn truncate(&self, latest_possible_end_date: DateTime<Utc>) -> Option<Self> {
        if self.start > latest_possible_end_date {
            return None;
        }

        Some(Self {
            interval: self.interval,
            start: self.start,
            end: self.end.min(latest_possible_end_date),
        })
    }

    pub fn days(&self) -> u32 {
        self.end.day() - self.start.day() + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InterestInterval {
    EndOfMonth,
    EndOfDay,
}

impl InterestInterval {
    pub fn period_from(&self, start_date: DateTime<Utc>) -> InterestPeriod {
        InterestPeriod::new(*self, start_date)
    }

    fn end_date_starting_at(&self, current_date: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            InterestInterval::EndOfMonth => {
                let current_year = current_date.year();
                let current_month = current_date.month();

                let (year, month) = if current_month == 12 {
                    (current_year + 1, 1)
                } else {
                    (current_year, current_month + 1)
                };

                Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
                    .single()
                    .expect("should return a valid date time")
                    - chrono::Duration::seconds(1)
            }
            InterestInterval::EndOfDay => Utc
                .with_ymd_and_hms(
                    current_date.year(),
                    current_date.month(),
                    current_date.day(),
                    23,
                    59,
                    59,
                )
                .single()
                .expect("should return a valid date time"),
        }
    }
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[builder(build_fn(validate = "Self::validate", error = "TermsError"))]
pub struct TermValues {
    #[builder(setter(into))]
    pub annual_rate: AnnualRatePct,
    #[builder(setter(into))]
    pub duration: FacilityDuration,
    #[builder(setter(into))]
    pub interest_due_duration_from_accrual: ObligationDuration,
    #[builder(setter(into))]
    pub obligation_overdue_duration_from_due: Option<ObligationDuration>,
    #[builder(setter(into))]
    pub obligation_liquidation_duration_from_due: Option<ObligationDuration>,
    #[builder(setter(into))]
    pub accrual_cycle_interval: InterestInterval,
    #[builder(setter(into))]
    pub accrual_interval: InterestInterval,
    #[builder(setter(into))]
    pub one_time_fee_rate: OneTimeFeeRatePct,
    #[builder(setter(into))]
    pub liquidation_cvl: CVLPct,
    #[builder(setter(into))]
    pub margin_call_cvl: CVLPct,
    #[builder(setter(into))]
    pub initial_cvl: CVLPct,
    #[builder(default)]
    pub disburse_all_at_activation: bool,
}

impl TermValues {
    pub fn maturity_date(&self, start_date: DateTime<Utc>) -> EffectiveDate {
        self.duration.maturity_date(start_date)
    }

    pub fn is_disbursal_allowed(
        &self,
        balance: CreditFacilityBalanceSummary,
        amount: UsdCents,
        price: PriceOfOneBTC,
    ) -> bool {
        let cvl = balance
            .with_added_disbursal(amount)
            .outstanding_amount_cvl(price);
        cvl >= self.margin_call_cvl
    }

    pub fn is_proposal_completion_allowed(
        &self,
        balance: CreditFacilityProposalBalanceSummary,
        price: PriceOfOneBTC,
    ) -> bool {
        let total = balance.facility_amount_cvl(price);
        total >= self.margin_call_cvl
    }

    pub fn builder() -> TermValuesBuilder {
        TermValuesBuilder::default()
    }

    pub fn disburse_all_at_activation(&self) -> bool {
        self.disburse_all_at_activation
    }

    pub fn required_collateral(
        &self,
        desired_principal: UsdCents,
        price: PriceOfOneBTC,
    ) -> Satoshis {
        let collateral_value = self.initial_cvl.scale(desired_principal);
        price.cents_to_sats_round_up(collateral_value)
    }

    pub fn collateralization(&self, cvl: CVLPct) -> CollateralizationState {
        let margin_call_cvl = self.margin_call_cvl;
        let liquidation_cvl = self.liquidation_cvl;

        if cvl == CVLPct::Infinite {
            CollateralizationState::NoExposure
        } else if cvl == CVLPct::ZERO {
            CollateralizationState::NoCollateral
        } else if cvl >= margin_call_cvl {
            CollateralizationState::FullyCollateralized
        } else if cvl >= liquidation_cvl {
            CollateralizationState::UnderMarginCallThreshold
        } else {
            CollateralizationState::UnderLiquidationThreshold
        }
    }

    pub fn collateralization_update(
        &self,
        current_cvl: CVLPct,
        last_collateralization_state: CollateralizationState,
        upgrade_buffer_cvl_pct: Option<CVLPct>,
        liquidation_upgrade_blocked: bool,
    ) -> Option<CollateralizationState> {
        let calculated_collateralization = &self.collateralization(current_cvl);

        match (last_collateralization_state, *calculated_collateralization) {
            // Redundant same state changes
            (CollateralizationState::NoCollateral, CollateralizationState::NoCollateral)
            | (
                CollateralizationState::FullyCollateralized,
                CollateralizationState::FullyCollateralized,
            )
            | (
                CollateralizationState::UnderMarginCallThreshold,
                CollateralizationState::UnderMarginCallThreshold,
            )
            | (
                CollateralizationState::UnderLiquidationThreshold,
                CollateralizationState::UnderLiquidationThreshold,
            )
            | (CollateralizationState::NoExposure, CollateralizationState::NoExposure) => None,

            // Validated liquidation changes
            (CollateralizationState::UnderLiquidationThreshold, _) => {
                if liquidation_upgrade_blocked {
                    None
                } else {
                    Some(*calculated_collateralization)
                }
            }

            // Optionally buffered collateral upgraded change
            (
                CollateralizationState::UnderMarginCallThreshold,
                CollateralizationState::FullyCollateralized,
            ) => match upgrade_buffer_cvl_pct {
                Some(buffer) => {
                    if self
                        .margin_call_cvl
                        .is_significantly_lower_than(current_cvl, buffer)
                    {
                        Some(*calculated_collateralization)
                    } else {
                        None
                    }
                }
                _ => Some(*calculated_collateralization),
            },

            // Valid other collateral changes
            (CollateralizationState::NoExposure, _)
            | (CollateralizationState::NoCollateral, _)
            | (CollateralizationState::FullyCollateralized, _)
            | (CollateralizationState::UnderMarginCallThreshold, _) => {
                Some(*calculated_collateralization)
            }
        }
    }
}

impl TermValuesBuilder {
    fn validate(&self) -> Result<(), TermsError> {
        let initial_cvl = self
            .initial_cvl
            .ok_or(UninitializedFieldError::new("initial_cvl"))?;
        let margin_call_cvl = self
            .margin_call_cvl
            .ok_or(UninitializedFieldError::new("margin_call_cvl"))?;
        let liquidation_cvl = self
            .liquidation_cvl
            .ok_or(UninitializedFieldError::new("liquidation_cvl"))?;

        if initial_cvl <= margin_call_cvl {
            return Err(TermsError::MarginCallAboveInitialLimit(
                margin_call_cvl,
                initial_cvl,
            ));
        }

        if margin_call_cvl <= liquidation_cvl {
            return Err(TermsError::MarginCallBelowLiquidationLimit(
                margin_call_cvl,
                liquidation_cvl,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use rust_decimal_macros::dec;

    use super::*;

    fn terms() -> TermValues {
        TermValues::builder()
            .annual_rate(AnnualRatePct(dec!(12)))
            .duration(FacilityDuration::Months(3))
            .interest_due_duration_from_accrual(ObligationDuration::Days(0))
            .obligation_overdue_duration_from_due(None)
            .obligation_liquidation_duration_from_due(None)
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .accrual_interval(InterestInterval::EndOfDay)
            .one_time_fee_rate(OneTimeFeeRatePct(dec!(1)))
            .liquidation_cvl(dec!(105))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .build()
            .expect("should build a valid term")
    }

    #[test]
    fn invalid_term_values_margin_call_greater_than_initial() {
        let result = TermValues::builder()
            .annual_rate(AnnualRatePct(dec!(12)))
            .duration(FacilityDuration::Months(3))
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .one_time_fee_rate(OneTimeFeeRatePct(dec!(1)))
            .liquidation_cvl(dec!(105))
            .margin_call_cvl(dec!(150))
            .initial_cvl(dec!(140))
            .build();

        match result.unwrap_err() {
            TermsError::MarginCallAboveInitialLimit(margin_call, initial) => {
                assert_eq!(margin_call, CVLPct::from(dec!(150)));
                assert_eq!(initial, CVLPct::from(dec!(140)));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn invalid_term_values_liquidation_greater_than_margin_call() {
        let result = TermValues::builder()
            .annual_rate(AnnualRatePct(dec!(12)))
            .duration(FacilityDuration::Months(3))
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .one_time_fee_rate(OneTimeFeeRatePct(dec!(1)))
            .liquidation_cvl(dec!(130))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .build();

        match result.unwrap_err() {
            TermsError::MarginCallBelowLiquidationLimit(margin_call, liquidation) => {
                assert_eq!(margin_call, CVLPct::from(dec!(125)));
                assert_eq!(liquidation, CVLPct::from(dec!(130)));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn invalid_term_values_margin_call_equal_to_liquidation() {
        let result = TermValues::builder()
            .annual_rate(AnnualRatePct(dec!(12)))
            .duration(FacilityDuration::Months(3))
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .one_time_fee_rate(OneTimeFeeRatePct(dec!(1)))
            .liquidation_cvl(dec!(125))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .build();

        match result.unwrap_err() {
            TermsError::MarginCallBelowLiquidationLimit(margin_call, liquidation) => {
                assert_eq!(margin_call, CVLPct::from(dec!(125)));
                assert_eq!(liquidation, CVLPct::from(dec!(125)));
            }
            _ => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn required_collateral() {
        let price =
            PriceOfOneBTC::new(UsdCents::try_from_usd(rust_decimal_macros::dec!(1000)).unwrap());
        let terms = terms();
        let principal = UsdCents::from(100000);
        let required_collateral = terms.required_collateral(principal, price);
        let sats = Satoshis::try_from_btc(dec!(1.4)).unwrap();
        assert_eq!(required_collateral, sats);
    }

    #[test]
    fn days() {
        let start_date = "2024-12-03T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfMonth.period_from(start_date).days(),
            29
        );

        let start_date = "2024-12-01T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfMonth.period_from(start_date).days(),
            31
        );
    }

    #[test]
    fn truncate() {
        let start_date = "2024-12-03T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let period = InterestInterval::EndOfMonth.period_from(start_date);

        let latest_before_start_date = "2024-12-02T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(period.truncate(latest_before_start_date), None);

        let latest_after_start_date_before_end_date =
            "2024-12-20T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(
            period
                .truncate(latest_after_start_date_before_end_date)
                .unwrap()
                .end
                < period.end
        );

        let latest_after_end_date = "2025-01-03T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            period.truncate(latest_after_end_date).unwrap().end,
            period.end
        );
    }

    #[test]
    fn interest_calculation() {
        let terms = terms();
        let principal = UsdCents::try_from_usd(dec!(100)).unwrap();
        let days = 365;
        let interest = terms.annual_rate.interest_for_time_period(principal, days);
        assert_eq!(interest, UsdCents::from(1200));

        let principal = UsdCents::try_from_usd(dec!(1000)).unwrap();
        let days = 23;
        let interest = terms.annual_rate.interest_for_time_period(principal, days);
        assert_eq!(interest, UsdCents::from(757));
    }

    #[test]
    fn maturity_date() {
        let start_date = "2024-12-03T14:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let duration = FacilityDuration::Months(3);
        let maturity_date = "2025-03-03T14:00:00Z"
            .parse::<DateTime<Utc>>()
            .unwrap()
            .into();
        assert_eq!(duration.maturity_date(start_date), maturity_date);
    }

    #[test]
    fn next_period() {
        let start_date = "2024-12-03T14:00:00Z".parse::<DateTime<Utc>>().unwrap();

        let expected_next_start_date = "2024-12-04T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let day_period = InterestInterval::EndOfDay.period_from(start_date);
        assert_eq!(day_period.next().start, expected_next_start_date);

        let expected_next_start_date = "2025-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let month_period = InterestInterval::EndOfMonth.period_from(start_date);
        assert_eq!(month_period.next().start, expected_next_start_date);
    }

    #[test]
    fn end_date_starting_at_month_interval() {
        let expected_end_date = "2024-12-31T23:59:59Z".parse::<DateTime<Utc>>().unwrap();

        let start_of_month = "2024-12-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfMonth.end_date_starting_at(start_of_month),
            expected_end_date
        );

        let middle_of_month = "2024-12-15T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfMonth.end_date_starting_at(middle_of_month),
            expected_end_date
        );

        let end_of_month = "2024-12-31T23:59:59Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfMonth.end_date_starting_at(end_of_month),
            expected_end_date
        );
    }

    #[test]
    fn end_date_starting_at_day_interval() {
        let expected_end_date = "2024-12-03T23:59:59Z".parse::<DateTime<Utc>>().unwrap();

        let start_of_day = "2024-12-03T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfDay.end_date_starting_at(start_of_day),
            expected_end_date
        );

        let middle_of_day = "2024-12-03T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfDay.end_date_starting_at(middle_of_day),
            expected_end_date
        );

        let end_of_day = "2024-12-03T23:59:59Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(
            InterestInterval::EndOfDay.end_date_starting_at(end_of_day),
            expected_end_date
        );
    }

    #[test]
    fn can_apply_one_time_fee() {
        let fee = OneTimeFeeRatePct(dec!(5)).apply(UsdCents::from(1000));
        assert_eq!(fee, UsdCents::from(50));
    }

    #[test]
    fn one_time_fee_rounds_up() {
        let fee = OneTimeFeeRatePct(dec!(5.01)).apply(UsdCents::from(1000));
        assert_eq!(fee, UsdCents::from(51));
    }

    fn default_terms() -> TermValues {
        TermValues::builder()
            .annual_rate(dec!(12))
            .duration(FacilityDuration::Months(3))
            .interest_due_duration_from_accrual(ObligationDuration::Days(0))
            .obligation_overdue_duration_from_due(None)
            .obligation_liquidation_duration_from_due(None)
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .accrual_interval(InterestInterval::EndOfDay)
            .one_time_fee_rate(OneTimeFeeRatePct(dec!(1)))
            .liquidation_cvl(dec!(105))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .build()
            .expect("should build a valid term")
    }

    mod collateralization_update {
        use super::*;

        fn default_upgrade_buffer_cvl_pct() -> CVLPct {
            CVLPct::new(5)
        }
        struct TestCVL {
            above_fully_collateralized: CVLPct,
            above_margin_called_and_buffer: CVLPct,
            above_margin_called_and_below_buffer: CVLPct,
            below_margin_called: CVLPct,
            below_liquidation: CVLPct,
        }
        fn cvl_test_values() -> TestCVL {
            let terms = default_terms();
            let upgrade_buffer_cvl = default_upgrade_buffer_cvl_pct();

            TestCVL {
                above_fully_collateralized: terms.initial_cvl + CVLPct::new(1),
                above_margin_called_and_buffer: terms.margin_call_cvl
                    + upgrade_buffer_cvl
                    + CVLPct::new(1),
                above_margin_called_and_below_buffer: terms.margin_call_cvl + upgrade_buffer_cvl
                    - CVLPct::new(1),
                below_margin_called: terms.margin_call_cvl - CVLPct::new(1),
                below_liquidation: terms.liquidation_cvl - CVLPct::new(1),
            }
        }

        fn collateralization_update(
            last_state: CollateralizationState,
            cvl: CVLPct,
        ) -> Option<CollateralizationState> {
            default_terms().collateralization_update(cvl, last_state, None, false)
        }

        fn collateralization_update_with_buffer(
            last_state: CollateralizationState,
            cvl: CVLPct,
        ) -> Option<CollateralizationState> {
            default_terms().collateralization_update(
                cvl,
                last_state,
                Some(default_upgrade_buffer_cvl_pct()),
                false,
            )
        }

        fn collateralization_update_with_liquidation_limit(
            last_state: CollateralizationState,
            cvl: CVLPct,
        ) -> Option<CollateralizationState> {
            default_terms().collateralization_update(cvl, last_state, None, true)
        }

        fn all_collaterization_update_fns()
        -> Vec<fn(CollateralizationState, CVLPct) -> Option<CollateralizationState>> {
            vec![
                collateralization_update,
                collateralization_update_with_buffer,
                collateralization_update_with_liquidation_limit,
            ]
        }

        #[test]
        fn fully_collateralized_to_fully_collateralized() {
            for collateralization_update in all_collaterization_update_fns() {
                assert_eq!(
                    collateralization_update(
                        CollateralizationState::FullyCollateralized,
                        cvl_test_values().above_fully_collateralized + CVLPct::new(1),
                    ),
                    None
                );
            }
        }

        #[test]
        fn fully_collateralized_to_under_margin_called() {
            for collateralization_update in all_collaterization_update_fns() {
                assert_eq!(
                    collateralization_update(
                        CollateralizationState::FullyCollateralized,
                        cvl_test_values().below_margin_called,
                    ),
                    Some(CollateralizationState::UnderMarginCallThreshold)
                );
            }
        }

        #[test]
        fn fully_collateralized_to_under_liquidation() {
            for collateralization_update in all_collaterization_update_fns() {
                assert_eq!(
                    collateralization_update(
                        CollateralizationState::FullyCollateralized,
                        cvl_test_values().below_liquidation,
                    ),
                    Some(CollateralizationState::UnderLiquidationThreshold)
                );
            }
        }

        #[test]
        fn under_margin_called_to_above_margin_called_and_below_buffer() {
            assert_eq!(
                collateralization_update(
                    CollateralizationState::UnderMarginCallThreshold,
                    cvl_test_values().above_margin_called_and_below_buffer,
                ),
                Some(CollateralizationState::FullyCollateralized)
            );

            assert_eq!(
                collateralization_update_with_buffer(
                    CollateralizationState::UnderMarginCallThreshold,
                    cvl_test_values().above_margin_called_and_below_buffer,
                ),
                None
            );

            assert_eq!(
                collateralization_update_with_liquidation_limit(
                    CollateralizationState::UnderMarginCallThreshold,
                    cvl_test_values().above_margin_called_and_below_buffer,
                ),
                Some(CollateralizationState::FullyCollateralized)
            );
        }

        #[test]
        fn under_margin_called_to_fully_collateralized() {
            for collateralization_update in all_collaterization_update_fns() {
                assert_eq!(
                    collateralization_update(
                        CollateralizationState::UnderMarginCallThreshold,
                        cvl_test_values().above_margin_called_and_buffer,
                    ),
                    Some(CollateralizationState::FullyCollateralized),
                );
            }
        }

        #[test]
        fn under_margin_called_to_under_liquidation() {
            for collateralization_update in all_collaterization_update_fns() {
                assert_eq!(
                    collateralization_update(
                        CollateralizationState::UnderMarginCallThreshold,
                        cvl_test_values().below_liquidation,
                    ),
                    Some(CollateralizationState::UnderLiquidationThreshold),
                );
            }
        }

        #[test]
        fn under_liquidation_to_fully_collateralized() {
            assert_eq!(
                collateralization_update(
                    CollateralizationState::UnderLiquidationThreshold,
                    cvl_test_values().above_fully_collateralized,
                ),
                Some(CollateralizationState::FullyCollateralized),
            );

            assert_eq!(
                collateralization_update_with_buffer(
                    CollateralizationState::UnderLiquidationThreshold,
                    cvl_test_values().above_fully_collateralized,
                ),
                Some(CollateralizationState::FullyCollateralized),
            );

            assert_eq!(
                collateralization_update_with_liquidation_limit(
                    CollateralizationState::UnderLiquidationThreshold,
                    cvl_test_values().above_fully_collateralized,
                ),
                None,
            );
        }
    }

    fn default_balances(facility: UsdCents) -> CreditFacilityBalanceSummary {
        CreditFacilityBalanceSummary {
            facility,
            facility_remaining: facility,
            collateral: Satoshis::ZERO,
            disbursed: UsdCents::ZERO,
            not_yet_due_disbursed_outstanding: UsdCents::ZERO,
            due_disbursed_outstanding: UsdCents::ZERO,
            overdue_disbursed_outstanding: UsdCents::ZERO,
            disbursed_defaulted: UsdCents::ZERO,
            interest_posted: UsdCents::ZERO,
            not_yet_due_interest_outstanding: UsdCents::ZERO,
            due_interest_outstanding: UsdCents::ZERO,
            overdue_interest_outstanding: UsdCents::ZERO,
            interest_defaulted: UsdCents::ZERO,
        }
    }

    #[test]
    fn check_approval_allowed() {
        let terms = default_terms();
        let price = PriceOfOneBTC::new(UsdCents::try_from_usd(dec!(100_000)).unwrap());
        let principal = UsdCents::try_from_usd(dec!(100_000)).unwrap();
        let required_collateral =
            price.cents_to_sats_round_up(terms.margin_call_cvl.scale(principal));

        let balance = CreditFacilityProposalBalanceSummary::new(
            principal,
            required_collateral - Satoshis::from(1),
        );

        assert!(!terms.is_proposal_completion_allowed(balance, price));

        let balance = CreditFacilityProposalBalanceSummary::new(principal, required_collateral);

        assert!(terms.is_proposal_completion_allowed(balance, price));
    }

    #[test]
    fn check_disbursal_allowed() {
        let terms = default_terms();
        let price = PriceOfOneBTC::new(UsdCents::try_from_usd(dec!(100_000)).unwrap());
        let principal = UsdCents::try_from_usd(dec!(100_000)).unwrap();
        let mut balance = default_balances(principal);
        balance.collateral = Satoshis::try_from_btc(dec!(1)).unwrap();

        let amount = UsdCents::try_from_usd(dec!(80_001)).unwrap();
        assert!(!terms.is_disbursal_allowed(balance, amount, price));

        let amount = UsdCents::try_from_usd(dec!(80_000)).unwrap();
        assert!(terms.is_disbursal_allowed(balance, amount, price));
    }

    #[test]
    fn disburse_all_at_activation_toggle() {
        let terms = default_terms();
        assert!(!terms.disburse_all_at_activation());

        let terms = TermValues::builder()
            .annual_rate(AnnualRatePct(dec!(12)))
            .duration(FacilityDuration::Months(3))
            .interest_due_duration_from_accrual(ObligationDuration::Days(0))
            .obligation_overdue_duration_from_due(None)
            .obligation_liquidation_duration_from_due(None)
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .accrual_interval(InterestInterval::EndOfDay)
            .one_time_fee_rate(OneTimeFeeRatePct(dec!(1)))
            .liquidation_cvl(dec!(105))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .disburse_all_at_activation(true)
            .build()
            .expect("should build a valid term");

        assert!(terms.disburse_all_at_activation());
    }
}
