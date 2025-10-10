use thiserror::Error;

use core_money::{Satoshis, UsdCents};

#[derive(Error, Debug)]
pub enum CreditFacilityError {
    #[error("CreditFacilityError - Sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("CreditFacilityError - EsEntityError: {0}")]
    EsEntityError(es_entity::EsEntityError),
    #[error("FacilityError - CursorDestructureError: {0}")]
    CursorDestructureError(#[from] es_entity::CursorDestructureError),
    #[error("CreditFacilityError - ConversionError: {0}")]
    ConversionError(#[from] crate::primitives::ConversionError),
    #[error("CreditFacilityError - InterestAccrualCycleError: {0}")]
    InterestAccrualCycleError(
        #[from] crate::interest_accrual_cycle::error::InterestAccrualCycleError,
    ),
    #[error("CreditFacilityError - ApprovalInProgress")]
    ApprovalInProgress,
    #[error("CreditFacilityError - Denied")]
    Denied,
    #[error("CreditFacilityError - DisbursalPastMaturityDate")]
    DisbursalPastMaturityDate,
    #[error("CreditFacilityError - NotActivatedYet")]
    NotActivatedYet,
    #[error("CreditFacilityError - InterestAccrualNotCompletedYet")]
    InterestAccrualNotCompletedYet,
    #[error("CreditFacilityError - NoDisbursalInProgress")]
    NoDisbursalInProgress,
    #[error("CreditFacilityError - CollateralNotUpdated: before({0}), after({1})")]
    CollateralNotUpdated(Satoshis, Satoshis),
    #[error("CreditFacilityError - NoCollateral")]
    NoCollateral,
    #[error("CreditFacilityError - BelowMarginLimit")]
    BelowMarginLimit,
    #[error("CreditFacilityError - PaymentExceedsOutstandingCreditFacilityAmount: {0} > {1}")]
    PaymentExceedsOutstandingCreditFacilityAmount(UsdCents, UsdCents),
    #[error("CreditFacilityError - FacilityLedgerBalanceMismatch")]
    FacilityLedgerBalanceMismatch,
    #[error("CreditFacilityError - OutstandingAmount")]
    OutstandingAmount,
    #[error("CreditFacilityError - InterestAccrualCycleWithInvalidFutureStartDate")]
    InterestAccrualCycleWithInvalidFutureStartDate,
    #[error("CreditFacilityError - InProgressInterestAccrualCycleNotCompletedYet")]
    InProgressInterestAccrualCycleNotCompletedYet,
    #[error(
        "CreditFacilityError - DisbursalAmountTooLarge: amount '{0}' is larger than facility balance '{1}'"
    )]
    DisbursalAmountTooLarge(UsdCents, UsdCents),
    #[error("CreditFacilityError - Additional disbursals disabled by terms")]
    DisbursalsDisabledByTerms,
    #[error("CreditFacilityError - AuthorizationError: {0}")]
    AuthorizationError(#[from] authz::error::AuthorizationError),
    #[error("CreditFacilityError - AuditError: {0}")]
    AuditError(#[from] audit::error::AuditError),
    #[error("CreditFacilityError - LedgerError: {0}")]
    LedgerError(#[from] crate::ledger::error::CreditLedgerError),
    #[error("CreditFacilityError - PriceError: {0}")]
    PriceError(#[from] core_price::error::PriceError),
    #[error("CreditFacilityError - ObligationError: {0}")]
    ObligationError(#[from] crate::obligation::error::ObligationError),
    #[error("CreditFacilityError - GovernanceError: {0}")]
    GovernanceError(#[from] governance::error::GovernanceError),
    #[error("CreditFacilityError - PublicIdError: {0}")]
    PublicIdError(#[from] public_id::PublicIdError),
    #[error("CreditFacilityError - PaymentAllocationError: {0}")]
    PaymentAllocationError(#[from] crate::payment_allocation::error::PaymentAllocationError),
    #[error("CreditFacilityError - JobError: {0}")]
    JobError(#[from] job::error::JobError),
    #[error("CreditFacilityError - CreditFacilityProposalError: {0}")]
    CreditFacilityProposalError(
        #[from] crate::credit_facility_proposal::error::CreditFacilityProposalError,
    ),
}

es_entity::from_es_entity_error!(CreditFacilityError);
