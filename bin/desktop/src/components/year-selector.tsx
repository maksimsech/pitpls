import { useCallback, useEffect, useState } from 'react'
import { Plus, Settings2, Trash2 } from 'lucide-react'

import { commands } from '@/bindings'
import { Button } from '@/components/ui/button'
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectSeparator,
    SelectTrigger,
    SelectValue,
} from '@/components/ui/select'
import { useYear } from '@/components/year-provider'
import { formatError } from '@/lib/utils'

const ALL = 'all'
const YEAR_MIN = 1900
const YEAR_MAX = 2100

function YearSelector() {
    const { year, setYear } = useYear()
    const [options, setOptions] = useState<number[]>([])
    const [open, setOpen] = useState(false)
    const [addOpen, setAddOpen] = useState(false)
    const [manageOpen, setManageOpen] = useState(false)
    const [deleteTarget, setDeleteTarget] = useState<number | null>(null)
    const [deleteError, setDeleteError] = useState<string | null>(null)
    const [deleting, setDeleting] = useState(false)

    const refresh = useCallback(async () => {
        const result = await commands.listYears()
        if (result.status === 'ok') {
            setOptions(result.data)
        }
    }, [])

    useEffect(() => {
        void refresh()
    }, [refresh])

    const handleValueChange = (value: string) => {
        if (value === ALL) {
            setYear(null)
        } else {
            setYear(parseInt(value, 10))
        }
    }

    const openAddDialog = () => {
        setOpen(false)
        setManageOpen(false)
        setTimeout(() => setAddOpen(true), 0)
    }

    const openManageDialog = () => {
        setOpen(false)
        setTimeout(() => setManageOpen(true), 0)
    }

    const handleAdded = async (newYear: number) => {
        await refresh()
        setYear(newYear)
    }

    const requestDelete = (value: number) => {
        setDeleteError(null)
        setDeleteTarget(value)
    }

    const confirmDelete = async () => {
        if (deleteTarget === null) return

        setDeleting(true)
        setDeleteError(null)
        try {
            const result = await commands.deleteYear(deleteTarget)
            if (result.status === 'error') {
                setDeleteError(result.error)
                return
            }
            if (year === deleteTarget) setYear(null)
            await refresh()
            setDeleteTarget(null)
        } catch (err) {
            setDeleteError(formatError(err))
        } finally {
            setDeleting(false)
        }
    }

    const currentValue = year === null ? ALL : String(year)

    return (
        <>
            <Select
                value={currentValue}
                onValueChange={handleValueChange}
                open={open}
                onOpenChange={setOpen}
            >
                <SelectTrigger className='min-w-28'>
                    <SelectValue>
                        {year === null ? 'All' : String(year)}
                    </SelectValue>
                </SelectTrigger>
                <SelectContent>
                    <SelectGroup>
                        <SelectItem value={ALL}>All</SelectItem>
                        {options.map((y) => (
                            <SelectItem key={y} value={String(y)}>
                                {y}
                            </SelectItem>
                        ))}
                    </SelectGroup>
                    <SelectSeparator />
                    <div className='flex flex-col gap-1 p-1'>
                        <Button
                            type='button'
                            variant='ghost'
                            size='sm'
                            className='w-full justify-start'
                            onClick={openAddDialog}
                        >
                            <Plus data-icon='inline-start' />
                            Add custom year...
                        </Button>
                        <Button
                            type='button'
                            variant='ghost'
                            size='sm'
                            className='w-full justify-start'
                            onClick={openManageDialog}
                            disabled={options.length === 0}
                        >
                            <Settings2 data-icon='inline-start' />
                            Manage years
                        </Button>
                    </div>
                </SelectContent>
            </Select>

            {addOpen && (
                <AddYearDialog
                    open={addOpen}
                    onOpenChange={setAddOpen}
                    onAdded={handleAdded}
                />
            )}

            {manageOpen && (
                <ManageYearsDialog
                    open={manageOpen}
                    onOpenChange={setManageOpen}
                    years={options}
                    onAddYear={openAddDialog}
                    onRequestDelete={requestDelete}
                />
            )}

            {deleteTarget !== null && (
                <DeleteYearDialog
                    year={deleteTarget}
                    open={deleteTarget !== null}
                    error={deleteError}
                    deleting={deleting}
                    onOpenChange={(nextOpen) => {
                        if (!nextOpen && !deleting) {
                            setDeleteTarget(null)
                            setDeleteError(null)
                        }
                    }}
                    onConfirm={confirmDelete}
                />
            )}
        </>
    )
}

