use chrono::{DateTime, Utc};
use derive_builder::Builder;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use es_entity::*;

use crate::{
    interest_accrual_cycle::*,
    ledger::*,
    obligation::{NewObligation, ObligationsAmounts},
    primitives::*,
    terms::{InterestPeriod, TermValues},
};

use super::error::CreditFacilityError;

#[derive(EsEvent, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[es_event(id = "CreditFacilityId")]
pub enum CreditFacilityEvent {
    Initialized {
        id: CreditFacilityId,
        credit_facility_proposal_id: CreditFacilityProposalId,
        customer_id: CustomerId,
        customer_type: CustomerType,
        collateral_id: CollateralId,
        ledger_tx_id: LedgerTxId,
        terms: TermValues,
        amount: UsdCents,
        account_ids: CreditFacilityLedgerAccountIds,
        disbursal_credit_account_id: CalaAccountId,
        public_id: PublicId,
        activated_at: DateTime<Utc>,
        maturity_date: EffectiveDate,
    },
    InterestAccrualCycleStarted {
        interest_accrual_id: InterestAccrualCycleId,
        interest_accrual_cycle_idx: InterestAccrualCycleIdx,
        interest_period: InterestPeriod,
    },
    InterestAccrualCycleConcluded {
        interest_accrual_cycle_idx: InterestAccrualCycleIdx,
        ledger_tx_id: LedgerTxId,
        obligation_id: Option<ObligationId>,
    },
    CollateralizationStateChanged {
        collateralization_state: CollateralizationState,
        collateral: Satoshis,
        outstanding: CreditFacilityReceivable,
        price: PriceOfOneBTC,
    },
    CollateralizationRatioChanged {
        collateralization_ratio: CollateralizationRatio,
    },
    Matured {},
    Completed {},
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct CreditFacilityReceivable {
    pub disbursed: UsdCents,
    pub interest: UsdCents,
}

impl From<CreditFacilityBalanceSummary> for CreditFacilityReceivable {
    fn from(balance: CreditFacilityBalanceSummary) -> Self {
        Self {
            disbursed: balance.disbursed_outstanding_payable(),
            interest: balance.interest_outstanding_payable(),
        }
    }
}

impl From<ObligationsAmounts> for CreditFacilityReceivable {
    fn from(outstanding: ObligationsAmounts) -> Self {
        Self {
            disbursed: outstanding.disbursed,
            interest: outstanding.interest,
        }
    }
}

impl CreditFacilityReceivable {
    pub fn total(&self) -> UsdCents {
        self.interest + self.disbursed
    }

    pub fn is_zero(&self) -> bool {
        self.total().is_zero()
    }
}

#[derive(Debug)]
pub(crate) struct NewAccrualPeriods {
    pub(crate) accrual: InterestPeriod,
}

struct InterestAccrualCycleInCreditFacility {
    idx: InterestAccrualCycleIdx,
    period: InterestPeriod,
}

impl From<(InterestAccrualData, CreditFacilityLedgerAccountIds)> for CreditFacilityInterestAccrual {
    fn from(data: (InterestAccrualData, CreditFacilityLedgerAccountIds)) -> Self {
        let (
            InterestAccrualData {
                interest,
                period,
                tx_ref,
                tx_id,
            },
            credit_facility_account_ids,
        ) = data;
        Self {
            interest,
            period,
            tx_ref,
            tx_id,
            credit_facility_account_ids,
        }
    }
}

impl From<(InterestAccrualCycleData, CreditFacilityLedgerAccountIds)>
    for CreditFacilityInterestAccrualCycle
{
    fn from(data: (InterestAccrualCycleData, CreditFacilityLedgerAccountIds)) -> Self {
        let (
            InterestAccrualCycleData {
                interest,
                effective,
                tx_ref,
                tx_id,
            },
            credit_facility_account_ids,
        ) = data;
        Self {
            interest,
            effective,
            tx_ref,
            tx_id,
            credit_facility_account_ids,
        }
    }
}

#[derive(EsEntity, Builder)]
#[builder(pattern = "owned", build_fn(error = "EsEntityError"))]
pub struct CreditFacility {
    pub id: CreditFacilityId,
    pub credit_facility_proposal_id: CreditFacilityProposalId,
    pub customer_id: CustomerId,
    pub collateral_id: CollateralId,
    pub amount: UsdCents,
    pub terms: TermValues,
    pub account_ids: CreditFacilityLedgerAccountIds,
    pub disbursal_credit_account_id: CalaAccountId,
    pub public_id: PublicId,
    pub activated_at: DateTime<Utc>,
    pub maturity_date: EffectiveDate,

