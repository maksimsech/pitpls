import { createContext, useContext, useMemo, useState } from "react";

type YearProviderProps = {
    children: React.ReactNode;
    storageKey?: string;
};

type YearProviderState = {
    year: number | null;
    setYear: (next: number | null) => void;
};

const YearProviderContext = createContext<YearProviderState | null>(null);

function readStoredYear(storageKey: string): number | null {
    const raw = localStorage.getItem(storageKey);
    if (raw === null || raw === "all") return null;
    const n = parseInt(raw, 10);
    return Number.isFinite(n) ? n : null;
}

export function YearProvider({
    children,
    storageKey = "pitpls-year",
}: YearProviderProps) {
    const [year, setYearState] = useState<number | null>(() =>
        readStoredYear(storageKey),
    );

    const value = useMemo<YearProviderState>(
        () => ({
            year,
            setYear: (next) => {
                localStorage.setItem(
                    storageKey,
                    next === null ? "all" : String(next),
                );
                setYearState(next);
            },
        }),
        [year, storageKey],
    );

    return (
        <YearProviderContext.Provider value={value}>
            {children}
        </YearProviderContext.Provider>
    );
}

export function useYear() {
    const context = useContext(YearProviderContext);
    if (!context) {
        throw new Error("useYear must be used within a YearProvider");
    }
    return context;
}