function ManageYearsDialog({
    open,
    onOpenChange,
    years,
    onAddYear,
    onRequestDelete,
}: {
    open: boolean
    onOpenChange: (open: boolean) => void
    years: number[]
    onAddYear: () => void
    onRequestDelete: (year: number) => void
}) {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Manage years</DialogTitle>
                </DialogHeader>
                {years.length === 0 ? (
                    <p className='text-muted-foreground text-sm'>
                        No custom years yet.
                    </p>
                ) : (
                    <div className='flex max-h-64 flex-col gap-1 overflow-y-auto'>
                        {years.map((year) => (
                            <div
                                key={year}
                                className='bg-background flex items-center justify-between gap-3 rounded-lg border p-2'
                            >
                                <span className='text-sm font-medium'>
                                    {year}
                                </span>
                                <Button
                                    type='button'
                                    variant='outline'
                                    size='sm'
                                    onClick={() => onRequestDelete(year)}
                                >
                                    <Trash2 data-icon='inline-start' />
                                    Remove
                                </Button>
                            </div>
                        ))}
                    </div>
                )}
                <DialogFooter>
                    <Button
                        type='button'
                        variant='outline'
                        onClick={() => onOpenChange(false)}
                    >
                        Done
                    </Button>
                    <Button type='button' onClick={onAddYear}>
                        <Plus data-icon='inline-start' />
                        Add year
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}

function DeleteYearDialog({
    year,
    open,
    error,
    deleting,
    onOpenChange,
    onConfirm,
}: {
    year: number
    open: boolean
    error: string | null
    deleting: boolean
    onOpenChange: (open: boolean) => void
    onConfirm: () => void | Promise<void>
}) {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>
                        Remove
                        {year}?
                    </DialogTitle>
                    <DialogDescription>
                        This removes {year} from the year selector.
                    </DialogDescription>
                </DialogHeader>
                {error && <p className='text-destructive text-sm'>{error}</p>}
                <DialogFooter>
                    <Button
                        type='button'
                        variant='outline'
                        disabled={deleting}
                        onClick={() => onOpenChange(false)}
                    >
                        Cancel
                    </Button>
                    <Button
                        type='button'
                        variant='destructive'
                        disabled={deleting}
                        onClick={() => void onConfirm()}
                    >
                        <Trash2 data-icon='inline-start' />
                        {deleting ? 'Removing...' : 'Remove'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}

function AddYearDialog({
    open,
    onOpenChange,
    onAdded,
}: {
    open: boolean
    onOpenChange: (open: boolean) => void
    onAdded: (year: number) => void | Promise<void>
}) {
    const [value, setValue] = useState('')
    const [error, setError] = useState<string | null>(null)
    const [submitting, setSubmitting] = useState(false)

    const submit = async (e: React.FormEvent) => {
        e.preventDefault()
        const parsed = parseInt(value, 10)
        if (
            !Number.isFinite(parsed) ||
            parsed < YEAR_MIN ||
            parsed > YEAR_MAX
        ) {
            setError(`Enter a year between ${YEAR_MIN} and ${YEAR_MAX}`)
            return
        }
        setSubmitting(true)
        setError(null)
        try {
            const result = await commands.addYear(parsed)
            if (result.status === 'error') {
                setError(result.error)
                return
            }
            await onAdded(parsed)
            onOpenChange(false)
        } catch (err) {
            setError(formatError(err))
        } finally {
            setSubmitting(false)
        }
    }

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Add custom year</DialogTitle>
                </DialogHeader>
                <form onSubmit={submit} className='flex flex-col gap-3'>
                    <div className='flex flex-col gap-1.5'>
                        <Label htmlFor='year-input'>Year</Label>
                        <Input
                            id='year-input'
                            type='number'
                            min={YEAR_MIN}
                            max={YEAR_MAX}
                            value={value}
                            onChange={(e) => setValue(e.target.value)}
                            autoFocus
                        />
                        {error && (
                            <p className='text-destructive text-xs'>{error}</p>
                        )}
                    </div>
                    <DialogFooter>
                        <Button
                            type='button'
                            variant='outline'
                            onClick={() => onOpenChange(false)}
                        >
                            Cancel
                        </Button>
                        <Button type='submit' disabled={submitting}>
                            Add
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    )
}

export { YearSelector }
