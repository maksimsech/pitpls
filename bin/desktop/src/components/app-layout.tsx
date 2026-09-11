import { useState } from 'react'
import {
    ArrowLeft,
    Bitcoin,
    ChartLine,
    Coins,
    Home,
    Menu,
    Percent,
    Settings,
    Upload,
} from 'lucide-react'
import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom'

import { ThemeToggle } from '@/components/theme-toggle'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

const menuItems = [
    { to: '/', label: 'Home', icon: Home },
    { to: '/imports', label: 'Imports', icon: Upload },
    { to: '/dividends', label: 'Dividends', icon: Coins },
    { to: '/interests', label: 'Interests', icon: Percent },
    { to: '/crypto', label: 'Crypto', icon: Bitcoin },
    { to: '/rates', label: 'Rates', icon: ChartLine },
    { to: '/settings', label: 'Settings', icon: Settings },
]

function AppLayout() {
    const [menuOpen, setMenuOpen] = useState(false)
    const navigate = useNavigate()
    const location = useLocation()
    const isHome = location.pathname === '/'
    const title =
        menuItems.find((item) => item.to === location.pathname)?.label ?? ''

    return (
        <div className='relative min-h-screen'>
            <div className='bg-background/80 fixed top-4 left-4 z-50 flex items-center gap-2 rounded-lg border p-1 shadow-sm backdrop-blur'>
                <Button
                    variant='outline'
                    size='icon'
                    aria-label='Toggle menu'
                    aria-expanded={menuOpen}
                    onClick={() => setMenuOpen((v) => !v)}
                >
                    <Menu />
                </Button>
                {!isHome && (
                    <Button
                        variant='outline'
                        size='icon'
                        aria-label='Back'
                        onClick={() => navigate(-1)}
                    >
                        <ArrowLeft />
                    </Button>
                )}
                <ThemeToggle />
                {title && (
                    <h1 className='px-2 text-lg font-semibold tracking-tight'>
                        {title}
                    </h1>
                )}
            </div>

            {menuOpen && (
                <div
                    className='bg-background/40 fixed inset-0 z-30 backdrop-blur-sm'
                    onClick={() => setMenuOpen(false)}
                    aria-hidden
                />
            )}

            <aside
                className={cn(
                    'fixed top-0 bottom-0 left-0 z-40 w-64 border-r bg-background shadow-lg transition-transform duration-200 ease-in-out',
                    menuOpen ? 'translate-x-0' : '-translate-x-full',
                )}
            >
                <nav className='flex flex-col gap-1 p-3 pt-16'>
                    {menuItems.map(({ to, label, icon: Icon }) => {
                        const active = location.pathname === to
                        return (
                            <Link
                                key={to}
                                to={to}
                                onClick={() => setMenuOpen(false)}
                                className={cn(
                                    'flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors hover:bg-muted',
                                    active && 'bg-muted',
                                )}
                            >
                                <Icon className='size-4' />
                                {label}
                            </Link>
                        )
                    })}
                </nav>
            </aside>

            <main className='w-full px-6 pt-16 pb-6'>
                <Outlet />
            </main>
        </div>
    )
}

export { AppLayout }
