import React, { useEffect, useState } from "react"
import { gql } from "@apollo/client"
import { toast } from "sonner"
import { PiPencilSimpleLineLight } from "react-icons/pi"
import { useTranslations } from "next-intl"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@lana/web/ui/dialog"
import { Input } from "@lana/web/ui/input"
import { Label } from "@lana/web/ui/label"
import { Checkbox } from "@lana/web/ui/check-box"

import { Button } from "@lana/web/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@lana/web/ui/select"

import { useCreateContext } from "@/app/create"

import {
  useCreditFacilityProposalCreateMutation,
  useGetRealtimePriceUpdatesQuery,
  useTermsTemplatesQuery,
  useCustodiansQuery,
  CreditFacilityProposalCreateInput,
} from "@/lib/graphql/generated"
import {
  currencyConverter,
  calculateInitialCollateralRequired,
  getCvlValue,
} from "@/lib/utils"
import { DetailItem, DetailsGroup } from "@/components/details"
import Balance from "@/components/balance/balance"
import { useModalNavigation } from "@/hooks/use-modal-navigation"
import { Satoshis } from "@/types"
import { DEFAULT_TERMS } from "@/lib/constants/terms"

const DEFAULT_CUSTODIAN = "manual-custodian"

gql`
  mutation CreditFacilityProposalCreate($input: CreditFacilityProposalCreateInput!) {
    creditFacilityProposalCreate(input: $input) {
      creditFacilityProposal {
        id
        creditFacilityProposalId
        customer {
          id
          email
          creditFacilityProposals {
            id
          }
        }
      }
    }
  }
`

type CreateCreditFacilityProposalDialogProps = {
  setOpenCreateCreditFacilityProposalDialog: (isOpen: boolean) => void
  openCreateCreditFacilityProposalDialog: boolean
  customerId: string
  disbursalCreditAccountId: string
}

const initialFormValues = {
  facility: "0",
  custodianId: DEFAULT_CUSTODIAN,
  annualRate: "",
  liquidationCvl: "",
  marginCallCvl: "",
  initialCvl: "",
  durationUnits: "",
  oneTimeFeeRate: "",
  disburseFullAmountOnActivation: false,
}

export const CreateCreditFacilityProposalDialog: React.FC<
  CreateCreditFacilityProposalDialogProps