    #[es_entity(nested)]
    #[builder(default)]
    interest_accruals: Nested<InterestAccrualCycle>,
    events: EntityEvents<CreditFacilityEvent>,
}

impl CreditFacility {
    pub(crate) fn activation_data(&self) -> CreditFacilityActivation {
        self.events
            .iter_all()
            .find_map(|event| match event {
                CreditFacilityEvent::Initialized {
                    id,
                    ledger_tx_id,
                    account_ids,
                    amount,
                    disbursal_credit_account_id,
                    customer_type,
                    terms,
                    ..
                } => Some(CreditFacilityActivation {
                    credit_facility_id: *id,
                    tx_id: *ledger_tx_id,
                    tx_ref: format!("{}-activate", *id),
                    account_ids: *account_ids,
                    debit_account_id: *disbursal_credit_account_id,
                    facility_amount: *amount,
                    structuring_fee_amount: self.structuring_fee(),
                    customer_type: *customer_type,
                    duration_type: terms.duration.duration_type(),
                }),
                _ => None,
            })
            .expect("Initialized event should exist")
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.events
            .entity_first_persisted_at()
            .expect("entity_first_persisted_at not found")
    }

    pub fn matures_at(&self) -> DateTime<Utc> {
        self.maturity_date.start_of_day()
    }

    pub(crate) fn structuring_fee(&self) -> UsdCents {
        self.terms.one_time_fee_rate.apply(self.amount)
    }

    pub(crate) fn disburse_all_at_activation(&self) -> bool {
        self.terms.disburse_all_at_activation()
    }

    pub(crate) fn activation_disbursal_amount(&self) -> UsdCents {
        if self.disburse_all_at_activation() {
            self.amount
        } else {
            let structuring_fee = self.structuring_fee();

            if structuring_fee >= self.amount {
                UsdCents::ZERO
            } else {
                self.amount - structuring_fee
            }
        }
    }

    fn is_matured(&self) -> bool {
        self.events
            .iter_all()
            .rev()
            .any(|event| matches!(event, CreditFacilityEvent::Matured { .. }))
    }

    pub fn status(&self) -> CreditFacilityStatus {
        if self.is_completed() {
            CreditFacilityStatus::Closed
        } else if self.is_matured() {
            CreditFacilityStatus::Matured
        } else {
            CreditFacilityStatus::Active
        }
    }

    pub(crate) fn mature(&mut self) -> Idempotent<()> {
        idempotency_guard!(self.events.iter_all(), CreditFacilityEvent::Matured { .. });

        if self.status() == CreditFacilityStatus::Closed {
            return Idempotent::Ignored;
        }

        self.events.push(CreditFacilityEvent::Matured {});
        Idempotent::Executed(())
    }

    pub(crate) fn check_disbursal_date(&self, initiated_at: DateTime<Utc>) -> bool {
        initiated_at < self.matures_at()
    }

    fn last_started_accrual_cycle(&self) -> Option<InterestAccrualCycleInCreditFacility> {
        self.events.iter_all().rev().find_map(|event| match event {
            CreditFacilityEvent::InterestAccrualCycleStarted {
                interest_accrual_cycle_idx,
                interest_period,
                ..
            } => Some(InterestAccrualCycleInCreditFacility {
                idx: *interest_accrual_cycle_idx,
                period: *interest_period,
            }),
            _ => None,
        })
    }

