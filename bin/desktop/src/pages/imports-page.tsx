import { useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { Loader2, Upload } from 'lucide-react'

import {
    commands,
    type ImporterKind,
    IMPORTERS,
    type InputType,
} from '@/bindings'
import { ErrorState } from '@/components/error-state'
import { formatError } from '@/lib/utils'

const EXTENSION: Record<InputType, 'csv' | 'pdf'> = {
    Csv: 'csv',
    Pdf: 'pdf',
}

function ImportsPage() {
    const [busy, setBusy] = useState<ImporterKind | null>(null)
    const [error, setError] = useState<string | null>(null)

    async function handleImport(
        kind: ImporterKind,
        inputs: readonly InputType[],
    ) {
        const extensions = inputs.map((i) => EXTENSION[i])
        const file = await open({
            multiple: false,
            directory: false,
            filters: [{ name: extensions.join('/').toUpperCase(), extensions }],
        })
        if (typeof file !== 'string') return

        setBusy(kind)
        setError(null)
        try {
            const result = await commands.runImport(kind, file)
            if (result.status === 'error') {
                setError(result.error)
            }
        } catch (e) {
            setError(formatError(e))
        } finally {
            setBusy(null)
        }
    }

    return (
        <div className='flex flex-col gap-4'>
            {error && <ErrorState title='Import failed' message={error} />}
            <div className='flex flex-col gap-2'>
                {IMPORTERS.map((importer) => {
                    const isBusy = busy === importer.kind
                    return (
                        <button
                            key={importer.kind}
                            onClick={() =>
                                handleImport(importer.kind, importer.input)
                            }
                            disabled={busy !== null}
                            className='group bg-card/40 hover:border-foreground/20 hover:bg-muted/60 focus-visible:border-ring focus-visible:ring-ring/50 flex w-full items-center justify-between gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors focus-visible:ring-3 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50'
                        >
                            <div className='flex flex-col'>
                                <span className='text-sm font-medium'>
                                    {importer.name}
                                </span>
                                <span className='text-muted-foreground font-mono text-[0.65rem] tracking-widest uppercase'>
                                    {isBusy
                                        ? 'Importing…'
                                        : importer.input.join(' · ')}
                                </span>
                            </div>
                            <div className='flex items-center gap-2'>
                                <div className='flex gap-1'>
                                    {importer.output.map((o) => (
                                        <span
                                            key={o}
                                            className='text-muted-foreground rounded-md border px-2 py-0.5 text-xs'
                                        >
                                            {o}
                                        </span>
                                    ))}
                                </div>
                                {isBusy ? (
                                    <Loader2 className='text-muted-foreground size-4 animate-spin' />
                                ) : (
                                    <Upload className='text-muted-foreground size-4 opacity-0 transition-opacity group-hover:opacity-100' />
                                )}
                            </div>
                        </button>
                    )
                })}
            </div>
        </div>
    )
}

export { ImportsPage }
