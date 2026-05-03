import {
    Suspense,
    use,
    useCallback,
    useRef,
    useState,
    useTransition,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { open } from "@tauri-apps/plugin-dialog";

import {
    commands,
    type Currency,
    type RateDay,
    type RatesViewModel,
    type Result,
} from "@/bindings";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { formatError } from "@/lib/utils";

const ROW_ESTIMATE_PX = 41;
const EMPTY_RATE = "—";
const NBP_MIN_YEAR = 2002;
const currentYear = new Date().getFullYear();

function loadRates() {
    return commands.listRates();
}

function RatesPage() {
    const [ratesPromise, setRatesPromise] = useState(loadRates);
    const [, startTransition] = useTransition();

    const refresh = useCallback(() => {
        startTransition(() => {
            setRatesPromise(loadRates());
        });
    }, []);

    return (
        <Suspense
            fallback={
                <LoadingState
                    title="Loading rates"
                    message="Fetching imported exchange rates."
                />
            }
        >
            <RatesContent ratesPromise={ratesPromise} refresh={refresh} />
        </Suspense>
    );
}

function RatesContent({
    ratesPromise,
    refresh,
}: {
    ratesPromise: Promise<Result<RatesViewModel, string>>;
    refresh: () => void;
}) {
    const result = use(ratesPromise);
    const [uploading, setUploading] = useState(false);
    const [resetting, setResetting] = useState(false);
    const [importModalOpen, setImportModalOpen] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const rates = result.status === "ok" ? result.data.rows : [];
    const currencies = result.status === "ok" ? result.data.currencies : [];
    const columnCount = currencies.length + 1;

    const scrollRef = useRef<HTMLDivElement>(null);
    const rowVirtualizer = useVirtualizer({
        count: rates.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => ROW_ESTIMATE_PX,
        getItemKey: (index) => rates[index].date,
        overscan: 8,
    });

    if (result.status === "error") {
        return (
            <ErrorState
                centered
                title="Couldn't load rates"
                message={result.error}
                onAction={refresh}
                actionLabel="Retry"
            />
        );
    }

    const virtualItems = rowVirtualizer.getVirtualItems();
    const totalSize = rowVirtualizer.getTotalSize();
    const paddingTop = virtualItems[0]?.start ?? 0;
    const paddingBottom =
        virtualItems.length > 0
            ? totalSize - virtualItems[virtualItems.length - 1].end
            : 0;

    async function handleUploadCsv() {
        const file = await open({
            multiple: false,
            directory: false,
            filters: [{ name: "CSV", extensions: ["csv"] }],
        });
        if (typeof file !== "string") return;

        setUploading(true);
        setError(null);
        try {
            const result = await commands.importCsv(file);
            if (result.status === "error") {
                setError(result.error);
                return;
            }
            refresh();
        } catch (e) {
            setError(formatError(e));
        } finally {
            setUploading(false);
        }
    }

    async function handleResetRates() {
        setResetting(true);
        setError(null);
        try {
            const result = await commands.resetRates();
            if (result.status === "error") {
                setError(result.error);
                return;
            }
            refresh();
        } catch (e) {
            setError(formatError(e));
        } finally {
            setResetting(false);
        }
    }

    async function handleImportedFromNbp() {
        setImportModalOpen(false);
        refresh();
    }

    return (
        <div className="flex h-[calc(100vh-5.5rem)] flex-col gap-3">
            <div className="flex items-center justify-end gap-2">
                <Button
                    variant="destructive"
                    onClick={handleResetRates}
                    disabled={resetting || rates.length === 0}
                >
                    {resetting ? "Resetting…" : "Reset"}
                </Button>
                <Button onClick={handleUploadCsv} disabled={uploading}>
                    {uploading ? "Uploading…" : "Upload CSV"}
                </Button>
                <Button
                    variant="outline"
                    onClick={() => setImportModalOpen(true)}
                >
                    Import from NBP
                </Button>
            </div>

            {error && (
                <ErrorState title="Couldn't update rates" message={error} />
            )}

            <div
                ref={scrollRef}
                className="min-h-0 flex-1 overflow-auto rounded-lg border"
            >
                <table className="w-full min-w-max caption-bottom text-sm">
                    <TableHeader className="sticky top-0 z-10 bg-muted">
                        <TableRow>
                            <TableHead className="whitespace-nowrap">
                                Date
                            </TableHead>
                            {currencies.map((currency) => (
                                <TableHead
                                    key={currency}
                                    className="whitespace-nowrap text-right"
                                >
                                    {currency}
                                </TableHead>
                            ))}
                        </TableRow>
                    </TableHeader>
                    {rates.length === 0 && (
                        <tbody>
                            <TableRow>
                                <TableCell
                                    colSpan={columnCount}
                                    className="py-6 text-center text-muted-foreground"
                                >
                                    No rates yet. Upload a CSV to get started.
                                </TableCell>
                            </TableRow>
                        </tbody>
                    )}
                    {paddingTop > 0 && (
                        <tbody>
                            <tr style={{ height: paddingTop }}>
                                <td colSpan={columnCount} />
                            </tr>
                        </tbody>
                    )}
                    {virtualItems.map((virtualRow) => {
                        const r = rates[virtualRow.index];
                        const ratesByCurrency = rateLookup(r);
                        return (
                            <tbody
                                key={r.date}
                                data-index={virtualRow.index}
                                ref={rowVirtualizer.measureElement}
                            >
                                <TableRow>
                                    <TableCell className="whitespace-nowrap">
                                        {r.date}
                                    </TableCell>
                                    {currencies.map((currency) => (
                                        <TableCell
                                            key={currency}
                                            className="whitespace-nowrap text-right font-mono"
                                        >
                                            {ratesByCurrency.get(currency) ??
                                                EMPTY_RATE}
                                        </TableCell>
                                    ))}
                                </TableRow>
                            </tbody>
                        );
                    })}
                    {paddingBottom > 0 && (
                        <tbody>
                            <tr style={{ height: paddingBottom }}>
                                <td colSpan={columnCount} />
                            </tr>
                        </tbody>
                    )}
                </table>
            </div>

            {importModalOpen && (
                <NbpImportModal
                    onClose={() => setImportModalOpen(false)}
                    onImported={handleImportedFromNbp}
                />
            )}
        </div>
    );
}

function rateLookup(row: RateDay) {
    return new Map<Currency, string>(
        row.rates.map((rate) => [rate.currency, rate.rate]),
    );
}

function NbpImportModal({
    onClose,
    onImported,
}: {
    onClose: () => void;
    onImported: () => void | Promise<void>;
}) {
    const [year, setYear] = useState(String(currentYear));
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();

        const parsed = parseInt(year, 10);
        if (
            !Number.isFinite(parsed) ||
            parsed < NBP_MIN_YEAR ||
            parsed > currentYear
        ) {
            setError(`Enter a year between ${NBP_MIN_YEAR} and ${currentYear}`);
            return;
        }

        setSubmitting(true);
        setError(null);
        try {
            const result = await commands.importApi(parsed);
            if (result.status === "error") {
                setError(result.error);
                return;
            }
            await onImported();
        } catch (e) {
            setError(formatError(e));
        } finally {
            setSubmitting(false);
        }
    }

    return (
        <Dialog open onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="sm:max-w-sm">
                <DialogHeader>
                    <DialogTitle>Import from NBP</DialogTitle>
                </DialogHeader>
                <form onSubmit={handleSubmit} className="flex flex-col gap-3">
                    <div className="flex flex-col gap-1.5">
                        <Label htmlFor="nbp-year">Year</Label>
                        <Input
                            id="nbp-year"
                            type="number"
                            min={NBP_MIN_YEAR}
                            max={currentYear}
                            value={year}
                            onChange={(e) => setYear(e.target.value)}
                            required
                            autoFocus
                        />
                        {error && (
                            <p className="text-xs text-destructive">{error}</p>
                        )}
                    </div>
                    <DialogFooter>
                        <Button
                            type="button"
                            variant="outline"
                            onClick={onClose}
                            disabled={submitting}
                        >
                            Cancel
                        </Button>
                        <Button type="submit" disabled={submitting}>
                            {submitting ? "Importing…" : "Import"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}

export { RatesPage };