    fn in_progress_accrual_cycle_id(&self) -> Option<InterestAccrualCycleId> {
        self.events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::InterestAccrualCycleConcluded { .. } => Some(None),
                CreditFacilityEvent::InterestAccrualCycleStarted {
                    interest_accrual_id,
                    ..
                } => Some(Some(*interest_accrual_id)),
                _ => None,
            })
            .flatten()
    }

    fn next_interest_accrual_cycle_period(
        &self,
    ) -> Result<Option<InterestPeriod>, CreditFacilityError> {
        let last_accrual_start_date = self
            .last_started_accrual_cycle()
            .map(|cycle| cycle.period.start);

        let interval = self.terms.accrual_cycle_interval;
        let full_period = match last_accrual_start_date {
            Some(last_accrual_start_date) => interval.period_from(last_accrual_start_date).next(),
            None => interval.period_from(self.activated_at),
        };

        Ok(full_period.truncate(self.matures_at()))
    }

    fn next_interest_accrual_cycle_idx(&self) -> InterestAccrualCycleIdx {
        self.last_started_accrual_cycle()
            .map(|cycle| cycle.idx.next())
            .unwrap_or(InterestAccrualCycleIdx::FIRST)
    }

    fn is_in_progress_interest_cycle_completed(&self) -> bool {
        let accrual_cycle = self.interest_accrual_cycle_in_progress();
        match accrual_cycle {
            Some(accrual_cycle) => accrual_cycle.is_completed(),
            None => true,
        }
    }

    pub(crate) fn start_interest_accrual_cycle(
        &mut self,
    ) -> Result<Option<NewAccrualPeriods>, CreditFacilityError> {
        if !self.is_in_progress_interest_cycle_completed() {
            return Err(CreditFacilityError::InProgressInterestAccrualCycleNotCompletedYet);
        }

        let accrual_cycle_period = match self.next_interest_accrual_cycle_period()? {
            Some(period) => period,
            None => return Ok(None),
        };
        let now = crate::time::now();
        if accrual_cycle_period.start > now {
            return Err(CreditFacilityError::InterestAccrualCycleWithInvalidFutureStartDate);
        }

        let idx = self.next_interest_accrual_cycle_idx();
        let id = InterestAccrualCycleId::new();
        self.events
            .push(CreditFacilityEvent::InterestAccrualCycleStarted {
                interest_accrual_id: id,
                interest_accrual_cycle_idx: idx,
                interest_period: accrual_cycle_period,
            });

        let new_accrual = NewInterestAccrualCycle::builder()
            .id(id)
            .credit_facility_id(self.id)
            .account_ids(self.account_ids.into())
            .idx(idx)
            .period(accrual_cycle_period)
            .facility_maturity_date(self.maturity_date)
            .terms(self.terms)
            .build()
            .expect("could not build new interest accrual");
        Ok(Some(NewAccrualPeriods {
            accrual: self
                .interest_accruals
                .add_new(new_accrual)
                .first_accrual_cycle_period(),
        }))
    }

    pub(crate) fn record_interest_accrual_cycle(
        &mut self,
    ) -> Result<Idempotent<(InterestAccrualCycleData, Option<NewObligation>)>, CreditFacilityError>
    {
        let accrual_cycle_data = self
            .interest_accrual_cycle_in_progress()
            .expect("accrual not found")
            .accrual_cycle_data()
            .ok_or(CreditFacilityError::InterestAccrualNotCompletedYet)?;

        let (idx, new_obligation) = {
            let accrual = self
                .interest_accrual_cycle_in_progress_mut()
                .expect("accrual not found");

            (
                accrual.idx,
                match accrual.record_accrual_cycle(accrual_cycle_data.clone()) {
                    Idempotent::Executed(new_obligation) => new_obligation,
                    Idempotent::Ignored => {
                        return Ok(Idempotent::Ignored);
                    }
                },
            )
        };

        self.events
            .push(CreditFacilityEvent::InterestAccrualCycleConcluded {
                interest_accrual_cycle_idx: idx,
                obligation_id: new_obligation.as_ref().map(|o| o.id),
                ledger_tx_id: accrual_cycle_data.tx_id,
            });

        Ok(Idempotent::Executed((accrual_cycle_data, new_obligation)))
    }

    pub fn interest_accrual_cycle_in_progress(&self) -> Option<&InterestAccrualCycle> {
        self.in_progress_accrual_cycle_id().map(|cycle_id| {
            self.interest_accruals
                .get_persisted(&cycle_id)
                .expect("Interest accrual not found")
        })
    }

    pub fn interest_accrual_cycle_in_progress_mut(&mut self) -> Option<&mut InterestAccrualCycle> {
        self.in_progress_accrual_cycle_id().map(|cycle_id| {
            self.interest_accruals
                .get_persisted_mut(&cycle_id)
                .expect("Interest accrual not found")
        })
    }

    pub fn last_collateralization_state(&self) -> CollateralizationState {
        if self.is_completed() {
            return CollateralizationState::NoCollateral;
        }

        self.events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::CollateralizationStateChanged {
                    collateralization_state: state,
                    ..
                } => Some(*state),
                _ => None,
            })
            .unwrap_or(CollateralizationState::NoCollateral)
    }

    pub fn last_collateralization_ratio(&self) -> CollateralizationRatio {
        self.events
            .iter_all()
            .rev()
            .find_map(|event| match event {
                CreditFacilityEvent::CollateralizationRatioChanged {
                    collateralization_ratio,
                    ..
                } => Some(*collateralization_ratio),
                _ => None,
            })
            .unwrap_or_default()
    }

    pub(crate) fn update_collateralization(
        &mut self,
        price: PriceOfOneBTC,
        upgrade_buffer_cvl_pct: CVLPct,
        balances: CreditFacilityBalanceSummary,
    ) -> Idempotent<Option<CollateralizationState>> {
        let ratio_changed = self.update_collateralization_ratio(&balances).did_execute();

        let last_collateralization_state = self.last_collateralization_state();

        let collateralization_update = match self.status() {
            CreditFacilityStatus::Active | CreditFacilityStatus::Matured => {
                self.terms.collateralization_update(
                    balances.current_cvl(price),
                    last_collateralization_state,
                    Some(upgrade_buffer_cvl_pct),
                    false,
                )
            }
            CreditFacilityStatus::Closed => Some(CollateralizationState::NoCollateral),
        };

        if let Some(calculated_collateralization) = collateralization_update {
            self.events
                .push(CreditFacilityEvent::CollateralizationStateChanged {
                    collateralization_state: calculated_collateralization,
                    collateral: balances.collateral(),
                    outstanding: balances.into(),
                    price,
                });

            Idempotent::Executed(Some(calculated_collateralization))
        } else if ratio_changed {
            Idempotent::Executed(None)
        } else {
            Idempotent::Ignored
        }
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.events
            .iter_all()
            .rev()
            .any(|event| matches!(event, CreditFacilityEvent::Completed { .. }))
    }

    pub(crate) fn complete(
        &mut self,
        _price: PriceOfOneBTC,
        _upgrade_buffer_cvl_pct: CVLPct,
        balances: CreditFacilityBalanceSummary,
    ) -> Result<Idempotent<CreditFacilityCompletion>, CreditFacilityError> {
        idempotency_guard!(
            self.events.iter_all(),
            CreditFacilityEvent::Completed { .. }
        );
        if balances.any_outstanding_or_defaulted() {
            return Err(CreditFacilityError::OutstandingAmount);
        }

        let res = CreditFacilityCompletion {
            tx_id: LedgerTxId::new(),
            collateral: balances.collateral(),
            credit_facility_account_ids: self.account_ids,
        };

        self.events.push(CreditFacilityEvent::Completed {});

        Ok(Idempotent::Executed(res))
    }

    fn update_collateralization_ratio(
        &mut self,
        balance: &CreditFacilityBalanceSummary,
    ) -> Idempotent<()> {
        let ratio = balance.current_collateralization_ratio();

        if self.last_collateralization_ratio() != ratio {
            self.events
                .push(CreditFacilityEvent::CollateralizationRatioChanged {
                    collateralization_ratio: ratio,
                });
        } else {
            return Idempotent::Ignored;
        }

        Idempotent::Executed(())
    }
}

