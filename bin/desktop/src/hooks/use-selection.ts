import { useState } from 'react'

function useSelection<T>(items: T[], getId: (item: T) => string) {
    const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
    const selectMode = selectedIds.size > 0

    function toggleRow(id: string) {
        if (!selectMode) {
            setSelectedIds(new Set([id]))
            return
        }
        setSelectedIds((prev) => {
            const next = new Set(prev)
            if (next.has(id)) {
                next.delete(id)
            } else {
                next.add(id)
            }
            return next
        })
    }

    function selectAll() {
        setSelectedIds(new Set(items.map(getId)))
    }

    function clear() {
        setSelectedIds(new Set())
    }

    return { selectMode, selectedIds, toggleRow, selectAll, clear }
}

export { useSelection }
