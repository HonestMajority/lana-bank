#!/usr/bin/env bats

load "helpers"

PERSISTED_LOG_FILE="credit-facility-proposal.e2e-logs"
RUN_LOG_FILE="credit-facility-proposal.run.e2e-logs"

setup_file() {
  start_server
  login_superadmin
  reset_log_files "$PERSISTED_LOG_FILE" "$RUN_LOG_FILE"
}

teardown_file() {
  stop_server
  cp "$LOG_FILE" "$PERSISTED_LOG_FILE"
}

wait_for_active() {
  credit_facility_id=$1

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"

  status=$(graphql_output '.data.creditFacility.status')
  [[ "$status" == "ACTIVE" ]] || exit 1
}

wait_for_disbursal() {
  credit_facility_id=$1
  disbursal_id=$2

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"
  echo "disbursal | $i. $(graphql_output)" >> $RUN_LOG_FILE
  num_disbursals=$(
    graphql_output \
      --arg disbursal_id "$disbursal_id" \
      '[
        .data.creditFacility.disbursals[]
        | select(.id == $disbursal_id)
        ] | length'
  )
  [[ "$num_disbursals" -eq "1" ]]
}

wait_for_accruals() {
  expected_num_accruals=$1
  credit_facility_id=$2

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"
  echo "accrual | $i. $(graphql_output)" >> $RUN_LOG_FILE
  num_accruals=$(
    graphql_output '[
      .data.creditFacility.history[]
      | select(.__typename == "CreditFacilityInterestAccrued")
      ] | length'
  )

  [[ "$num_accruals" == "$expected_num_accruals" ]] || exit 1
}

wait_for_dashboard_disbursed() {
  before=$1
  disbursed_amount=$2

  expected_after="$(( $before + $disbursed_amount ))"

  exec_admin_graphql 'dashboard'
  after=$(graphql_output '.data.dashboard.totalDisbursed')

  [[ "$after" -eq "$expected_after" ]] || exit 1
}

wait_for_dashboard_payment() {
  before=$1
  payment_amount=$2

  expected_after="$(( $before - $payment_amount ))"

  exec_admin_graphql 'dashboard'
  after=$(graphql_output '.data.dashboard.totalDisbursed')

  [[ "$after" -eq "$expected_after" ]] || exit 1
}

wait_for_full_activation_disbursal() {
  credit_facility_id=$1

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"

  facility_amount=$(graphql_output '.data.creditFacility.facilityAmount')
  [[ "$facility_amount" != "null" ]] || exit 1

  disbursal_count=$(graphql_output '.data.creditFacility.disbursals | length')
  [[ "$disbursal_count" != "null" ]] || exit 1
  [[ "$disbursal_count" -eq 1 ]] || exit 1

  disbursal_status=$(graphql_output '.data.creditFacility.disbursals[0].status')
  [[ "$disbursal_status" == "CONFIRMED" ]] || exit 1

  facility_remaining=$(graphql_output '.data.creditFacility.balance.facilityRemaining.usdBalance')
  [[ "$facility_remaining" == "0" ]] || exit 1

  disbursed_total=$(graphql_output '.data.creditFacility.balance.disbursed.total.usdBalance')
  [[ "$disbursed_total" == "$facility_amount" ]] || exit 1

  disbursed_outstanding=$(graphql_output '.data.creditFacility.balance.disbursed.outstanding.usdBalance')
  [[ "$disbursed_outstanding" == "$facility_amount" ]] || exit 1
}

ymd() {
  local date_value
  read -r date_value
  echo $date_value | cut -d 'T' -f1 | tr -d '-'
}