impl TryFromEvents<CreditFacilityEvent> for CreditFacility {
    fn try_from_events(events: EntityEvents<CreditFacilityEvent>) -> Result<Self, EsEntityError> {
        let mut builder = CreditFacilityBuilder::default();
        for event in events.iter_all() {
            match event {
                CreditFacilityEvent::Initialized {
                    id,
                    credit_facility_proposal_id,
                    amount,
                    customer_id,
                    collateral_id,
                    account_ids,
                    disbursal_credit_account_id,
                    terms: t,
                    public_id,
                    maturity_date,
                    activated_at,
                    ..
                } => {
                    builder = builder
                        .id(*id)
                        .credit_facility_proposal_id(*credit_facility_proposal_id)
                        .amount(*amount)
                        .customer_id(*customer_id)
                        .collateral_id(*collateral_id)
                        .terms(*t)
                        .account_ids(*account_ids)
                        .disbursal_credit_account_id(*disbursal_credit_account_id)
                        .public_id(public_id.clone())
                        .activated_at(*activated_at)
                        .maturity_date(*maturity_date);
                }
                CreditFacilityEvent::InterestAccrualCycleStarted { .. } => (),
                CreditFacilityEvent::InterestAccrualCycleConcluded { .. } => (),
                CreditFacilityEvent::CollateralizationStateChanged { .. } => (),
                CreditFacilityEvent::CollateralizationRatioChanged { .. } => (),
                CreditFacilityEvent::Matured { .. } => (),
                CreditFacilityEvent::Completed { .. } => (),
            }
        }
        builder.events(events).build()
    }
}

