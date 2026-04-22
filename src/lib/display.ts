import type { Action, Country, Currency } from "@/bindings";

export type { Action, Country, Currency };

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