@test "credit-facility-proposal: can create" {
  # Setup prerequisites
  customer_id=$(create_customer)

  retry 80 1 wait_for_checking_account "$customer_id"

  variables=$(
    jq -n \
      --arg customerId "$customer_id" \
    '{
      id: $customerId
    }'
  )
  exec_admin_graphql 'customer' "$variables"

  deposit_account_id=$(graphql_output '.data.customer.depositAccount.depositAccountId')
  [[ "$deposit_account_id" != "null" ]] || exit 1


  facility=100000
  variables=$(
    jq -n \
    --arg customerId "$customer_id" \
    --arg disbursal_credit_account_id "$deposit_account_id" \
    --argjson facility "$facility" \
    '{
      input: {
        customerId: $customerId,
        facility: $facility,
        disbursalCreditAccountId: $disbursal_credit_account_id,
        terms: {
          annualRate: "12",
          accrualCycleInterval: "END_OF_MONTH",
          accrualInterval: "END_OF_DAY",
          oneTimeFeeRate: "5",
          disburseAllAtActivation: false,
          duration: { period: "MONTHS", units: 3 },
          interestDueDurationFromAccrual: { period: "DAYS", units: 0 },
          obligationOverdueDurationFromDue: { period: "DAYS", units: 50 },
          obligationLiquidationDurationFromDue: { period: "DAYS", units: 360 },
          liquidationCvl: "105",
          marginCallCvl: "125",
          initialCvl: "140"
        }
      }
    }'
  )

  exec_admin_graphql 'credit-facility-proposal-create' "$variables"

  address=$(graphql_output '.data.creditFacilityProposalCreate.creditFacilityProposal.wallet.address')
  [[ "$address" == "null" ]] || exit 1

  credit_facility_proposal_id=$(graphql_output '.data.creditFacilityProposalCreate.creditFacilityProposal.creditFacilityProposalId')
  [[ "$credit_facility_proposal_id" != "null" ]] || exit 1

  cache_value 'credit_facility_proposal_id' "$credit_facility_proposal_id"
}

@test "credit-facility-proposal: can update collateral" {
  credit_facility_proposal_id=$(read_value 'credit_facility_proposal_id')

  variables=$(
    jq -n \
      --arg credit_facility_proposal_id "$credit_facility_proposal_id" \
      --arg effective "$(naive_now)" \
    '{
      input: {
        creditFacilityProposalId: $credit_facility_proposal_id,
        collateral: 50000000,
        effective: $effective,
      }
    }'
  )
  exec_admin_graphql 'credit-facility-proposal-collateral-update' "$variables"
  credit_facility_proposal_id=$(graphql_output '.data.creditFacilityProposalCollateralUpdate.creditFacilityProposal.creditFacilityProposalId')
  [[ "$credit_facility_proposal_id" != "null" ]] || exit 1

  credit_facility_id=$credit_facility_proposal_id

  retry 10 1 wait_for_active "$credit_facility_id"

  cache_value 'credit_facility_id' "$credit_facility_id"
}

@test "credit-facility: can initiate disbursal" {
  credit_facility_id=$(read_value 'credit_facility_id')

  exec_admin_graphql 'dashboard'
  disbursed_before=$(graphql_output '.data.dashboard.totalDisbursed')

  amount=50000
  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
      --argjson amount "$amount" \
    '{
      input: {
        creditFacilityId: $creditFacilityId,
        amount: $amount,
      }
    }'
  )
  exec_admin_graphql 'credit-facility-disbursal-initiate' "$variables"
  disbursal_id=$(graphql_output '.data.creditFacilityDisbursalInitiate.disbursal.id')
  [[ "$disbursal_id" != "null" ]] || exit 1

  retry 10 1 wait_for_disbursal "$credit_facility_id" "$disbursal_id"
  retry 10 1 wait_for_dashboard_disbursed "$disbursed_before" "$amount"
}

@test "credit-facility: records accruals" {

  credit_facility_id=$(read_value 'credit_facility_id')
  retry 30 2 wait_for_accruals 4 "$credit_facility_id"

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"
  status=$(graphql_output '.data.creditFacility.status')
  [[ "$status" == "MATURED" ]] || exit 1

  num_accruals=$(
    graphql_output '[
      .data.creditFacility.history[]
      | select(.__typename == "CreditFacilityInterestAccrued")
      ] | length'
  )
  [[ "$num_accruals" -eq "4" ]] || exit 1

  # assert_accounts_balanced
}