#[derive(Debug, Builder)]
pub struct NewCreditFacility {
    #[builder(setter(into))]
    pub(super) id: CreditFacilityId,
    #[builder(setter(into))]
    pub(super) credit_facility_proposal_id: CreditFacilityProposalId,
    #[builder(setter(into))]
    pub(super) ledger_tx_id: LedgerTxId,
    #[builder(setter(into))]
    pub(super) customer_id: CustomerId,
    pub(super) customer_type: CustomerType,
    #[builder(setter(into))]
    pub(super) collateral_id: CollateralId,
    terms: TermValues,
    amount: UsdCents,
    activated_at: DateTime<Utc>,
    maturity_date: EffectiveDate,
    #[builder(setter(skip), default)]
    pub(super) status: CreditFacilityStatus,
    #[builder(setter(skip), default)]
    pub(super) collateralization_state: CollateralizationState,
    account_ids: CreditFacilityLedgerAccountIds,
    disbursal_credit_account_id: CalaAccountId,
    #[builder(setter(into))]
    pub(super) public_id: PublicId,
}

impl NewCreditFacility {
    pub fn builder() -> NewCreditFacilityBuilder {
        NewCreditFacilityBuilder::default()
    }
}

impl IntoEvents<CreditFacilityEvent> for NewCreditFacility {
    fn into_events(self) -> EntityEvents<CreditFacilityEvent> {
        EntityEvents::init(
            self.id,
            [CreditFacilityEvent::Initialized {
                id: self.id,
                credit_facility_proposal_id: self.credit_facility_proposal_id,
                ledger_tx_id: self.ledger_tx_id,
                customer_id: self.customer_id,
                customer_type: self.customer_type,
                collateral_id: self.collateral_id,
                terms: self.terms,
                amount: self.amount,
                account_ids: self.account_ids,
                disbursal_credit_account_id: self.disbursal_credit_account_id,
                public_id: self.public_id,
                activated_at: self.activated_at,
                maturity_date: self.maturity_date,
            }],
        )
    }
}

#[cfg(test)]
mod test {
    use rust_decimal_macros::dec;

    use crate::{
        terms::{FacilityDuration, InterestInterval, OneTimeFeeRatePct},
        *,
    };

    use super::*;

    fn default_terms() -> TermValues {
        TermValues::builder()
            .annual_rate(dec!(12))
            .duration(FacilityDuration::Months(3))
            .interest_due_duration_from_accrual(ObligationDuration::Days(0))
            .obligation_overdue_duration_from_due(None)
            .obligation_liquidation_duration_from_due(None)
            .accrual_cycle_interval(InterestInterval::EndOfMonth)
            .accrual_interval(InterestInterval::EndOfDay)
            .one_time_fee_rate(OneTimeFeeRatePct::new(5))
            .liquidation_cvl(dec!(105))
            .margin_call_cvl(dec!(125))
            .initial_cvl(dec!(140))
            .build()
            .expect("should build a valid term")
    }

    fn disburse_all_terms() -> TermValues {
        let mut terms = default_terms();
        terms.disburse_all_at_activation = true;
        terms
    }

    fn date_from(d: &str) -> DateTime<Utc> {
        d.parse::<DateTime<Utc>>().unwrap()
    }

    fn default_facility() -> UsdCents {
        UsdCents::from(10_00)
    }

    fn default_price() -> PriceOfOneBTC {
        PriceOfOneBTC::new(UsdCents::from(5000000))
    }

    fn default_upgrade_buffer_cvl_pct() -> CVLPct {
        CVLPct::new(5)
    }

    fn facility_from(events: Vec<CreditFacilityEvent>) -> CreditFacility {
        CreditFacility::try_from_events(EntityEvents::init(CreditFacilityId::new(), events))
            .unwrap()
    }

    fn activated_at() -> DateTime<Utc> {
        date_from("2021-01-15T12:00:00Z")
    }

