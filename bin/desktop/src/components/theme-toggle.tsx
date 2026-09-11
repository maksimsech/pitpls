import { Moon, Sun } from 'lucide-react'

import { useTheme } from '@/components/theme-provider'
import { Button } from '@/components/ui/button'

export function ThemeToggle() {
    const { resolvedTheme, setTheme } = useTheme()
    const isDark = resolvedTheme === 'dark'

    return (
        <Button
            variant='ghost'
            size='icon'
            aria-label='Toggle theme'
            onClick={() => setTheme(isDark ? 'light' : 'dark')}
        >
            {isDark ? <Moon /> : <Sun />}
        </Button>
    )
}
