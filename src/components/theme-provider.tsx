import {
    createContext,
    useContext,
    useEffect,
    useMemo,
    useState,
} from "react";

export type Theme = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";

type ThemeProviderProps = {
    children: React.ReactNode;
    defaultTheme?: Theme;
    storageKey?: string;
};

type ThemeProviderState = {
    theme: Theme;
    resolvedTheme: ResolvedTheme;
    setTheme: (theme: Theme) => void;
};

const ThemeProviderContext = createContext<ThemeProviderState | null>(null);

function systemTheme(): ResolvedTheme {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";
}

export function ThemeProvider({
    children,
    defaultTheme = "system",
    storageKey = "pitpls-theme",
}: ThemeProviderProps) {
    const [theme, setThemeState] = useState<Theme>(
        () => (localStorage.getItem(storageKey) as Theme | null) ?? defaultTheme,
    );
    const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() =>
        theme === "system" ? systemTheme() : theme,
    );

    useEffect(() => {
        if (theme !== "system") {
            setResolvedTheme(theme);
            return;
        }
        const media = window.matchMedia("(prefers-color-scheme: dark)");
        const update = () => setResolvedTheme(systemTheme());
        update();
        media.addEventListener("change", update);
        return () => media.removeEventListener("change", update);
    }, [theme]);

    useEffect(() => {
        document.documentElement.classList.toggle(
            "dark",
            resolvedTheme === "dark",
        );
    }, [resolvedTheme]);

    const value = useMemo<ThemeProviderState>(
        () => ({
            theme,
            resolvedTheme,
            setTheme: (next) => {
                localStorage.setItem(storageKey, next);
                setThemeState(next);
            },
        }),
        [theme, resolvedTheme, storageKey],
    );

    return (
        <ThemeProviderContext.Provider value={value}>
            {children}
        </ThemeProviderContext.Provider>
    );
}

export function useTheme() {
    const context = useContext(ThemeProviderContext);
    if (!context) {
        throw new Error("useTheme must be used within a ThemeProvider");
    }
    return context;
}
