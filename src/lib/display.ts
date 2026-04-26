import type { Action, Country, Currency, DividendRounding } from "@/bindings";

export type { Action, Country, Currency, DividendRounding };

export const cryptoActionDisplay: Record<Action, string> = {
    FiatBuy: "Buy",
    FiatSell: "Sell",
};

export const countryDisplay: Record<Country, string> = {
    Japan: "Japan",
    USA: "USA",
};

export const currencyDisplay: Record<Currency, string> = {
    EUR: "EUR",
    USD: "USD",
    PLN: "PLN",
};

export const dividendRoundingDisplay: Record<DividendRounding, string> = {
    SumToGroszy: "Sum to groszy",
    SumToZlote: "Sum to złote",
    AllToZlote: "All to złote",
};