    fn account_ids() -> CreditFacilityLedgerAccountIds {
        CreditFacilityLedgerAccountIds {
            facility_account_id: CalaAccountId::new(),
            in_liquidation_account_id: CalaAccountId::new(),
            disbursed_receivable_not_yet_due_account_id: CalaAccountId::new(),
            disbursed_receivable_due_account_id: CalaAccountId::new(),
            disbursed_receivable_overdue_account_id: CalaAccountId::new(),
            disbursed_defaulted_account_id: CalaAccountId::new(),
            collateral_account_id: CalaAccountId::new(),
            interest_receivable_not_yet_due_account_id: CalaAccountId::new(),
            interest_receivable_due_account_id: CalaAccountId::new(),
            interest_receivable_overdue_account_id: CalaAccountId::new(),
            interest_defaulted_account_id: CalaAccountId::new(),
            interest_income_account_id: CalaAccountId::new(),
            fee_income_account_id: CalaAccountId::new(),
        }
    }

    fn initial_events() -> Vec<CreditFacilityEvent> {
        initial_events_with_terms(default_terms())
    }

    fn initial_events_with_terms(terms: TermValues) -> Vec<CreditFacilityEvent> {
        let id = CreditFacilityId::new();
        let maturity_date = terms.maturity_date(activated_at());

        vec![CreditFacilityEvent::Initialized {
            id,
            credit_facility_proposal_id: id.into(),
            ledger_tx_id: LedgerTxId::new(),
            customer_id: CustomerId::new(),
            customer_type: CustomerType::Individual,
            collateral_id: CollateralId::new(),
            amount: default_facility(),
            terms,
            account_ids: account_ids(),
            disbursal_credit_account_id: CalaAccountId::new(),
            public_id: PublicId::new(format!("test-public-id-{}", uuid::Uuid::new_v4())),
            activated_at: activated_at(),
            maturity_date,
        }]
    }

    fn hydrate_accruals_in_facility(credit_facility: &mut CreditFacility) {
        let new_entities = credit_facility
            .interest_accruals
            .new_entities_mut()
            .drain(..)
            .map(|new| {
                InterestAccrualCycle::try_from_events(new.into_events()).expect("hydrate failed")
            })
            .collect::<Vec<_>>();
        credit_facility.interest_accruals.load(new_entities);
    }

    fn start_interest_accrual_cycle(credit_facility: &mut CreditFacility) {
        credit_facility.start_interest_accrual_cycle().unwrap();
        hydrate_accruals_in_facility(credit_facility);
    }

    fn iterate_in_progress_accrual_cycle_to_completion(credit_facility: &mut CreditFacility) {
        let accrual = credit_facility
            .interest_accrual_cycle_in_progress_mut()
            .unwrap();
        while accrual.next_accrual_period().is_some() {
            accrual.record_accrual(UsdCents::ONE);
        }
        let _ = accrual.record_accrual_cycle(accrual.accrual_cycle_data().unwrap());
    }

    #[test]
    fn can_progress_next_accrual_idx() {
        let mut events = initial_events();
        let credit_facility = facility_from(events.clone());
        assert_eq!(
            credit_facility.next_interest_accrual_cycle_idx(),
            InterestAccrualCycleIdx::FIRST
        );

        events.push(CreditFacilityEvent::InterestAccrualCycleStarted {
            interest_accrual_id: InterestAccrualCycleId::new(),
            interest_accrual_cycle_idx: InterestAccrualCycleIdx::FIRST,
            interest_period: InterestInterval::EndOfDay.period_from(activated_at()),
        });
        let credit_facility = facility_from(events);
        assert_eq!(
            credit_facility.next_interest_accrual_cycle_idx(),
            InterestAccrualCycleIdx::FIRST.next()
        );
    }

    mod next_interest_accrual_cycle_period {

        use super::*;

        #[test]
        fn first_period_starts_at_activation_when_no_prior_accrual() {
            let events = initial_events();
            let first_interest_period = InterestInterval::EndOfMonth.period_from(activated_at());
            let credit_facility = facility_from(events);

            let period = credit_facility
                .next_interest_accrual_cycle_period()
                .unwrap()
                .unwrap();
            assert_eq!(period, first_interest_period);
        }

        #[test]
        fn next_period_after_accrual_event() {
            let mut events = initial_events();
            let first_interest_period = InterestInterval::EndOfMonth.period_from(activated_at());
            events.extend([CreditFacilityEvent::InterestAccrualCycleStarted {
                interest_accrual_id: InterestAccrualCycleId::new(),
                interest_accrual_cycle_idx: InterestAccrualCycleIdx::FIRST,
                interest_period: first_interest_period,
            }]);
            let credit_facility = facility_from(events);

            let period = credit_facility
                .next_interest_accrual_cycle_period()
                .unwrap()
                .unwrap();
            assert_eq!(period, first_interest_period.next());
        }