@test "credit-facility: record payment" {
  credit_facility_id=$(read_value 'credit_facility_id')

  exec_admin_graphql 'dashboard'
  disbursed_before=$(graphql_output '.data.dashboard.totalDisbursed')

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"
  balance=$(graphql_output '.data.creditFacility.balance')

  interest=$(echo $balance | jq -r '.interest.total.usdBalance')
  interest_outstanding=$(echo $balance | jq -r '.interest.outstanding.usdBalance')
  [[ "$interest" -eq "$interest_outstanding" ]] || exit 1

  disbursed=$(echo $balance | jq -r '.disbursed.total.usdBalance')
  disbursed_outstanding=$(echo $balance | jq -r '.disbursed.outstanding.usdBalance')
  [[ "$disbursed" -eq "$disbursed_outstanding" ]] || exit 1

  total_outstanding=$(echo $balance | jq -r '.outstanding.usdBalance')
  [[ "$total_outstanding" -eq "$(( $interest_outstanding + $disbursed_outstanding ))" ]] || exit 1

  disbursed_payment=25000
  amount="$(( $disbursed_payment + $interest_outstanding ))"
  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
      --argjson amount "$amount" \
    '{
      input: {
        creditFacilityId: $creditFacilityId,
        amount: $amount,
      }
    }'
  )
  exec_admin_graphql 'credit-facility-partial-payment-record' "$variables"
  updated_balance=$(graphql_output '.data.creditFacilityPartialPaymentRecord.creditFacility.balance')

  updated_interest=$(echo $updated_balance | jq -r '.interest.total.usdBalance')
  [[ "$interest" -eq "$updated_interest" ]] || exit 1
  updated_disbursed=$(echo $updated_balance | jq -r '.disbursed.total.usdBalance')
  [[ "$disbursed" -eq "$updated_disbursed" ]] || exit 1

  updated_total_outstanding=$(echo $updated_balance | jq -r '.outstanding.usdBalance')
  [[ "$updated_total_outstanding" -lt "$total_outstanding" ]] || exit 1

  updated_interest_outstanding=$(echo $updated_balance | jq -r '.interest.outstanding.usdBalance')
  [[ "$updated_interest_outstanding" -eq "0" ]] || exit 1

  retry 10 1 wait_for_dashboard_payment "$disbursed_before" "$disbursed_payment"

  # assert_accounts_balanced
}

@test "credit-facility: disburses all liquidity at activation when configured" {
  customer_id=$(create_customer)

  retry 80 1 wait_for_checking_account "$customer_id"

  variables=$(
    jq -n \
      --arg customerId "$customer_id" \
    '{
      id: $customerId
    }'
  )
  exec_admin_graphql 'customer' "$variables"

  disbursal_credit_account_id=$(graphql_output '.data.customer.depositAccount.depositAccountId')
  [[ "$disbursal_credit_account_id" != "null" ]] || exit 1

  facility=150000
  variables=$(
    jq -n \
      --arg customerId "$customer_id" \
      --arg disbursal_credit_account_id "$disbursal_credit_account_id" \
      --argjson facility "$facility" \
    '{
      input: {
        customerId: $customerId,
        facility: $facility,
        disbursalCreditAccountId: $disbursal_credit_account_id,
        terms: {
          annualRate: "12",
          accrualCycleInterval: "END_OF_MONTH",
          accrualInterval: "END_OF_DAY",
          oneTimeFeeRate: "5",
          disburseAllAtActivation: true,
          duration: { period: "MONTHS", units: 3 },
          interestDueDurationFromAccrual: { period: "DAYS", units: 0 },
          obligationOverdueDurationFromDue: { period: "DAYS", units: 50 },
          obligationLiquidationDurationFromDue: { period: "DAYS", units: 360 },
          liquidationCvl: "105",
          marginCallCvl: "125",
          initialCvl: "140"
        }
      }
    }'
  )
  exec_admin_graphql 'credit-facility-proposal-create' "$variables"

  credit_facility_id=$(graphql_output '.data.creditFacilityProposalCreate.creditFacilityProposal.creditFacilityProposalId')
  [[ "$credit_facility_id" != "null" ]] || exit 1

  variables=$(
    jq -n \
      --arg creditFacilityProposalId "$credit_facility_id" \
      --arg effective "$(naive_now)" \
    '{
      input: {
        creditFacilityProposalId: $creditFacilityProposalId,
        collateral: 50000000,
        effective: $effective,
      }
    }'
  )
  exec_admin_graphql 'credit-facility-proposal-collateral-update' "$variables"

  retry 10 1 wait_for_active "$credit_facility_id"
  retry 20 1 wait_for_full_activation_disbursal "$credit_facility_id"

  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
    '{ id: $creditFacilityId }'
  )
  exec_admin_graphql 'find-credit-facility' "$variables"
  facility_amount=$(graphql_output '.data.creditFacility.facilityAmount')
  [[ "$facility_amount" == "$facility" ]] || exit 1
  
  additional_disbursal_amount=10000
  variables=$(
    jq -n \
      --arg creditFacilityId "$credit_facility_id" \
      --argjson amount "$additional_disbursal_amount" \
    '{
      input: {
        creditFacilityId: $creditFacilityId,
        amount: $amount,
      }
    }'
  )
  exec_admin_graphql 'credit-facility-disbursal-initiate' "$variables"

  error_message=$(graphql_output '.errors[0].message')
  [[ "$error_message" == *"CreditFacilityError - Additional disbursals disabled by terms"* ]] || exit 1

  disbursal_response=$(graphql_output '.data.creditFacilityDisbursalInitiate')
  [[ "$disbursal_response" == "null" ]] || exit 1
}
