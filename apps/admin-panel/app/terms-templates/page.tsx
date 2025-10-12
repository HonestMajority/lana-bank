"use client"

import { useTranslations } from "next-intl"
import React, { useState } from "react"
import { gql } from "@apollo/client"

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@lana/web/ui/card"

import DataTable, { Column } from "../../components/data-table"

import {
  TermsTemplateFieldsFragment,
  TermsTemplatesQuery,
  useTermsTemplatesQuery,
} from "@/lib/graphql/generated"
import { PeriodLabel } from "@/app/credit-facilities/label"
import { UpdateTermsTemplateDialog } from "@/app/terms-templates/[terms-template-id]/update"
import { formatCvl } from "@/lib/utils"

gql`
  fragment TermsTemplateFields on TermsTemplate {
    id
    name
    termsId
    createdAt
    userCanUpdateTermsTemplate
    values {
      annualRate
      liquidationCvl {
        __typename
        ... on FiniteCVLPct {
          value
        }
        ... on InfiniteCVLPct {
          isInfinite
        }
      }
      marginCallCvl {
        __typename
        ... on FiniteCVLPct {
          value
        }
        ... on InfiniteCVLPct {
          isInfinite
        }
      }
      initialCvl {
        __typename
        ... on FiniteCVLPct {
          value
        }
        ... on InfiniteCVLPct {
          isInfinite
        }
      }
      oneTimeFeeRate
      disburseAllAtActivation
      duration {
        period
        units
      }
    }
  }

  query TermsTemplates {
    termsTemplates {
      ...TermsTemplateFields
    }
  }
`

const columns = (
  t: ReturnType<typeof useTranslations>,
  commonT: ReturnType<typeof useTranslations>,
): Column<NonNullable<TermsTemplatesQuery["termsTemplates"]>[number]>[] => [
  {
    key: "name",
    header: t("table.headers.name"),
  },
  {
    key: "values",
    header: t("table.headers.duration"),
    render: (values) => (
      <>
        {String(values.duration.units)} <PeriodLabel period={values.duration.period} />
      </>
    ),
  },
  {
    key: "values",
    header: t("table.headers.annualRate"),
    render: (values) => `${values.annualRate}%`,
  },
  {
    key: "values",
    header: t("table.headers.initialCvl"),
    render: (values) => formatCvl(values.initialCvl),
  },
  {
    key: "values",
    header: t("table.headers.marginCallCvl"),
    render: (values) => formatCvl(values.marginCallCvl),
  },
  {
    key: "values",
    header: t("table.headers.liquidationCvl"),
    render: (values) => formatCvl(values.liquidationCvl),
  },
  {
    key: "values",
    header: t("table.headers.disburseAllAtActivation"),
    render: (values) => (values.disburseAllAtActivation ? commonT("yes") : commonT("no")),
  },
]

function TermPage() {
  const t = useTranslations("TermsTemplates")
  const commonT = useTranslations("Common")

  const { data, loading, error } = useTermsTemplatesQuery()
  const [openUpdateTermsTemplateDialog, setOpenUpdateTermsTemplateDialog] =
    useState<TermsTemplateFieldsFragment | null>(null)

  if (error) {
    return (
      <Card>
        <CardContent>
          <p className="text-destructive mt-6">{t("errors.general")}</p>
        </CardContent>
      </Card>
    )
  }

  return (
    <main>
      {openUpdateTermsTemplateDialog && (
        <UpdateTermsTemplateDialog
          termsTemplate={openUpdateTermsTemplateDialog}
          openUpdateTermsTemplateDialog={Boolean(openUpdateTermsTemplateDialog)}
          setOpenUpdateTermsTemplateDialog={() => setOpenUpdateTermsTemplateDialog(null)}
        />
      )}
      <Card>
        <CardHeader>
          <CardTitle>{t("title")}</CardTitle>
          <CardDescription>{t("description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <DataTable
            data={data?.termsTemplates || []}
            columns={columns(t, commonT)}
            loading={loading}
            navigateTo={(template) => `/terms-templates/${template.termsId}`}
          />
        </CardContent>
      </Card>
    </main>
  )
}

export default TermPage
