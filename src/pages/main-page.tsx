import {
    Suspense,
    use,
    useCallback,
    useState,
    useTransition,
} from "react";
import { TriangleAlert } from "lucide-react";
import { Link } from "react-router-dom";

import {
    commands,
    type Result,
    type TaxSummary,
    type Warnings,
} from "@/bindings";
import { CurrencyValue } from "@/components/currency-value";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { YearSelector } from "@/components/year-selector";
import { useYear } from "@/components/year-provider";
import { NO_RATES_AVAILABLE_ERROR } from "@/lib/utils";

function loadWarnings(year: number | null) {
    return commands.getWarnings(year);
}

function loadTaxSummary(year: number | null) {
    return commands.loadTaxSummary(year);
}

function MainPage() {
    const { year } = useYear();

    return <MainPageForYear key={year ?? "all"} year={year} />;
}

function MainPageForYear({ year }: { year: number | null }) {
    const [warningsPromise, setWarningsPromise] = useState(() =>
        loadWarnings(year),
    );
    const [summaryPromise, setSummaryPromise] = useState(() =>
        loadTaxSummary(year),
    );
    const [, startTransition] = useTransition();

    const refresh = useCallback(() => {
        startTransition(() => {
            setWarningsPromise(loadWarnings(year));
            setSummaryPromise(loadTaxSummary(year));
        });
    }, [year]);

    return (
        <Suspense
            fallback={
                <LoadingState
                    title="Loading dashboard"
                    message="Collecting warnings and tax summary."
                />
            }
        >
            <MainContent
                warningsPromise={warningsPromise}
                summaryPromise={summaryPromise}
                refresh={refresh}
            />
        </Suspense>
    );
}

function MainContent({
    warningsPromise,
    summaryPromise,
    refresh,
}: {
    warningsPromise: Promise<Result<Warnings, string>>;
    summaryPromise: Promise<Result<TaxSummary, string>>;
    refresh: () => void;
}) {
    const warningsResult = use(warningsPromise);
    const summaryResult = use(summaryPromise);

    if (warningsResult.status === "error") {
        return (
            <ErrorState
                centered
                title="Couldn't load dashboard"
                message={warningsResult.error}
                onAction={refresh}
                actionLabel="Retry"
            />
        );
    }

    const warnings = warningsResult.data;
    const summary = summaryResult.status === "ok" ? summaryResult.data : null;
    const summaryError =
        summaryResult.status === "error" &&
        summaryResult.error !== NO_RATES_AVAILABLE_ERROR
            ? summaryResult.error
            : null;

    return (
        <div className="flex flex-col gap-3">
            <div className="flex items-center gap-2">
                <YearSelector />
            </div>
            {warnings.rates_empty && warnings.has_records_in_year && (
                <Link
                    to="/rates"
                    className="flex items-center gap-2 rounded-lg border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive transition-colors hover:bg-destructive/20"
                >
                    <TriangleAlert className="size-4" />
                    No rates loaded. Load rates from CSV.
                </Link>
            )}

            {summaryError && (
                <ErrorState
                    title="Tax summary unavailable"
                    message={summaryError}
                />
            )}

            {summary && (
                <>
                    <h2 className="text-xl font-semibold">Tax information</h2>

                    <section className="flex flex-col gap-2">
                        <h3 className="text-sm font-semibold uppercase tracking-widest text-muted-foreground">
                            Crypto
                        </h3>
                        <div className="grid grid-cols-2 gap-3">
                            <SummaryCell
                                label="Income (E-36)"
                                value={summary.crypto.income}
                            />
                            <SummaryCell
                                label="Costs (E-37)"
                                value={summary.crypto.costs}
                            />
                        </div>
                    </section>

                    <section className="flex flex-col gap-2">
                        <h3 className="text-sm font-semibold uppercase tracking-widest text-muted-foreground">
                            Foreign dividends and interest
                        </h3>
                        <div className="grid grid-cols-3 gap-3">
                            <SummaryCell
                                label="Income (I-65)"
                                value={summary.foreign.income}
                            />
                            <SummaryCell
                                label="Tax to pay (G-47)"
                                value={summary.foreign.tax_to_pay}
                            />
                            <SummaryCell
                                label="Paid tax (G-48)"
                                value={summary.foreign.tax_paid}
                            />
                        </div>
                    </section>
                </>
            )}
        </div>
    );
}

function SummaryCell({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-lg border p-3">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="font-mono text-lg">
                <CurrencyValue value={value} currency="pln" />
            </div>
        </div>
    );
}

export { MainPage };