        #[test]
        fn next_period_after_last_accrual_event_is_none() {
            let mut events = initial_events();
            let matures_at = facility_from(events.clone()).matures_at();
            let final_interest_period =
                InterestInterval::EndOfMonth.period_from(matures_at - chrono::Duration::days(1));
            events.push(CreditFacilityEvent::InterestAccrualCycleStarted {
                interest_accrual_id: InterestAccrualCycleId::new(),
                interest_accrual_cycle_idx: InterestAccrualCycleIdx::FIRST,
                interest_period: final_interest_period,
            });
            let credit_facility = facility_from(events.clone());

            let period = credit_facility
                .next_interest_accrual_cycle_period()
                .unwrap();
            assert!(period.is_none());
        }
    }

    mod start_interest_accrual_cycle {

        use super::*;

        #[test]
        fn can_start() {
            let events = initial_events();
            let mut credit_facility = facility_from(events);

            let first_accrual_cycle_period @ InterestPeriod { start, .. } = credit_facility
                .next_interest_accrual_cycle_period()
                .unwrap()
                .unwrap();
            assert_eq!(start, activated_at());

            credit_facility
                .start_interest_accrual_cycle()
                .unwrap()
                .unwrap();
            let second_accrual_period = credit_facility
                .next_interest_accrual_cycle_period()
                .unwrap()
                .unwrap();
            assert_eq!(second_accrual_period, first_accrual_cycle_period.next());
        }

        #[test]
        fn errors_if_previous_cycle_not_completed() {
            let events = initial_events();
            let mut credit_facility = facility_from(events);

            start_interest_accrual_cycle(&mut credit_facility);
            assert!(matches!(
                credit_facility.start_interest_accrual_cycle(),
                Err(CreditFacilityError::InProgressInterestAccrualCycleNotCompletedYet)
            ));
        }

        #[test]
        fn does_not_start_after_last_cycle() {
            let events = initial_events();
            let mut credit_facility = facility_from(events);

            while credit_facility
                .next_interest_accrual_cycle_period()
                .unwrap()
                .is_some()
            {
                assert!(
                    credit_facility
                        .start_interest_accrual_cycle()
                        .unwrap()
                        .is_some(),
                );
                hydrate_accruals_in_facility(&mut credit_facility);
                iterate_in_progress_accrual_cycle_to_completion(&mut credit_facility);
            }
            assert!(
                credit_facility
                    .start_interest_accrual_cycle()
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn structuring_fee() {
        let credit_facility = facility_from(initial_events());
        let expected_fee = default_terms().one_time_fee_rate.apply(default_facility());
        assert_eq!(credit_facility.structuring_fee(), expected_fee);
    }

    #[test]
    fn activation_disbursal_amount_excludes_structuring_fee_when_not_disbursing_all() {
        let credit_facility = facility_from(initial_events());
        let expected_amount = credit_facility.amount - credit_facility.structuring_fee();

        assert_eq!(
            credit_facility.activation_disbursal_amount(),
            expected_amount
        );
    }

    #[test]
    fn activation_disbursal_amount_disburse_all_returns_full_amount() {
        let credit_facility =
            facility_from(initial_events_with_terms(disburse_all_terms()));

        assert_eq!(
            credit_facility.activation_disbursal_amount(),
            credit_facility.amount
        );
    }

    mod completion {
        use super::*;

        impl From<CreditFacilityReceivable> for ObligationsAmounts {
            fn from(receivable: CreditFacilityReceivable) -> Self {
                Self {
                    disbursed: receivable.disbursed,
                    interest: receivable.interest,
                }
            }
        }

        #[test]
        fn can_complete() {
            let mut credit_facility = facility_from(initial_events());

            let _ = credit_facility
                .complete(
                    default_price(),
                    default_upgrade_buffer_cvl_pct(),
                    CreditFacilityBalanceSummary {
                        collateral: Satoshis::ZERO,
                        not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                        due_disbursed_outstanding: UsdCents::ZERO,
                        overdue_disbursed_outstanding: UsdCents::ZERO,
                        disbursed_defaulted: UsdCents::ZERO,
                        not_yet_due_interest_outstanding: UsdCents::ZERO,
                        due_interest_outstanding: UsdCents::ZERO,
                        overdue_interest_outstanding: UsdCents::ZERO,
                        interest_defaulted: UsdCents::ZERO,

                        facility: UsdCents::from(2),
                        facility_remaining: UsdCents::from(1),
                        disbursed: UsdCents::from(1),
                        interest_posted: UsdCents::from(1),
                    },
                )
                .unwrap();
            assert!(credit_facility.is_completed());
            assert!(credit_facility.status() == CreditFacilityStatus::Closed);
        }

        #[test]
        fn errors_if_not_yet_due_outstanding() {
            let mut credit_facility = facility_from(initial_events());

            let res_disbursed = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    not_yet_due_disbursed_outstanding: UsdCents::from(1),
                    not_yet_due_interest_outstanding: UsdCents::ZERO,

                    collateral: Satoshis::ZERO,
                    due_disbursed_outstanding: UsdCents::ZERO,
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    disbursed_defaulted: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::ZERO,
                    interest_defaulted: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_disbursed,
                Err(CreditFacilityError::OutstandingAmount)
            ));

            let res_interest = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::from(1),

                    collateral: Satoshis::ZERO,
                    due_disbursed_outstanding: UsdCents::ZERO,
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    disbursed_defaulted: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::ZERO,
                    interest_defaulted: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_interest,
                Err(CreditFacilityError::OutstandingAmount)
            ));
        }

