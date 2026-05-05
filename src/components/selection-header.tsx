import { Button } from '@/components/ui/button'

type SelectionHeaderProps = {
    selectMode: boolean
    selectedCount: number
    totalCount: number
    onSelectAll: () => void
    onClean: () => void
    onRemove: () => void
    deleting: boolean
}

function SelectionHeader({
    selectMode,
    selectedCount,
    totalCount,
    onSelectAll,
    onClean,
    onRemove,
    deleting,
}: SelectionHeaderProps) {
    return (
        <header className='flex items-center justify-end gap-2'>
            <Button
                variant='outline'
                onClick={onSelectAll}
                disabled={deleting || totalCount === 0}
            >
                Select all
            </Button>
            {selectMode && (
                <>
                    <Button
                        variant='outline'
                        onClick={onClean}
                        disabled={deleting}
                    >
                        Clean selection
                    </Button>
                    <Button
                        variant='destructive'
                        onClick={onRemove}
                        disabled={deleting || selectedCount === 0}
                    >
                        {deleting ? 'Removing…' : 'Remove selected'}
                    </Button>
                </>
            )}
        </header>
    )
}

export { SelectionHeader }
