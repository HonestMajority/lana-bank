import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

import {
  ApprovalProcessType,
  ApprovalRules,
  CollateralAction,
  CvlPctDataFragment,
  GetRealtimePriceUpdatesQuery,
} from "./graphql/generated"

import { Satoshis, UsdCents } from "@/types"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export const SATS_PER_BTC = 100_000_000
export const CENTS_PER_USD = 100

export const currencyConverter = {
  centsToUsd: (cents: number) => {
    return Number((cents / CENTS_PER_USD).toFixed(2))
  },

  btcToSatoshi: (btc: number) => {
    return Number((btc * SATS_PER_BTC).toFixed(0)) as Satoshis
  },

  satoshiToBtc: (satoshi: number) => {
    return satoshi / SATS_PER_BTC
  },

  usdToCents: (usd: number) => {
    return Number((usd * CENTS_PER_USD).toFixed(0)) as UsdCents
  },
}

export const formatRole = (role: string) => {
  return role
    .toLowerCase()
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ")
}

export const formatCollateralAction = (collateralAction: CollateralAction) => {
  return collateralAction === CollateralAction.Add ? "(Added)" : "(Removed)"
}

export const formatTransactionType = (typename: string) => {
  return typename
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/^\w/, (c) => c.toUpperCase())
}

export const isEmail = (str: string) => {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
  return emailRegex.test(str)
}

export const calculateInitialCollateralRequired = ({
  amount,
  initialCvl,
  priceInfo,
}: {
  amount: number
  initialCvl: number
  priceInfo: GetRealtimePriceUpdatesQuery | undefined
}) => {
  if (!priceInfo) return 0

  const basisAmountInUsd = amount
  const initialCvlDecimal = initialCvl / 100

  const requiredCollateralInSats =
    (initialCvlDecimal * basisAmountInUsd * SATS_PER_BTC) /
    (priceInfo.realtimePrice.usdCentsPerBtc / CENTS_PER_USD)

  return Math.floor(requiredCollateralInSats)
}

export const formatRule = (rule: ApprovalRules | null | undefined): string => {
  if (!rule) {
    return "No rules defined"
  }

  if (rule.__typename === "CommitteeThreshold") {
    return `${rule.threshold} ${rule.threshold === 1 ? "member" : "members"} required`
  }

  if (rule.__typename === "SystemApproval") {
    return `System ${rule.autoApprove ? "Auto" : "Manual"} Approval`
  }

  return "Unknown rule type"
}

export const formatProcessType = (processType: ApprovalProcessType) => {
  switch (processType) {
    case ApprovalProcessType.CreditFacilityProposalApproval:
      return "Credit Facility Proposal"
    case ApprovalProcessType.WithdrawalApproval:
      return "Withdrawal"
    case ApprovalProcessType.DisbursalApproval:
      return "Disbursal"
  }
}

/**
 * Converts a camelCase string to SCREAMING_SNAKE_CASE.
 *
 * @param input - The camelCase string to convert.
 * @returns The converted SCREAMING_SNAKE_CASE string.
 */
export const camelToScreamingSnake = (input: string): string => {
  if (!input) return ""

  // Insert an underscore before each uppercase letter (except the first character)
  const snakeCase = input.replace(/([a-z0-9])([A-Z])/g, "$1_$2")

  // Convert the entire string to uppercase
  return snakeCase.toUpperCase()
}

export const removeUnderscore = (str: string | undefined) => {
  if (!str) return undefined

  return str
    .toLowerCase()
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ")
}

export const getCurrentLocalDate = (): string => {
  const now = new Date()
  const offset = now.getTimezoneOffset() * 60000
  return new Date(now.getTime() - offset).toISOString().split("T")[0]
}

export const formatCvl = (cvl: CvlPctDataFragment): string =>
  cvl.__typename === "FiniteCVLPct" ? `${Number(cvl.value || 0)}%` : "-"

export const getCvlValue = (cvl: CvlPctDataFragment): number =>
  cvl.__typename === "FiniteCVLPct" ? Number(cvl.value) : Infinity

type ActivationDrawdownConfig = {
  disburseFullAmountOnActivation?: boolean | null
}

const isActivationDrawdownConfig = (
  terms: unknown,
): terms is ActivationDrawdownConfig =>
  typeof terms === "object" &&
  terms !== null &&
  "disburseFullAmountOnActivation" in terms

export const hasActivationDrawdown = (terms: unknown): boolean =>
  isActivationDrawdownConfig(terms)
    ? Boolean(terms.disburseFullAmountOnActivation)
    : false
