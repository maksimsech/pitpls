import type { Action, Currency, DividendRounding } from "@/bindings";

export type { Action, Currency, DividendRounding };

export const cryptoActionDisplay: Record<Action, string> = {
    FiatBuy: "Buy",
    FiatSell: "Sell",
};

export const currencyDisplay: Record<Currency, string> = {
    EUR: "EUR",
    USD: "USD",
    PLN: "PLN",
};

export const dividendRoundingDisplay: Record<DividendRounding, string> = {
    SumToGroszy: "Sum to groszy",
    SumToPayToZlote: "Sum to pay to złote",
    SumBothToZlote: "Sum both to złote",
    AllToZlote: "All to złote",
};