> = ({
  setOpenCreateCreditFacilityProposalDialog,
  openCreateCreditFacilityProposalDialog,
  customerId,
  disbursalCreditAccountId,
}) => {
  const t = useTranslations("CreditFacilityProposals.CreateCreditFacilityProposal")
  const commonT = useTranslations("Common")

  const handleCloseDialog = () => {
    setOpenCreateCreditFacilityProposalDialog(false)
    resetForm()
    reset()
  }

  const { navigate, isNavigating } = useModalNavigation({
    closeModal: handleCloseDialog,
  })
  const { customer } = useCreateContext()

  const { data: priceInfo } = useGetRealtimePriceUpdatesQuery({
    fetchPolicy: "cache-only",
  })

  const { data: termsTemplatesData, loading: termsTemplatesLoading } =
    useTermsTemplatesQuery()
  const { data: custodiansData, loading: custodiansLoading } = useCustodiansQuery({
    variables: { first: 50 },
  })
  const [createCreditFacility, { loading, error, reset }] =
    useCreditFacilityProposalCreateMutation()

  const isLoading = loading || isNavigating

  const [useTemplateTerms, setUseTemplateTerms] = useState(true)
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("")

  const [formValues, setFormValues] = useState(initialFormValues)

  useEffect(() => {
    if (
      termsTemplatesData?.termsTemplates &&
      termsTemplatesData.termsTemplates.length > 0
    ) {
      const latestTemplate = termsTemplatesData.termsTemplates[0]
      setSelectedTemplateId(latestTemplate.id)
      setFormValues((prevValues) => ({
        ...prevValues,
        annualRate: latestTemplate.values.annualRate.toString(),
        liquidationCvl: getCvlValue(latestTemplate.values.liquidationCvl).toString(),
        marginCallCvl: getCvlValue(latestTemplate.values.marginCallCvl).toString(),
        initialCvl: getCvlValue(latestTemplate.values.initialCvl).toString(),
        durationUnits: latestTemplate.values.duration.units.toString(),
        oneTimeFeeRate: latestTemplate.values.oneTimeFeeRate.toString(),
        disburseFullAmountOnActivation: Boolean(
          (latestTemplate.values as unknown as {
            disburseFullAmountOnActivation?: boolean
          }).disburseFullAmountOnActivation,
        ),
      }))
    }
  }, [termsTemplatesData])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target
    setFormValues((prevValues) => ({
      ...prevValues,
      [name]: value,
    }))
    if (name === "facility") return
    setSelectedTemplateId("")
  }

  const handleDisburseFullAmountChange = (checked: boolean) => {
    setFormValues((prevValues) => ({
      ...prevValues,
      disburseFullAmountOnActivation: checked,
    }))
    setSelectedTemplateId("")
  }

  const handleTemplateChange = (templateId: string) => {
    setSelectedTemplateId(templateId)
    const selectedTemplate = termsTemplatesData?.termsTemplates.find(
      (t) => t.id === templateId,
    )
    if (selectedTemplate) {
      setFormValues((prevValues) => ({
        ...prevValues,
        annualRate: selectedTemplate.values.annualRate.toString(),
        liquidationCvl: getCvlValue(selectedTemplate.values.liquidationCvl).toString(),
        marginCallCvl: getCvlValue(selectedTemplate.values.marginCallCvl).toString(),
        initialCvl: getCvlValue(selectedTemplate.values.initialCvl).toString(),
        durationUnits: selectedTemplate.values.duration.units.toString(),
        oneTimeFeeRate: selectedTemplate.values.oneTimeFeeRate.toString(),
        disburseFullAmountOnActivation: Boolean(
          (selectedTemplate.values as unknown as {
            disburseFullAmountOnActivation?: boolean
          }).disburseFullAmountOnActivation,
        ),
      }))
    }
  }

  const handleCreateCreditFacility = async (event: React.FormEvent) => {
    event.preventDefault()
    const {
      facility,
      custodianId,
      annualRate,
      liquidationCvl,
      marginCallCvl,
      initialCvl,
      durationUnits,
      oneTimeFeeRate,
      disburseFullAmountOnActivation,
    } = formValues

    if (
      !facility ||
      !annualRate ||
      !liquidationCvl ||
      !marginCallCvl ||
      !initialCvl ||
      !durationUnits ||
      !oneTimeFeeRate
    ) {
      toast.error(t("form.messages.fillAllFields"))
      return
    }

    try {
      await createCreditFacility({
        variables: {
          input: ({
            disbursalCreditAccountId,
            customerId,
            facility: currencyConverter.usdToCents(Number(facility)),
            custodianId: custodianId === DEFAULT_CUSTODIAN ? null : custodianId,
            terms: {
              annualRate: parseFloat(annualRate),
              accrualCycleInterval: DEFAULT_TERMS.ACCRUAL_CYCLE_INTERVAL,
              accrualInterval: DEFAULT_TERMS.ACCRUAL_INTERVAL,
              liquidationCvl: parseFloat(liquidationCvl),
              marginCallCvl: parseFloat(marginCallCvl),
              initialCvl: parseFloat(initialCvl),
              oneTimeFeeRate: parseFloat(oneTimeFeeRate),
              disburseFullAmountOnActivation,
              duration: {
                units: parseInt(durationUnits),
                period: DEFAULT_TERMS.DURATION_PERIOD,
              },
              interestDueDurationFromAccrual: {
                period: DEFAULT_TERMS.INTEREST_DUE_DURATION_FROM_ACCRUAL.PERIOD,
                units: DEFAULT_TERMS.INTEREST_DUE_DURATION_FROM_ACCRUAL.UNITS,
              },
              obligationOverdueDurationFromDue: {
                period: DEFAULT_TERMS.OBLIGATION_OVERDUE_DURATION_FROM_DUE.PERIOD,
                units: DEFAULT_TERMS.OBLIGATION_OVERDUE_DURATION_FROM_DUE.UNITS,
              },
              obligationLiquidationDurationFromDue: {
                period: DEFAULT_TERMS.OBLIGATION_LIQUIDATION_DURATION_FROM_DUE.PERIOD,
                units: DEFAULT_TERMS.OBLIGATION_LIQUIDATION_DURATION_FROM_DUE.UNITS,
              },
            },
          }) as unknown as CreditFacilityProposalCreateInput,
        },
        onCompleted: (data) => {
          if (data.creditFacilityProposalCreate) {
            toast.success(t("form.messages.success"))
            navigate(
              `/credit-facility-proposals/${data?.creditFacilityProposalCreate.creditFacilityProposal.creditFacilityProposalId}`,
            )
          }
        },
      })
    } catch (err) {
      console.error(err)
    }
  }

  const resetForm = () => {
    setUseTemplateTerms(true)
    if (
      termsTemplatesData?.termsTemplates &&
      termsTemplatesData.termsTemplates.length > 0
    ) {
      const latestTemplate = termsTemplatesData.termsTemplates[0]
      setSelectedTemplateId(latestTemplate.id)
      setFormValues({
        facility: "0",
        custodianId: DEFAULT_CUSTODIAN,
        annualRate: latestTemplate.values.annualRate.toString(),
        liquidationCvl: getCvlValue(latestTemplate.values.liquidationCvl).toString(),
        marginCallCvl: getCvlValue(latestTemplate.values.marginCallCvl).toString(),
        initialCvl: getCvlValue(latestTemplate.values.initialCvl).toString(),
        durationUnits: latestTemplate.values.duration.units.toString(),
        oneTimeFeeRate: latestTemplate.values.oneTimeFeeRate?.toString(),
        disburseFullAmountOnActivation: Boolean(
          (latestTemplate.values as unknown as {
            disburseFullAmountOnActivation?: boolean
          }).disburseFullAmountOnActivation,
        ),
      })
    } else {
      setFormValues(initialFormValues)
    }
  }

  useEffect(() => {
    if (openCreateCreditFacilityProposalDialog) {
      resetForm()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openCreateCreditFacilityProposalDialog])

  const collateralRequiredForDesiredFacility = calculateInitialCollateralRequired({
    amount: Number(formValues.facility) || 0,
    initialCvl: Number(formValues.initialCvl) || 0,
    priceInfo: priceInfo,
  })
  return (
    <Dialog
      open={openCreateCreditFacilityProposalDialog}
      onOpenChange={handleCloseDialog}
    >
      <DialogContent className="max-w-[40rem]">
        {customer?.email && (
          <div
            className="absolute -top-6 -left-[1px] bg-primary rounded-tl-md rounded-tr-md text-md px-2 py-1 text-secondary"
            style={{ width: "100.35%" }}
          >
            {t("dialog.customerInfo", { email: customer.email })}
          </div>
        )}
        <DialogHeader>
          <DialogTitle>{t("dialog.title")}</DialogTitle>
          <DialogDescription>{t("dialog.description")}</DialogDescription>
        </DialogHeader>
        <form className="flex flex-col gap-4" onSubmit={handleCreateCreditFacility}>
          <div>
            <Label>{t("form.labels.facilityAmount")}</Label>
            <div className="flex items-center gap-1">
              <Input
                type="number"
                name="facility"
                value={formValues.facility}
                onChange={handleChange}
                placeholder={t("form.placeholders.facilityAmount")}
                min={0}
                data-testid="facility-amount-input"
                required
              />
              <div className="p-1.5 bg-input-text rounded-md px-4">USD</div>
            </div>
          </div>
          {priceInfo && (
            <div className="text-sm ml-1 flex space-x-1 items-center">
              <Balance
                amount={collateralRequiredForDesiredFacility as Satoshis}
                currency="btc"
              />
              <div>{t("form.messages.collateralRequired")} (</div>
              <div>{t("form.messages.btcUsdRate")} </div>
              <Balance amount={priceInfo?.realtimePrice.usdCentsPerBtc} currency="usd" />
              <div>)</div>
            </div>
          )}
          <div>
            <Label>{t("form.labels.custodian")}</Label>
            <Select
              defaultValue={DEFAULT_CUSTODIAN}
              value={formValues.custodianId}
              onValueChange={(value) =>
                setFormValues((prev) => ({ ...prev, custodianId: value }))
              }
              disabled={custodiansLoading}
            >
              <SelectTrigger>
                <SelectValue placeholder={t("form.placeholders.custodian")} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem key={DEFAULT_CUSTODIAN} value={DEFAULT_CUSTODIAN}>
                  {t("form.labels.manualCustodian")}
                </SelectItem>
                {custodiansData?.custodians.edges.map(({ node: custodian }) => (
                  <SelectItem key={custodian.id} value={custodian.custodianId}>
                    {custodian.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {useTemplateTerms && termsTemplatesData?.termsTemplates.length === 0 ? (
            <div className="text-sm mt-1">{t("form.messages.noTemplates")}</div>
          ) : (
            <div>
              <Label>{t("form.labels.termsTemplate")}</Label>
              <Select
                value={selectedTemplateId}
                onValueChange={handleTemplateChange}
                disabled={termsTemplatesLoading}
              >
                <SelectTrigger data-testid="credit-facility-terms-template-select">
                  <SelectValue placeholder={t("form.placeholders.termsTemplate")} />
                </SelectTrigger>
                <SelectContent>
                  {termsTemplatesData?.termsTemplates.map((template) => (
                    <SelectItem key={template.id} value={template.id}>
                      {template.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
          {useTemplateTerms ? (
            <>
              <button
                type="button"
                onClick={() => setUseTemplateTerms(false)}
                className="mt-2 flex items-center space-x-2 ml-2 cursor-pointer text-sm hover:underline w-fit"
              >
                <div>{t("form.labels.creditFacilityTerms")}</div>
                <PiPencilSimpleLineLight className="w-5 h-5 cursor-pointer text-primary" />
              </button>
              <DetailsGroup
                layout="horizontal"
                className="grid auto-rows-fr sm:grid-cols-2"
              >
                <DetailItem
                  label={t("form.labels.interestRate")}
                  value={formValues.annualRate + "%"}
                />
                <DetailItem
                  label={t("form.labels.initialCvl")}
                  value={formValues.initialCvl}
                />
                <DetailItem
                  label={t("form.labels.duration")}
                  value={
                    <>
                      {formValues.durationUnits} {t("form.labels.months")}
                    </>
                  }
                />
                <DetailItem
                  label={t("form.labels.marginCallCvl")}
                  value={formValues.marginCallCvl}
                />
                <DetailItem
                  label={t("form.labels.structuringFeeRate")}
                  value={formValues.oneTimeFeeRate}
                />
                <DetailItem
                  label={t("form.labels.disburseFullAmountOnActivation")}
                  value={
                    formValues.disburseFullAmountOnActivation
                      ? commonT("yes")
                      : commonT("no")
                  }
                />
                <DetailItem
                  label={t("form.labels.liquidationCvl")}
                  value={formValues.liquidationCvl}
                />
              </DetailsGroup>
            </>
          ) : (
            <>
              <div className="grid auto-rows-fr sm:grid-cols-2 gap-4">
                <div>
                  <Label>{t("form.labels.interestRate")}</Label>
                  <Input
                    type="number"
                    name="annualRate"
                    value={formValues.annualRate}
                    onChange={handleChange}
                    placeholder={t("form.placeholders.annualRate")}
                    required
                  />
                </div>
                <div>
                  <Label>{t("form.labels.initialCvl")}</Label>
                  <Input
                    type="number"
                    name="initialCvl"
                    value={formValues.initialCvl}
                    onChange={handleChange}
                    placeholder={t("form.placeholders.initialCvl")}
                    required
                  />
                </div>
                <div>
                  <Label>{t("form.labels.duration")}</Label>
                  <div className="flex gap-2 items-center">
                    <Input
                      type="number"
                      name="durationUnits"
                      value={formValues.durationUnits}
                      onChange={handleChange}
                      placeholder={t("form.placeholders.duration")}
                      min={0}
                      required
                    />
                    <div className="p-1.5 bg-input-text rounded-md px-4">
                      {t("form.labels.months")}
                    </div>
                  </div>
                </div>
                <div>
                  <Label>{t("form.labels.marginCallCvl")}</Label>
                  <Input
                    type="number"
                    name="marginCallCvl"
                    value={formValues.marginCallCvl}
                    onChange={handleChange}
                    placeholder={t("form.placeholders.marginCallCvl")}
                    required
                  />
                </div>
                <div>
                  <Label>{t("form.labels.structuringFeeRate")}</Label>
                  <Input
                    type="number"
                    name="oneTimeFeeRate"
                    value={formValues.oneTimeFeeRate}
                    onChange={handleChange}
                    placeholder={t("form.placeholders.structuringFeeRate")}
                    min={0}
                    required
                  />
                </div>
                <div>
                  <Label>{t("form.labels.liquidationCvl")}</Label>
                  <Input
                    type="number"
                    name="liquidationCvl"
                    value={formValues.liquidationCvl}
                    onChange={handleChange}
                    placeholder={t("form.placeholders.liquidationCvl")}
                    min={0}
                    required
                  />
                </div>
              </div>
              <div className="flex items-start gap-2">
                <Checkbox
                  id="disburseFullAmountOnActivation"
                  checked={formValues.disburseFullAmountOnActivation}
                  onCheckedChange={(checked) =>
                    handleDisburseFullAmountChange(Boolean(checked))
                  }
                />
                <div className="grid gap-1 text-sm">
                  <Label htmlFor="disburseFullAmountOnActivation">
                    {t("form.labels.disburseFullAmountOnActivation")}
                  </Label>
                  <p className="text-muted-foreground">
                    {t("form.descriptions.disburseFullAmountOnActivation")}
                  </p>
                </div>
              </div>
            </>
          )}
          {error && <span className="text-destructive">{error.message}</span>}
          <DialogFooter className="mt-4">
            {!useTemplateTerms && (
              <Button
                type="button"
                onClick={() => setUseTemplateTerms(true)}
                variant="ghost"
              >
                {commonT("back")}
              </Button>
            )}
            <Button
              disabled={isLoading}
              type="submit"
              loading={isLoading}
              data-testid="create-credit-facility-submit"
            >
              {t("form.buttons.create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
