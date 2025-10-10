import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

import { Period } from "./graphql/generated"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export const currencyConverter = {
  centsToUsd: (cents: number) => {
    return Number((cents / 100).toFixed(2))
  },

  btcToSatoshi: (btc: number) => {
    return Number((btc * 100000000).toFixed(0))
  },

  satoshiToBtc: (satoshi: number) => {
    return satoshi / 100000000
  },

  usdToCents: (usd: number) => {
    return Number((usd * 100).toFixed(0))
  },
}

export const formatCurrency = ({
  amount,
  currency,
}: {
  amount: number
  currency: string
}) => {
  if (currency === "SATS") {
    return (
      new Intl.NumberFormat("en-US", {
        maximumFractionDigits: 0,
        useGrouping: true,
      }).format(amount) + " Sats"
    )
  }

  if (currency === "BTC") {
    return `${amount} BTC`
  }

  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency,
  }).format(amount)
}

export const createErrorResponse = ({
  errorMessage,
  id,
}: {
  errorMessage: string
  id?: number
}) => {
  return {
    data: null,
    error: {
      id,
      message: errorMessage,
    },
  }
}

export const removeUnderscore = (str: string) => {
  return str
    .toLowerCase()
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ")
}

export const formatPeriod = (period: Period) => {
  return period.charAt(0).toUpperCase() + period.slice(1).toLowerCase()
}

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
