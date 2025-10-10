use cala_ledger::{
    tx_template::{Params, error::TxTemplateError, *},
    *,
};
use rust_decimal::Decimal;
use tracing::instrument;

use crate::ledger::error::*;

pub const CHARGE_STRUCTURING_FEE_CODE: &str = "CHARGE_STRUCTURING_FEE";

#[derive(Debug)]
pub struct ChargeStructuringFeeParams {
    pub journal_id: JournalId,
    pub facility_fee_income_account: CalaAccountId,
    pub debit_account_id: CalaAccountId,
    pub structuring_fee_amount: Decimal,
    pub currency: Currency,
    pub external_id: String,
}

impl ChargeStructuringFeeParams {
    pub fn defs() -> Vec<NewParamDefinition> {
        vec![
            NewParamDefinition::builder()
                .name("journal_id")
                .r#type(ParamDataType::Uuid)
                .build()
                .unwrap(),
            NewParamDefinition::builder()
                .name("facility_fee_income_account")
                .r#type(ParamDataType::Uuid)
                .build()
                .unwrap(),
            NewParamDefinition::builder()
                .name("debit_account_id")
                .r#type(ParamDataType::Uuid)
                .build()
                .unwrap(),
            NewParamDefinition::builder()
                .name("structuring_fee_amount")
                .r#type(ParamDataType::Decimal)
                .build()
                .unwrap(),
            NewParamDefinition::builder()
                .name("currency")
                .r#type(ParamDataType::String)
                .build()
                .unwrap(),
            NewParamDefinition::builder()
                .name("external_id")
                .r#type(ParamDataType::String)
                .build()
                .unwrap(),
        ]
    }
}

impl From<ChargeStructuringFeeParams> for Params {
    fn from(
        ChargeStructuringFeeParams {
            journal_id,
            facility_fee_income_account,
            debit_account_id,
            structuring_fee_amount,
            currency,
            external_id,
        }: ChargeStructuringFeeParams,
    ) -> Self {
        let mut params = Self::default();
        params.insert("journal_id", journal_id);
        params.insert("facility_fee_income_account", facility_fee_income_account);
        params.insert("debit_account_id", debit_account_id);
        params.insert("structuring_fee_amount", structuring_fee_amount);
        params.insert("currency", currency);
        params.insert("external_id", external_id);
        params
    }
}

pub struct ChargeStructuringFee;

impl ChargeStructuringFee {
    #[instrument(name = "ledger.charge_structuring_fee.init", skip_all)]
    pub async fn init(ledger: &CalaLedger) -> Result<(), CreditLedgerError> {
        let tx_input = NewTxTemplateTransaction::builder()
            .journal_id("params.journal_id")
            .external_id("params.external_id")
            .description("'Charge structuring fee'")
            .build()
            .expect("Couldn't build TxInput");

        let entries = vec![
            NewTxTemplateEntry::builder()
                .account_id("params.debit_account_id")
                .units("params.structuring_fee_amount")
                .currency("params.currency")
                .entry_type("'ADD_STRUCTURING_FEE_DR'")
                .direction("DEBIT")
                .layer("SETTLED")
                .build()
                .expect("Couldn't build entry"),
            NewTxTemplateEntry::builder()
                .account_id("params.facility_fee_income_account")
                .units("params.structuring_fee_amount")
                .currency("params.currency")
                .entry_type("'ADD_STRUCTURING_FEE_CR'")
                .direction("CREDIT")
                .layer("SETTLED")
                .build()
                .expect("Couldn't build entry"),
        ];

        let params = ChargeStructuringFeeParams::defs();
        let template = NewTxTemplate::builder()
            .id(TxTemplateId::new())
            .code(CHARGE_STRUCTURING_FEE_CODE)
            .transaction(tx_input)
            .entries(entries)
            .params(params)
            .build()
            .expect("Couldn't build template");

        match ledger.tx_templates().create(template).await {
            Err(TxTemplateError::DuplicateCode) => Ok(()),
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }
}
