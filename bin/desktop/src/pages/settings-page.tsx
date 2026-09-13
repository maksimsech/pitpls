import { Suspense, use, useState } from 'react'

import {
    commands,
    type DividendRounding,
    type Result,
    type Settings,
} from '@/bindings'
import { ErrorState } from '@/components/error-state'
import { LabeledSelect } from '@/components/form-fields'
import { LoadingState } from '@/components/loading-state'
import { Button } from '@/components/ui/button'
import { dividendRoundingDisplay } from '@/lib/display'
import { formatError } from '@/lib/utils'

function loadSettings() {
    return commands.loadSettings()
}

function SettingsPage() {
    const [settingsPromise, setSettingsPromise] = useState(loadSettings)

    function refresh() {
        setSettingsPromise(loadSettings())
    }

    return (
        <Suspense
            fallback={
                <LoadingState
                    title='Loading settings'
                    message='Fetching application settings.'
                />
            }
        >
            <SettingsContent
                settingsPromise={settingsPromise}
                refresh={refresh}
            />
        </Suspense>
    )
}

function SettingsContent({
    settingsPromise,
    refresh,
}: {
    settingsPromise: Promise<Result<Settings, string>>
    refresh: () => void
}) {
    const result = use(settingsPromise)

    if (result.status === 'error') {
        return (
            <ErrorState
                centered
                title="Couldn't load settings"
                message={result.error}
                onAction={refresh}
                actionLabel='Retry'
            />
        )
    }

    return <SettingsDataContent settings={result.data} />
}

function SettingsDataContent({ settings }: { settings: Settings }) {
    const [dividendRounding, setDividendRounding] = useState(
        settings.dividend_rounding,
    )
    const [savedSettings, setSavedSettings] = useState(settings)
    const [saving, setSaving] = useState(false)
    const [saved, setSaved] = useState(false)
    const [error, setError] = useState<string | null>(null)

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault()
        setSaving(true)
        setSaved(false)
        setError(null)

        const nextSettings: Settings = {
            dividend_rounding: dividendRounding,
        }

        try {
            const result = await commands.updateSettings(nextSettings)
            if (result.status === 'error') {
                setError(result.error)
                return
            }
            setSavedSettings(nextSettings)
            setSaved(true)
        } catch (e) {
            setError(formatError(e))
        } finally {
            setSaving(false)
        }
    }

    const hasChanges = dividendRounding !== savedSettings.dividend_rounding

    return (
        <form
            onSubmit={handleSubmit}
            className='flex max-w-xl flex-col gap-6 px-2 pt-8 sm:px-4'
        >
            {error && (
                <ErrorState title="Couldn't update settings" message={error} />
            )}

            <LabeledSelect
                id='dividend-rounding'
                label='Dividend rounding'
                value={dividendRounding}
                onChange={(value: DividendRounding) => {
                    setDividendRounding(value)
                    setSaved(false)
                }}
                options={dividendRoundingDisplay}
                disabled={saving}
            />

            <div className='flex items-center gap-3'>
                <Button type='submit' disabled={saving || !hasChanges}>
                    {saving ? 'Saving…' : 'Save'}
                </Button>
                {saved && (
                    <p className='text-muted-foreground text-sm'>
                        Settings saved.
                    </p>
                )}
            </div>
        </form>
    )
}

export { SettingsPage }
