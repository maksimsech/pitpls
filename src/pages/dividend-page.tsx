import {
    Suspense,
    use,
    useCallback,
    useRef,
    useState,
    useTransition,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";

import {
    commands,
    type CalculatedDividend,
    type DividendTaxData,
    type Result,
} from "@/bindings";
import { CurrencyValue } from "@/components/currency-value";
import { DividendFormModal } from "@/components/dividend-form-modal";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { SelectionHeader } from "@/components/selection-header";
import { useYear } from "@/components/year-provider";
import { Button } from "@/components/ui/button";
import {
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { useSelection } from "@/hooks/use-selection";
import { formatError } from "@/lib/utils";

const COLUMN_COUNT = 8;
const ROW_ESTIMATE_PX = 41;

function loadDividends(year: number | null) {
    return commands.loadDividends(year);
}

function DividendPage() {
    const { year } = useYear();

    return <DividendPageForYear key={year ?? "all"} year={year} />;
}

function DividendPageForYear({ year }: { year: number | null }) {
    const [taxPromise, setTaxPromise] = useState(() => loadDividends(year));
    const [, startTransition] = useTransition();

    const refresh = useCallback(() => {
        startTransition(() => {
            setTaxPromise(loadDividends(year));
        });
    }, [year]);

    return (
        <Suspense
            fallback={
                <LoadingState
                    title="Loading dividends"
                    message="Calculating dividend totals and entries."
                />
            }
        >
            <DividendContent taxPromise={taxPromise} refresh={refresh} />
        </Suspense>
    );
}

function DividendContent({
    taxPromise,
    refresh,
}: {
    taxPromise: Promise<Result<DividendTaxData, string>>;
    refresh: () => void;
}) {
    const result = use(taxPromise);

    if (result.status === "error") {
        return (
            <ErrorState
                centered
                title="Couldn't load dividends"
                message={result.error}
                onAction={refresh}
                actionLabel="Retry"
            />
        );
    }

    return <DividendDataContent tax={result.data} refresh={refresh} />;
}

function DividendDataContent({
    tax,
    refresh,
}: {
    tax: DividendTaxData;
    refresh: () => void;
}) {
    const dividends = tax.calculated;
    const [deleting, setDeleting] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [modalOpen, setModalOpen] = useState(false);
    const [editing, setEditing] = useState<CalculatedDividend | null>(null);
    const [expandedIds, setExpandedIds] = useState<Set<string>>(
        () => new Set(),
    );
    const { selectMode, selectedIds, toggleRow, selectAll, clear } =
        useSelection(dividends, (d) => d.id);

    const scrollRef = useRef<HTMLDivElement>(null);
    const rowVirtualizer = useVirtualizer({
        count: dividends.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => ROW_ESTIMATE_PX,
        getItemKey: (index) => dividends[index].id,
        overscan: 8,
    });

    function toggleExpanded(id: string) {
        setExpandedIds((prev) => {
            const next = new Set(prev);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return next;
        });
    }

    async function handleRemoveSelected() {
        setDeleting(true);
        setError(null);
        try {
            const result = await commands.deleteDividends(
                Array.from(selectedIds),
            );
            if (result.status === "error") {
                setError(result.error);
                return;
            }
            clear();
            refresh();
        } catch (e) {
            setError(formatError(e));
        } finally {
            setDeleting(false);
        }
    }

    async function handleDelete(id: string) {
        setDeleting(true);
        setError(null);
        try {
            const result = await commands.deleteDividends([id]);
            if (result.status === "error") {
                setError(result.error);
                return;
            }
            refresh();
        } catch (e) {
            setError(formatError(e));
        } finally {
            setDeleting(false);
        }
    }

    const virtualItems = rowVirtualizer.getVirtualItems();
    const totalSize = rowVirtualizer.getTotalSize();
    const paddingTop = virtualItems[0]?.start ?? 0;
    const paddingBottom =
        virtualItems.length > 0
            ? totalSize - virtualItems[virtualItems.length - 1].end
            : 0;

    return (
        <div className="flex h-[calc(100vh-5.5rem)] flex-col gap-3">
            <div className="flex items-center justify-between gap-2">
                <Button onClick={() => setModalOpen(true)}>Add new</Button>
                <div className="flex items-center justify-between gap-2">
                    {expandedIds.size > 0 && (
                        <Button
                            variant="outline"
                            onClick={() => setExpandedIds(new Set())}
                        >
                            Collapse all
                        </Button>
                    )}
                    <SelectionHeader
                        selectMode={selectMode}
                        selectedCount={selectedIds.size}
                        totalCount={dividends.length}
                        onSelectAll={selectAll}
                        onClean={clear}
                        onRemove={handleRemoveSelected}
                        deleting={deleting}
                    />
                </div>
            </div>

            {error && (
                <ErrorState title="Couldn't update dividends" message={error} />
            )}

            <div className="grid grid-cols-3 gap-3">
                <div className="rounded-lg border p-3">
                    <div className="text-xs text-muted-foreground">
                        To pay (G-47)
                    </div>
                    <div className="font-mono text-lg">
                        <CurrencyValue value={tax.to_pay} currency={"pln"} />
                    </div>
                </div>
                <div className="rounded-lg border p-3">
                    <div className="text-xs text-muted-foreground">
                        Paid (G-48)
                    </div>
                    <div className="font-mono text-lg">
                        <CurrencyValue value={tax.paid} currency={"pln"} />
                    </div>
                </div>
                <div className="rounded-lg border p-3">
                    <div className="text-xs text-muted-foreground">
                        Income (I-65)
                    </div>
                    <div className="font-mono text-lg">
                        <CurrencyValue value={tax.income} currency={"pln"} />
                        <span> ({tax.calculated.length})</span>
                    </div>
                </div>
            </div>

            <div
                ref={scrollRef}
                className="min-h-0 flex-1 overflow-auto rounded-lg border"
            >
                <table className="w-full caption-bottom text-sm">
                    <TableHeader className="sticky top-0 z-10 bg-muted">
                        <TableRow>
                            <TableHead>ID</TableHead>
                            <TableHead>Date</TableHead>
                            <TableHead>Ticker</TableHead>
                            <TableHead>Country</TableHead>
                            <TableHead className="text-right">Value</TableHead>
                            <TableHead className="text-right">
                                Tax paid
                            </TableHead>
                            <TableHead className="text-right">
                                Provider
                            </TableHead>
                            <TableHead className="text-right">
                                Actions
                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    {dividends.length === 0 && (
                        <tbody>
                            <TableRow>
                                <TableCell
                                    colSpan={COLUMN_COUNT}
                                    className="py-6 text-center text-muted-foreground"
                                >
                                    No records yet. Use the Imports page to add
                                    dividends.
                                </TableCell>
                            </TableRow>
                        </tbody>
                    )}
                    {paddingTop > 0 && (
                        <tbody>
                            <tr style={{ height: paddingTop }}>
                                <td colSpan={COLUMN_COUNT} />
                            </tr>
                        </tbody>
                    )}
                    {virtualItems.map((virtualRow) => {
                        const d = dividends[virtualRow.index];
                        const selected = selectedIds.has(d.id);
                        const expanded = expandedIds.has(d.id);
                        return (
                            <tbody
                                key={d.id}
                                data-index={virtualRow.index}
                                ref={rowVirtualizer.measureElement}
                            >
                                <TableRow
                                    data-state={
                                        selected ? "selected" : undefined
                                    }
                                    className="cursor-pointer"
                                    onClick={() => toggleRow(d.id)}
                                >
                                    <TableCell
                                        className="font-mono text-xs text-muted-foreground"
                                        title={d.id}
                                    >
                                        {d.id.slice(0, 8)}
                                    </TableCell>
                                    <TableCell>{d.date}</TableCell>
                                    <TableCell>{d.ticker}</TableCell>
                                    <TableCell className="font-mono">
                                        {d.country}
                                    </TableCell>
                                    <TableCell className="text-right font-mono">
                                        <CurrencyValue
                                            value={d.value.value}
                                            currency={d.value.currency}
                                        />
                                    </TableCell>
                                    <TableCell className="text-right font-mono">
                                        <CurrencyValue
                                            value={d.tax_paid.value}
                                            currency={d.tax_paid.currency}
                                        />
                                    </TableCell>
                                    <TableCell className="text-right font-mono text-xs font-semibold uppercase tracking-widest">
                                        {d.provider}
                                    </TableCell>
                                    <TableCell className="text-right">
                                        <div className="flex justify-end gap-1">
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    toggleExpanded(d.id);
                                                }}
                                            >
                                                {expanded
                                                    ? "Collapse"
                                                    : "Expand"}
                                            </Button>
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    setEditing(d);
                                                    setModalOpen(true);
                                                }}
                                            >
                                                Edit
                                            </Button>
                                            <Button
                                                variant="destructive"
                                                size="sm"
                                                disabled={deleting}
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    handleDelete(d.id);
                                                }}
                                            >
                                                Delete
                                            </Button>
                                        </div>
                                    </TableCell>
                                </TableRow>
                                {expanded && (
                                    <TableRow
                                        data-state={
                                            selected ? "selected" : undefined
                                        }
                                    >
                                        <TableCell colSpan={COLUMN_COUNT}>
                                            <div className="w-0 min-w-full overflow-hidden whitespace-normal">
                                                <div className="grid grid-cols-5 gap-3">
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Calculated value
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    d.calculated_value
                                                                }
                                                                currency="pln"
                                                            />
                                                        </div>
                                                    </div>
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Calculated to pay
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    d.calculated_to_pay
                                                                }
                                                                currency="pln"
                                                            />
                                                        </div>
                                                    </div>
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Calculated tax paid
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    d.calculated_tax_paid
                                                                }
                                                                currency="pln"
                                                            />
                                                        </div>
                                                    </div>
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Max tax paid
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    d.max_tax_paid
                                                                }
                                                                currency="pln"
                                                            />
                                                        </div>
                                                    </div>
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Used tax paid
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    d.used_tax_paid
                                                                }
                                                                currency="pln"
                                                            />
                                                        </div>
                                                    </div>
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            NBP date
                                                        </div>
                                                        <div className="font-mono">
                                                            {d.nbp_date}
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        </TableCell>
                                    </TableRow>
                                )}
                            </tbody>
                        );
                    })}
                    {paddingBottom > 0 && (
                        <tbody>
                            <tr style={{ height: paddingBottom }}>
                                <td colSpan={COLUMN_COUNT} />
                            </tr>
                        </tbody>
                    )}
                </table>
            </div>

            {modalOpen && (
                <DividendFormModal
                    onClose={() => {
                        setModalOpen(false);
                        setEditing(null);
                    }}
                    onCreated={refresh}
                    dividend={editing}
                />
            )}
        </div>
    );
}

export { DividendPage };
