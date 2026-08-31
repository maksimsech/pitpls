import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
} from '@/components/ui/tooltip'

const CURRENCY_SYMBOLS: Record<string, string> = {
    USD: '$',
    EUR: '€',
    GBP: '£',
    PLN: 'zł',
    JPY: '¥',
    CHF: 'CHF',
    CAD: 'C$',
    AUD: 'A$',
}

const MAX_DECIMALS = 5

function trimZeros(value: string) {
    if (!value.includes('.')) return value
    return value.replace(/\.?0+$/, '')
}

function formatWithSymbol(value: string, currency: string) {
    const symbol = CURRENCY_SYMBOLS[currency.toLocaleUpperCase()]
    if (!symbol) return `${value} ${currency}`
    if (symbol === 'zł') return `${value} zł`
    return `${symbol}${value}`
}

type CurrencyValueProps = {
    value: string
    currency: string
}

function CurrencyValue({ value, currency }: CurrencyValueProps) {
    const trimmed = trimZeros(value)
    const dotIdx = trimmed.indexOf('.')
    const decimals = dotIdx === -1 ? 0 : trimmed.length - dotIdx - 1

    if (decimals <= MAX_DECIMALS) {
        return <>{formatWithSymbol(trimmed, currency)}</>
    }

    const truncated = `${trimmed.slice(0, dotIdx + 1 + MAX_DECIMALS)}..`

    return (
        <Tooltip>
            <TooltipTrigger asChild>
                <span className='cursor-help underline decoration-dotted underline-offset-2'>
                    {formatWithSymbol(truncated, currency)}
                </span>
            </TooltipTrigger>
            <TooltipContent className='font-mono'>
                {formatWithSymbol(trimmed, currency)}
            </TooltipContent>
        </Tooltip>
    )
}

export { CurrencyValue }
