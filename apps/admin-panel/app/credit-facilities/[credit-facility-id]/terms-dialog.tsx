"use client"

import React from "react"
import { useTranslations } from "next-intl"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@lana/web/ui/dialog"

import { formatDate } from "@lana/web/utils"

import { GetCreditFacilityLayoutDetailsQuery } from "@/lib/graphql/generated"
import { PeriodLabel } from "@/app/credit-facilities/label"
import { DetailsCard, DetailItemProps } from "@/components/details"
import { formatCvl } from "@/lib/utils"

type CreditFacilityTermsDialogProps = {
  openTermsDialog: boolean
  setOpenTermsDialog: (isOpen: boolean) => void
  creditFacility: NonNullable<
    GetCreditFacilityLayoutDetailsQuery["creditFacilityByPublicId"]
  >
}

export const CreditFacilityTermsDialog: React.FC<CreditFacilityTermsDialogProps> = ({
  openTermsDialog,
  setOpenTermsDialog,
  creditFacility,
}) => {
  const t = useTranslations("CreditFacilities.CreditFacilityDetails.TermsDialog")
  const commonT = useTranslations("Common")

  const effectiveRate =
    Number(creditFacility.creditFacilityTerms.annualRate) +
    Number(creditFacility.creditFacilityTerms.oneTimeFeeRate)

  const details: DetailItemProps[] = [
    {
      label: t("details.duration"),
      value: (
        <>
          {creditFacility.creditFacilityTerms.duration.units}{" "}
          <PeriodLabel period={creditFacility.creditFacilityTerms.duration.period} />
        </>
      ),
    },
    {
      label: t("details.interestRate"),
      value: `${creditFacility.creditFacilityTerms.annualRate}%`,
    },
    {
      label: t("details.targetCvl"),
      value: `${formatCvl(creditFacility.creditFacilityTerms.initialCvl)}`,
    },
    {
      label: t("details.marginCallCvl"),
      value: `${formatCvl(creditFacility.creditFacilityTerms.marginCallCvl)}`,
    },
    {
      label: t("details.liquidationCvl"),
      value: `${formatCvl(creditFacility.creditFacilityTerms.liquidationCvl)}`,
    },
    {
      label: t("details.dateCreated"),
      value: formatDate(creditFacility.activatedAt),
    },
    {
      label: t("details.structuringFeeRate"),
      value: `${creditFacility.creditFacilityTerms.oneTimeFeeRate}%`,
    },
    {
      label: t("details.disburseAllAtActivation"),
      value: creditFacility.creditFacilityTerms.disburseAllAtActivation
        ? commonT("yes")
        : commonT("no"),
    },
    { label: t("details.effectiveRate"), value: `${effectiveRate}%` },
  ]

  return (
    <Dialog open={openTermsDialog} onOpenChange={setOpenTermsDialog}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("title")}</DialogTitle>
        </DialogHeader>
        <div className="py-2">
          <DetailsCard columns={2} variant="container" details={details} />
        </div>
      </DialogContent>
    </Dialog>
  )
}
