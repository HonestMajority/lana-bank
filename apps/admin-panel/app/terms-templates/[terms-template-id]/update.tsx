"use client"

import { useTranslations } from "next-intl"
import React, { useState, useEffect } from "react"
import { gql } from "@apollo/client"
import { toast } from "sonner"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@lana/web/ui/dialog"

import { Input } from "@lana/web/ui/input"
import { Button } from "@lana/web/ui/button"
import { Label } from "@lana/web/ui/label"
import { Checkbox } from "@lana/web/ui/check-box"

import {
  useUpdateTermsTemplateMutation,
  TermsTemplateFieldsFragment,
} from "@/lib/graphql/generated"
import { DEFAULT_TERMS } from "@/lib/constants/terms"
import { getCvlValue } from "@/lib/utils"

gql`
  mutation UpdateTermsTemplate($input: TermsTemplateUpdateInput!) {
    termsTemplateUpdate(input: $input) {
      termsTemplate {
        ...TermsTemplateFields
      }
    }
  }
`

type UpdateTermsTemplateDialogProps = {
  setOpenUpdateTermsTemplateDialog: (isOpen: boolean) => void
  openUpdateTermsTemplateDialog: boolean
  termsTemplate: TermsTemplateFieldsFragment
}

export const UpdateTermsTemplateDialog: React.FC<UpdateTermsTemplateDialogProps> = ({
  setOpenUpdateTermsTemplateDialog,
  openUpdateTermsTemplateDialog,
  termsTemplate,
}) => {
  const t = useTranslations("TermsTemplates.TermsTemplateDetails.UpdateTermsTemplate")

  const [updateTermsTemplate, { loading, error: updateTermsTemplateError }] =
    useUpdateTermsTemplateMutation()

  const [formValues, setFormValues] = useState({
    name: termsTemplate.name,
    annualRate: termsTemplate.values.annualRate.toString(),
    durationUnits: termsTemplate.values.duration.units.toString(),
    liquidationCvl: termsTemplate.values.liquidationCvl.toString(),
    marginCallCvl: termsTemplate.values.marginCallCvl.toString(),
    initialCvl: termsTemplate.values.initialCvl.toString(),
    oneTimeFeeRate: termsTemplate.values.oneTimeFeeRate.toString(),
    disburseAllAtActivation: termsTemplate.values.disburseAllAtActivation,
  })

  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (openUpdateTermsTemplateDialog) {
      setFormValues({
        name: termsTemplate.name,
        annualRate: termsTemplate.values.annualRate.toString(),
        durationUnits: termsTemplate.values.duration.units.toString(),
        liquidationCvl: getCvlValue(termsTemplate.values.liquidationCvl).toString(),
        marginCallCvl: getCvlValue(termsTemplate.values.marginCallCvl).toString(),
        initialCvl: getCvlValue(termsTemplate.values.initialCvl).toString(),
        oneTimeFeeRate: termsTemplate.values.oneTimeFeeRate.toString(),
        disburseAllAtActivation:
          termsTemplate.values.disburseAllAtActivation,
      })
    }
  }, [openUpdateTermsTemplateDialog, termsTemplate])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target
    setFormValues((prevValues) => ({
      ...prevValues,
      [name]: value,
    }))
  }

  const handleCheckboxChange = (checked: boolean) => {
    setFormValues((prevValues) => ({
      ...prevValues,
      disburseAllAtActivation: checked,
    }))
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)

    try {
      const { data } = await updateTermsTemplate({
        variables: {
          input: {
            id: termsTemplate.termsId,
            annualRate: formValues.annualRate,
            accrualCycleInterval: DEFAULT_TERMS.ACCRUAL_CYCLE_INTERVAL,
            accrualInterval: DEFAULT_TERMS.ACCRUAL_INTERVAL,
            duration: {
              period: DEFAULT_TERMS.DURATION_PERIOD,
              units: parseInt(formValues.durationUnits),
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
            liquidationCvl: formValues.liquidationCvl,
            marginCallCvl: formValues.marginCallCvl,
            initialCvl: formValues.initialCvl,
            oneTimeFeeRate: formValues.oneTimeFeeRate,
            disburseAllAtActivation: formValues.disburseAllAtActivation,
          },
        },
      })
      if (data?.termsTemplateUpdate.termsTemplate) {
        toast.success(t("success.updated"))
        setOpenUpdateTermsTemplateDialog(false)
      } else {
        throw new Error(t("errors.updateFailed"))
      }
    } catch (err) {
      console.error("Error updating Terms Template:", err)
      if (err instanceof Error) {
        setError(err.message)
      } else if (updateTermsTemplateError?.message) {
        setError(updateTermsTemplateError.message)
      } else {
        setError(t("errors.general"))
      }
      toast.error(t("errors.updateFailed"))
    }
  }

  const resetForm = () => {
    setFormValues({
      name: termsTemplate.name,
      annualRate: termsTemplate.values.annualRate.toString(),
      durationUnits: termsTemplate.values.duration.units.toString(),
      liquidationCvl: termsTemplate.values.liquidationCvl.toString(),
      marginCallCvl: termsTemplate.values.marginCallCvl.toString(),
      initialCvl: termsTemplate.values.initialCvl.toString(),
      oneTimeFeeRate: termsTemplate.values.oneTimeFeeRate.toString(),
      disburseAllAtActivation: termsTemplate.values.disburseAllAtActivation,
    })
    setError(null)
  }

  return (
    <Dialog
      open={openUpdateTermsTemplateDialog}
      onOpenChange={(isOpen) => {
        setOpenUpdateTermsTemplateDialog(isOpen)
        if (!isOpen) {
          resetForm()
        }
      }}
    >
      <DialogContent className="max-w-[38rem]">
        <DialogHeader>
          <DialogTitle>{t("title", { name: formValues.name })}</DialogTitle>
          <DialogDescription>{t("description")}</DialogDescription>
        </DialogHeader>
        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <div className="grid auto-rows-fr sm:grid-cols-2 gap-4">
            <div className="space-y-4">
              <div>
                <Label htmlFor="annualRate">{t("fields.annualRate")}</Label>
                <Input
                  data-testid="terms-template-annual-rate-input"
                  id="annualRate"
                  name="annualRate"
                  type="number"
                  required
                  placeholder={t("placeholders.annualRate")}
                  value={formValues.annualRate}
                  onChange={handleChange}
                />
              </div>
              <div>
                <Label>{t("fields.duration")}</Label>
                <div className="flex gap-2 items-center">
                  <Input
                    type="number"
                    name="durationUnits"
                    value={formValues.durationUnits}
                    onChange={handleChange}
                    placeholder={t("placeholders.durationUnits")}
                    min={0}
                    required
                  />
                  <div className="p-1.5 bg-input-text rounded-md px-4">
                    {t("fields.months")}
                  </div>
                </div>
              </div>
              <div>
                <Label htmlFor="oneTimeFeeRate">{t("fields.oneTimeFeeRate")}</Label>
                <Input
                  id="oneTimeFeeRate"
                  name="oneTimeFeeRate"
                  type="number"
                  required
                  placeholder={t("placeholders.oneTimeFeeRate")}
                  value={formValues.oneTimeFeeRate}
                  onChange={handleChange}
                />
              </div>
              <div className="flex items-center space-x-2">
                <Checkbox
                  id="disburseAllAtActivation"
                  checked={formValues.disburseAllAtActivation}
                  onCheckedChange={handleCheckboxChange}
                  data-testid="terms-template-update-disburse-all-checkbox"
                />
                <Label htmlFor="disburseAllAtActivation">
                  {t("fields.disburseAllAtActivation")}
                </Label>
              </div>
            </div>
            <div className="space-y-4">
              <div>
                <Label htmlFor="initialCvl">{t("fields.initialCvl")}</Label>
                <Input
                  id="initialCvl"
                  name="initialCvl"
                  type="number"
                  required
                  placeholder={t("placeholders.initialCvl")}
                  value={formValues.initialCvl}
                  onChange={handleChange}
                />
              </div>
              <div>
                <Label htmlFor="marginCallCvl">{t("fields.marginCallCvl")}</Label>
                <Input
                  id="marginCallCvl"
                  name="marginCallCvl"
                  type="number"
                  required
                  placeholder={t("placeholders.marginCallCvl")}
                  value={formValues.marginCallCvl}
                  onChange={handleChange}
                />
              </div>
              <div>
                <Label htmlFor="liquidationCvl">{t("fields.liquidationCvl")}</Label>
                <Input
                  id="liquidationCvl"
                  name="liquidationCvl"
                  type="number"
                  required
                  placeholder={t("placeholders.liquidationCvl")}
                  value={formValues.liquidationCvl}
                  onChange={handleChange}
                />
              </div>
            </div>
          </div>
          {error && <p className="text-destructive">{error}</p>}
          <DialogFooter>
            <Button
              type="submit"
              loading={loading}
              data-testid="terms-template-update-submit-button"
            >
              {t("buttons.submit")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
