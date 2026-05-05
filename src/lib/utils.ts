import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs))
}

export const NO_RATES_AVAILABLE_ERROR = 'There is no rates available.'

export function formatError(e: unknown) {
    return e instanceof Error ? e.message : String(e)
}

export function today() {
    return new Date().toISOString().slice(0, 10)
}
