import { RefreshCcw, TriangleAlert } from 'lucide-react'

import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

type ErrorStateProps = {
    message: string
    title?: string
    actionLabel?: string
    onAction?: () => void
    centered?: boolean
    className?: string
}

function ErrorState({
    message,
    title = 'Something went wrong',
    actionLabel = 'Try again',
    onAction,
    centered = false,
    className,
}: ErrorStateProps) {
    return (
        <div
            className={cn(
                'w-full',
                centered &&
                    'flex h-[calc(100vh-5.5rem)] items-center justify-center',
                className,
            )}
        >
            <div
                role='alert'
                className={cn(
                    'w-full rounded-3xl border border-destructive/20 bg-card/70 shadow-sm',
                    centered ? 'mx-auto max-w-lg px-8 py-10' : 'px-4 py-4',
                )}
            >
                <div
                    className={cn(
                        'flex gap-4',
                        centered
                            ? 'flex-col items-center text-center'
                            : 'flex-col sm:flex-row sm:items-center sm:justify-between',
                    )}
                >
                    <div
                        className={cn(
                            'flex gap-3',
                            centered ? 'flex-col items-center' : 'items-start',
                        )}
                    >
                        <div className='bg-destructive/10 text-destructive ring-destructive/20 flex size-11 shrink-0 items-center justify-center rounded-2xl ring-1'>
                            <TriangleAlert />
                        </div>
                        <div
                            className={cn(
                                'flex flex-col gap-1.5',
                                centered && 'items-center',
                            )}
                        >
                            <h2 className='text-base font-semibold tracking-tight'>
                                {title}
                            </h2>
                            <p className='text-muted-foreground max-w-md text-sm leading-6 wrap-break-word'>
                                {message}
                            </p>
                        </div>
                    </div>

                    {onAction && (
                        <Button variant='outline' onClick={onAction}>
                            <RefreshCcw data-icon='inline-start' />
                            {actionLabel}
                        </Button>
                    )}
                </div>
            </div>
        </div>
    )
}

export { ErrorState }
