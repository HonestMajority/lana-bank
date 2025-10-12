import {
  InterestInterval,
  Period,
  CreateCommitteeMutationResult,
} from "../../lib/graphql/generated/index"
import { DEFAULT_TERMS } from "../../lib/constants/terms"

import { t } from "../support/translation"

const CF = "CreditFacilities"
const CFP = "CreditFacilityProposals"
const Committee = "Committees.CommitteeDetails"
const Policy = "Policies.PolicyDetails"
const Disbursals = "Disbursals"

describe("credit facility", () => {
  let customerId: string
  let customerPublicId: string
  let proposalId: string
  const termsTemplateName: string = `Test Template ${Date.now()}`

  before(() => {
    Cypress.env("creditFacilityPublicId", null)
    Cypress.env("creditFacilityProposalId", null)
    cy.createTermsTemplate({
      name: termsTemplateName,
      annualRate: "5.5",
      accrualCycleInterval: InterestInterval.EndOfMonth,
      accrualInterval: InterestInterval.EndOfDay,
      oneTimeFeeRate: "5",
      disburseAllAtActivation: false,
      liquidationCvl: "110",
      marginCallCvl: "120",
      initialCvl: "140",
      duration: {
        units: 12 * 100,
        period: Period.Months,
      },
      interestDueDurationFromAccrual: {
        units: DEFAULT_TERMS.INTEREST_DUE_DURATION_FROM_ACCRUAL.UNITS,
        period: DEFAULT_TERMS.INTEREST_DUE_DURATION_FROM_ACCRUAL.PERIOD,
      },
      obligationOverdueDurationFromDue: {
        units: DEFAULT_TERMS.OBLIGATION_OVERDUE_DURATION_FROM_DUE.UNITS,
        period: DEFAULT_TERMS.OBLIGATION_OVERDUE_DURATION_FROM_DUE.PERIOD,
      },
      obligationLiquidationDurationFromDue: {
        period: DEFAULT_TERMS.OBLIGATION_LIQUIDATION_DURATION_FROM_DUE.PERIOD,
        units: DEFAULT_TERMS.OBLIGATION_LIQUIDATION_DURATION_FROM_DUE.UNITS,
      },
    }).then((id) => {
      cy.log(`Created terms template with ID: ${id}`)
    })

    const testEmail = `t${Date.now().toString().slice(-6)}@example.com`
    const testTelegramId = `t${Date.now()}`
    cy.createCustomer(testEmail, testTelegramId).then((customer) => {
      customerId = customer.customerId
      customerPublicId = customer.publicId
      cy.log(`Created customer with ID: ${customerId}`)
    })
  })

  beforeEach(() => {
    cy.on("uncaught:exception", (err) => {
      if (err.message.includes("ResizeObserver loop")) {
        return false
      }
    })
  })

  it("should add admin to credit facility and disbursal approvers", () => {
    const committeeName = `${Date.now()}-CF-and-Disbursal-Approvers`
    const createCommitteeMutation = `mutation CreateCommittee($input: CommitteeCreateInput!) {
      committeeCreate(input: $input) {
        committee {
          committeeId
        }
      }
    }`
    cy.graphqlRequest<CreateCommitteeMutationResult>(createCommitteeMutation, {
      input: { name: committeeName },
    }).then((response) => {
      const committeeId = response.data?.committeeCreate.committee.committeeId
      cy.visit(`/committees/${committeeId}`)
      cy.get('[data-testid="committee-add-member-button"]').click()
      cy.get('[data-testid="committee-add-user-select"]').should("be.visible").click()
      cy.get('[role="option"]')
        .contains("admin")
        .then((option) => {
          cy.wrap(option).click()
          cy.get('[data-testid="committee-add-user-submit-button"]').click()
          cy.contains(t(Committee + ".AddUserCommitteeDialog.success")).should(
            "be.visible",
          )
          cy.contains(option.text().split(" ")[0]).should("be.visible")
        })

      cy.visit(`/policies`)
      cy.get('[data-testid="table-row-1"] > :nth-child(3) > a > .gap-2').should(
        "be.visible",
      )
      cy.get('[data-testid="table-row-1"] > :nth-child(3) > a > .gap-2').click()
      cy.get('[data-testid="policy-assign-committee"]').click()
      cy.get('[data-testid="policy-select-committee-selector"]').click()
      cy.get('[role="option"]').contains(committeeName).click()
      cy.get("[data-testid=policy-assign-committee-threshold-input]").type("1")
      cy.get("[data-testid=policy-assign-committee-submit-button]").click()
      cy.contains(t(Policy + ".CommitteeAssignmentDialog.success.assigned")).should(
        "be.visible",
      )
      cy.contains(committeeName).should("be.visible")

      cy.visit(`/policies`)
      cy.get('[data-testid="table-row-0"] > :nth-child(3) > a > .gap-2').should(
        "be.visible",
      )
      cy.get('[data-testid="table-row-0"] > :nth-child(3) > a > .gap-2').click()
      cy.get('[data-testid="policy-assign-committee"]').click()
      cy.get('[data-testid="policy-select-committee-selector"]').click()
      cy.get('[role="option"]').contains(committeeName).click()
      cy.get("[data-testid=policy-assign-committee-threshold-input]").type("1")
      cy.get("[data-testid=policy-assign-committee-submit-button]").click()
      cy.contains(t(Policy + ".CommitteeAssignmentDialog.success.assigned")).should(
        "be.visible",
      )
      cy.contains(committeeName).should("be.visible")
    })
  })

  it("should create a credit facility proposal and verify initial state", () => {
    cy.visit(`/customers/${customerPublicId}`)
    cy.get('[data-testid="loading-skeleton"]').should("not.exist")

    cy.get('[data-testid="global-create-button"]').click()
    cy.takeScreenshot("01_click_create_proposal_button")

    cy.get('[data-testid="create-credit-facility-button"]').should("be.visible").click()
    cy.takeScreenshot("02_open_proposal_form")

    cy.get('[data-testid="facility-amount-input"]').type("5000")
    cy.get('[data-testid="credit-facility-terms-template-select"]').click()
    cy.get('[role="option"]').contains(termsTemplateName).click()

    cy.takeScreenshot("03_enter_facility_amount")

    cy.get('[data-testid="create-credit-facility-submit"]').click()
    cy.takeScreenshot("04_submit_proposal_form")

    cy.url()
      .should("match", /\/credit-facility-proposals\/[a-f0-9-]+$/)
      .then((url) => {
        proposalId = url.split("/").pop() as string
        Cypress.env("creditFacilityProposalId", proposalId)
      })

    cy.contains(t(CFP + ".collateralizationState.undercollateralized")).should(
      "be.visible",
    )
    cy.takeScreenshot("05_proposal_created_success")
  })

  it("should show newly created proposal in the list", () => {
    cy.visit(`/credit-facility-proposals`)
    cy.get('[data-testid="table-row-0"] > :nth-child(7) > a > .gap-2').click()
    cy.contains("$5,000.00").should("be.visible")
    cy.takeScreenshot("proposal_in_list")
  })

  it("should navigate to proposal and verify initial state", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    expect(proposalUuid).to.exist

    cy.visit(`/credit-facility-proposals/${proposalUuid}`)
    cy.contains("$5,000").should("be.visible")
    cy.takeScreenshot("06_visit_proposal_page")

    cy.get('[data-testid="collateral-to-reach-target"]').should("be.visible")
    cy.get('[data-testid="update-collateral-button"]').should("be.visible")
  })

  it("should calculate and store required collateral amount", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    expect(proposalUuid).to.exist

    cy.visit(`/credit-facility-proposals/${proposalUuid}`)
    cy.wait(2000)

    cy.get('[data-testid="collateral-to-reach-target"]')
      .should("be.visible")
      .invoke("text")
      .then((collateralValue) => {
        const numericValue = parseFloat(collateralValue.split(" ")[0])
        Cypress.env("requiredCollateralAmount", numericValue)
        cy.log(`Required collateral amount: ${numericValue}`)
      })
  })

  it("should open collateral update form and enter required amount", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    const requiredAmount = Cypress.env("requiredCollateralAmount")
    expect(proposalUuid).to.exist
    expect(requiredAmount).to.exist

    cy.visit(`/credit-facility-proposals/${proposalUuid}`)

    cy.get('[data-testid="update-collateral-button"]').should("be.visible").click()
    cy.takeScreenshot("07_click_update_collateral_button")

    cy.get('[data-testid="new-collateral-input"]')
      .should("be.visible")
      .clear()
      .type(requiredAmount.toString())
    cy.takeScreenshot("08_enter_new_collateral_value")

    cy.get('[data-testid="proceed-to-confirm-button"]').should("be.visible")
  })

  it("should confirm collateral update", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    const requiredAmount = Cypress.env("requiredCollateralAmount")
    expect(proposalUuid).to.exist
    expect(requiredAmount).to.exist
    cy.visit(`/credit-facility-proposals/${proposalUuid}`)
    cy.get('[data-testid="update-collateral-button"]').should("be.visible").click()
    cy.get('[data-testid="new-collateral-input"]')
      .should("be.visible")
      .clear()
      .type(requiredAmount.toString())

    cy.get('[data-testid="proceed-to-confirm-button"]').should("be.visible")
    cy.takeScreenshot("09_confirm_collateral_update")

    cy.get('[data-testid="proceed-to-confirm-button"]')
      .should("be.visible")
      .then(($el) => {
        $el.on("click", (e) => e.preventDefault())
      })
      .click()

    cy.get('[data-testid="confirm-update-button"]')
      .should("be.visible")
      .click()
      .wait(2000)
    cy.takeScreenshot("10_collateral_updated")
  })

  it("should approve the proposal after collateral update", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    expect(proposalUuid).to.exist

    cy.visit(`/credit-facility-proposals/${proposalUuid}`)
    cy.reload()

    cy.get('[data-testid="approval-process-approve-button"]').should("be.visible")
    cy.takeScreenshot("11_approve_proposal_button")
    cy.get('[data-testid="approval-process-approve-button"]').click()

    cy.wait(2000).then(() => {
      cy.takeScreenshot("12_approve_proposal_dialog")
      cy.get('[data-testid="approval-process-dialog-approve-button"]')
        .should("exist")
        .should("be.visible")
        .click()
      cy.wait(2000)
    })
  })

  it("should verify proposal completion status", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    expect(proposalUuid).to.exist
    cy.visit(`/credit-facility-proposals/${proposalUuid}`)
    cy.wait(2000)
    cy.reload()
    cy.get("[data-testid=proposal-status-badge]")
      .should("be.visible")
      .invoke("text")
      .should("eq", t(CFP + ".status.completed"))
    cy.takeScreenshot("13_proposal_completed_status")
  })

  it("should navigate to created credit facility", () => {
    const proposalUuid = Cypress.env("creditFacilityProposalId")
    expect(proposalUuid).to.exist
    cy.visit(`/credit-facility-proposals/${proposalUuid}`)
    cy.get('[data-testid="view-facility-button"]').should("be.visible")
    cy.takeScreenshot("14_view_facility_button")
    cy.get('[data-testid="view-facility-button"]').click()
    cy.url()
      .should("match", /\/credit-facilities\/\d+$/)
      .then((url) => {
        const publicId = url.split("/").pop() as string
        Cypress.env("creditFacilityPublicId", publicId)
        cy.log(`Credit facility public ID: ${publicId}`)
      })
  })

  it("should verify credit facility is active", () => {
    const publicId = Cypress.env("creditFacilityPublicId")
    expect(publicId).to.exist

    cy.visit(`/credit-facilities/${publicId}`)

    cy.get("[data-testid=credit-facility-status-badge]")
      .should("be.visible")
      .invoke("text")
      .should("eq", t(CF + ".CreditFacilityStatus.active"))
    cy.takeScreenshot("15_verify_active_status")
  })

  it("should show newly created credit facility in the list", () => {
    cy.visit(`/credit-facilities`)
    cy.get('[data-testid="table-row-0"] > :nth-child(7) > a > .gap-2').click()
    cy.contains("$5,000.00").should("be.visible")
    cy.takeScreenshot("16_credit_facility_in_list")
  })

  it("should successfully initiate and confirm a disbursal", () => {
    const publicId = Cypress.env("creditFacilityPublicId")
    expect(publicId).to.exist

    cy.visit(`/credit-facilities/${publicId}`)
    cy.contains("$5,000").should("be.visible")

    cy.get('[data-testid="global-create-button"]').click()
    cy.get('[data-testid="initiate-disbursal-button"]').should("be.visible").click()
    cy.takeScreenshot("17_click_initiate_disbursal_button")

    cy.get('[data-testid="disbursal-amount-input"]')
      .type("1000")
      .should("have.value", "1,000")
    cy.takeScreenshot("18_enter_disbursal_amount")

    cy.get('[data-testid="disbursal-submit-button"]').click()
    cy.takeScreenshot("19_submit_disbursal_request")

    cy.url().should("match", /\/disbursals\/\w+$/)

    cy.takeScreenshot("20_disbursal_page")

    cy.reload()
    cy.get('[data-testid="disbursal-approve-button"]').should("be.visible").click()
    cy.wait(2000).then(() => {
      cy.takeScreenshot("21_1_approve")
      cy.get('[data-testid="approval-process-dialog-approve-button"]')
        .should("be.visible")
        .click()

      cy.wait(2000).then(() => {
        cy.get('[data-testid="disbursal-status-badge"]')
          .should("be.visible")
          .invoke("text")
          .should("eq", t(Disbursals + ".DisbursalStatus.confirmed"))
        cy.takeScreenshot("22_verify_disbursal_status_confirmed")
      })
    })
  })

  it("should show disbursal in the list page", () => {
    cy.visit(`/disbursals`)
    cy.contains("$1,000.00").should("be.visible")
    cy.takeScreenshot("23_disbursal_in_list")
  })
})
