import { useState } from 'react'

import { type CalculatedInterest, commands, type Currency } from '@/bindings'
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
    interest?: CalculatedInterest | null
}

function InterestFormModal({ onClose, onCreated, interest }: Props) {
    const isEdit = !!interest
    const [id, setId] = useState(interest?.id ?? '')
    const [date, setDate] = useState(interest?.date ?? today())
    const [value, setValue] = useState(interest?.value.value ?? '')
    const [valueCurrency, setValueCurrency] = useState<Currency>(
        interest?.value.currency ?? 'USD',
    )
    const [provider, setProvider] = useState(interest?.provider ?? '')
    const [submitting, setSubmitting] = useState(false)
    const [error, setError] = useState<string | null>(null)

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault()
        setSubmitting(true)
        setError(null)
        try {
            const base = {
                date,
                value: value.trim(),
                value_currency: valueCurrency,
                provider: provider.trim(),
            }
            if (isEdit) {
                const result = await commands.updateInterest({
                    ...base,
                    id: id.trim(),
                })
                if (result.status === 'error') {
                    setError(result.error)
                    return
                }
            } else {
                const result = await commands.createInterest({
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
                        {isEdit ? 'Edit interest' : 'Add interest'}
                    </DialogTitle>
                </DialogHeader>
                <form onSubmit={handleSubmit} className='flex flex-col gap-3'>
                    <IdField
                        id='interest-id'
                        value={id}
                        onChange={setId}
                        isEdit={isEdit}
                    />
                    <LabeledInput
                        id='interest-date'
                        label='Date'
                        type='date'
                        value={date}
                        onChange={(e) => setDate(e.target.value)}
                        required
                    />
                    <CurrencyAmountField
                        id='interest-value'
                        label='Value'
                        value={value}
                        onValueChange={setValue}
                        currency={valueCurrency}
                        onCurrencyChange={setValueCurrency}
                    />
                    <LabeledInput
                        id='interest-provider'
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

export { InterestFormModal }