        #[test]
        fn errors_if_due_outstanding() {
            let mut credit_facility = facility_from(initial_events());

            let res_disbursed = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    due_disbursed_outstanding: UsdCents::from(1),
                    due_interest_outstanding: UsdCents::ZERO,

                    collateral: Satoshis::ZERO,
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    disbursed_defaulted: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::ZERO,
                    interest_defaulted: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_disbursed,
                Err(CreditFacilityError::OutstandingAmount)
            ));

            let res_interest = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    due_disbursed_outstanding: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::from(1),

                    collateral: Satoshis::ZERO,
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    disbursed_defaulted: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::ZERO,
                    interest_defaulted: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_interest,
                Err(CreditFacilityError::OutstandingAmount)
            ));
        }

        #[test]
        fn errors_if_overdue_outstanding() {
            let mut credit_facility = facility_from(initial_events());

            let res_disbursed = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    overdue_disbursed_outstanding: UsdCents::from(1),
                    overdue_interest_outstanding: UsdCents::ZERO,

                    collateral: Satoshis::ZERO,
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    due_disbursed_outstanding: UsdCents::ZERO,
                    disbursed_defaulted: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::ZERO,
                    interest_defaulted: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_disbursed,
                Err(CreditFacilityError::OutstandingAmount)
            ));

            let res_interest = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::from(1),

                    collateral: Satoshis::ZERO,
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    due_disbursed_outstanding: UsdCents::ZERO,
                    disbursed_defaulted: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::ZERO,
                    interest_defaulted: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_interest,
                Err(CreditFacilityError::OutstandingAmount)
            ));
        }

        #[test]
        fn errors_if_defaulted_outstanding() {
            let mut credit_facility = facility_from(initial_events());

            let res_disbursed = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    disbursed_defaulted: UsdCents::from(1),
                    interest_defaulted: UsdCents::ZERO,

                    collateral: Satoshis::ZERO,
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    due_disbursed_outstanding: UsdCents::ZERO,
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_disbursed,
                Err(CreditFacilityError::OutstandingAmount)
            ));

            let res_interest = credit_facility.complete(
                default_price(),
                default_upgrade_buffer_cvl_pct(),
                CreditFacilityBalanceSummary {
                    disbursed_defaulted: UsdCents::ZERO,
                    interest_defaulted: UsdCents::from(1),

                    collateral: Satoshis::ZERO,
                    not_yet_due_disbursed_outstanding: UsdCents::ZERO,
                    due_disbursed_outstanding: UsdCents::ZERO,
                    overdue_disbursed_outstanding: UsdCents::ZERO,
                    not_yet_due_interest_outstanding: UsdCents::ZERO,
                    due_interest_outstanding: UsdCents::ZERO,
                    overdue_interest_outstanding: UsdCents::ZERO,

                    facility: UsdCents::from(2),
                    facility_remaining: UsdCents::from(1),
                    disbursed: UsdCents::from(1),
                    interest_posted: UsdCents::from(1),
                },
            );
            assert!(matches!(
                res_interest,
                Err(CreditFacilityError::OutstandingAmount)
            ));
        }
    }
}
