import { useState } from 'react'

import { type CalculatedDividend, commands, type Currency } from '@/bindings'
import {
    CurrencyAmountField,
    IdField,
    LabeledInput,
} from '@/components/form-fields'
import { Button } from '@/components/ui/button'
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog'
import { formatError, today } from '@/lib/utils'

type Props = {
    onClose: () => void
    onCreated: () => void
    dividend?: CalculatedDividend | null
}

function DividendFormModal({ onClose, onCreated, dividend }: Props) {
    const isEdit = !!dividend
    const [id, setId] = useState(dividend?.id ?? '')
    const [date, setDate] = useState(dividend?.date ?? today())
    const [ticker, setTicker] = useState(dividend?.ticker ?? '')
    const [value, setValue] = useState(dividend?.value.value ?? '')
    const [valueCurrency, setValueCurrency] = useState<Currency>(
        dividend?.value.currency ?? 'USD',
    )
    const [taxPaid, setTaxPaid] = useState(dividend?.tax_paid.value ?? '')
    const [taxPaidCurrency, setTaxPaidCurrency] = useState<Currency>(
        dividend?.tax_paid.currency ?? 'USD',
    )
    const [country, setCountry] = useState<string>(dividend?.country ?? 'US')
    const [provider, setProvider] = useState(dividend?.provider ?? '')
    const [submitting, setSubmitting] = useState(false)
    const [error, setError] = useState<string | null>(null)

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault()
        setSubmitting(true)
        setError(null)
        try {
            const base = {
                date,
                ticker: ticker.trim(),
                value: value.trim(),
                value_currency: valueCurrency,
                tax_paid: taxPaid.trim(),
                tax_paid_currency: taxPaidCurrency,
                country: country.trim().toUpperCase(),
                provider: provider.trim(),
            }
            if (isEdit) {
                const result = await commands.updateDividend({
                    ...base,
                    id: id.trim(),
                })
                if (result.status === 'error') {
                    setError(result.error)
                    return
                }
            } else {
                const result = await commands.createDividend({
                    ...base,
                    id: id.trim() === '' ? null : id.trim(),
                })
                if (result.status === 'error') {
                    setError(result.error)
                    return
                }
            }
            onCreated()
            onClose()
        } catch (e) {
            setError(formatError(e))
        } finally {
            setSubmitting(false)
        }
    }

    return (
        <Dialog open onOpenChange={(open) => !open && onClose()}>
            <DialogContent className='sm:max-w-lg'>
                <DialogHeader>
                    <DialogTitle>
                        {isEdit ? 'Edit dividend' : 'Add dividend'}
                    </DialogTitle>
                </DialogHeader>
                <form onSubmit={handleSubmit} className='flex flex-col gap-3'>
                    <IdField
                        id='dividend-id'
                        value={id}
                        onChange={setId}
                        isEdit={isEdit}
                    />
                    <LabeledInput
                        id='dividend-date'
                        label='Date'
                        type='date'
                        value={date}
                        onChange={(e) => setDate(e.target.value)}
                        required
                    />
                    <LabeledInput
                        id='dividend-ticker'
                        label='Ticker'
                        value={ticker}
                        onChange={(e) => setTicker(e.target.value)}
                        required
                    />
                    <CurrencyAmountField
                        id='dividend-value'
                        label='Value'
                        value={value}
                        onValueChange={setValue}
                        currency={valueCurrency}
                        onCurrencyChange={setValueCurrency}
                    />
                    <CurrencyAmountField
                        id='dividend-tax'
                        label='Tax paid'
                        value={taxPaid}
                        onValueChange={setTaxPaid}
                        currency={taxPaidCurrency}
                        onCurrencyChange={setTaxPaidCurrency}
                    />
                    <LabeledInput
                        id='dividend-country'
                        label='Country code'
                        value={country}
                        onChange={(e) =>
                            setCountry(e.target.value.toUpperCase().slice(0, 2))
                        }
                        className='font-mono uppercase'
                        maxLength={2}
                        pattern='[A-Za-z]{2}'
                        autoCapitalize='characters'
                        required
                    />
                    <LabeledInput
                        id='dividend-provider'
                        label='Provider'
                        value={provider}
                        onChange={(e) => setProvider(e.target.value)}
                        required
                    />

                    {error && (
                        <p className='text-destructive text-sm wrap-break-word'>
                            {error}
                        </p>
                    )}

                    <DialogFooter>
                        <Button
                            type='button'
                            variant='outline'
                            onClick={onClose}
                            disabled={submitting}
                        >
                            Cancel
                        </Button>
                        <Button type='submit' disabled={submitting}>
                            {submitting ? 'Saving…' : 'Save'}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    )
}

export { DividendFormModal }
