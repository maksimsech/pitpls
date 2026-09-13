import type { Action, Currency, DividendRounding } from '@/bindings'

export type { Action, Currency, DividendRounding }

export const cryptoActionDisplay: Record<Action, string> = {
    FiatBuy: 'Buy',
    FiatSell: 'Sell',
}

export const currencyDisplay: Record<Currency, string> = {
    PLN: 'PLN',
    USD: 'USD',
    EUR: 'EUR',
    GBP: 'GBP',
    CHF: 'CHF',
    AUD: 'AUD',
    CAD: 'CAD',
    NZD: 'NZD',
    SGD: 'SGD',
    HKD: 'HKD',
    THB: 'THB',
    HUF: 'HUF',
    UAH: 'UAH',
    JPY: 'JPY',
    CZK: 'CZK',
    DKK: 'DKK',
    ISK: 'ISK',
    NOK: 'NOK',
    SEK: 'SEK',
    RON: 'RON',
    BGN: 'BGN',
    TRY: 'TRY',
    ILS: 'ILS',
    CLP: 'CLP',
    PHP: 'PHP',
    MXN: 'MXN',
    ZAR: 'ZAR',
    BRL: 'BRL',
    MYR: 'MYR',
    IDR: 'IDR',
    INR: 'INR',
    KRW: 'KRW',
    CNY: 'CNY',
    XDR: 'XDR',
}

export const dividendRoundingDisplay: Record<DividendRounding, string> = {
    SumToGroszy: 'Sum to groszy',
    SumToPayToZlote: 'Sum to pay to złote',
    SumBothToZlote: 'Sum both to złote',
    AllToZlote: 'All to złote',
}
