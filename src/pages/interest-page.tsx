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
    type CalculatedInterest,
    type InterestTaxData,
    type Result,
} from "@/bindings";
import { CurrencyValue } from "@/components/currency-value";
import { ErrorState } from "@/components/error-state";
import { InterestFormModal } from "@/components/interest-form-modal";
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

const COLUMN_COUNT = 5;
const ROW_ESTIMATE_PX = 41;

function loadInterests(year: number | null) {
    return commands.loadInterests(year);
}

function InterestPage() {
    const { year } = useYear();

    return <InterestPageForYear key={year ?? "all"} year={year} />;
}

function InterestPageForYear({ year }: { year: number | null }) {
    const [taxPromise, setTaxPromise] = useState(() => loadInterests(year));
    const [, startTransition] = useTransition();

    const refresh = useCallback(() => {
        startTransition(() => {
            setTaxPromise(loadInterests(year));
        });
    }, [year]);

    return (
        <Suspense
            fallback={
                <LoadingState
                    title="Loading interests"
                    message="Preparing the latest interest entries."
                />
            }
        >
            <InterestContent taxPromise={taxPromise} refresh={refresh} />
        </Suspense>
    );
}

function InterestContent({
    taxPromise,
    refresh,
}: {
    taxPromise: Promise<Result<InterestTaxData, string>>;
    refresh: () => void;
}) {
    const result = use(taxPromise);

    if (result.status === "error") {
        return (
            <ErrorState
                centered
                title="Couldn't load interests"
                message={result.error}
                onAction={refresh}
                actionLabel="Retry"
            />
        );
    }

    return <InterestDataContent tax={result.data} refresh={refresh} />;
}

function InterestDataContent({
    tax,
    refresh,
}: {
    tax: InterestTaxData;
    refresh: () => void;
}) {
    const interests = tax.calculated;
    const [deleting, setDeleting] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [modalOpen, setModalOpen] = useState(false);
    const [editing, setEditing] = useState<CalculatedInterest | null>(null);
    const [expandedIds, setExpandedIds] = useState<Set<string>>(
        () => new Set(),
    );
    const { selectMode, selectedIds, toggleRow, selectAll, clear } =
        useSelection(interests, (i) => i.id);

    const scrollRef = useRef<HTMLDivElement>(null);
    const rowVirtualizer = useVirtualizer({
        count: interests.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => ROW_ESTIMATE_PX,
        getItemKey: (index) => interests[index].id,
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
            const result = await commands.deleteInterests(
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
            const result = await commands.deleteInterests([id]);
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
                <SelectionHeader
                    selectMode={selectMode}
                    selectedCount={selectedIds.size}
                    totalCount={interests.length}
                    onSelectAll={selectAll}
                    onClean={clear}
                    onRemove={handleRemoveSelected}
                    deleting={deleting}
                />
            </div>

            {error && (
                <ErrorState title="Couldn't update interests" message={error} />
            )}

            <div className="grid grid-cols-2 gap-3">
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
                        Income (I-65)
                    </div>
                    <div className="font-mono text-lg">
                        <CurrencyValue value={tax.income} currency={"pln"} />
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
                            <TableHead className="text-right">Value</TableHead>
                            <TableHead className="text-right">
                                Provider
                            </TableHead>
                            <TableHead className="text-right">
                                Actions
                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    {interests.length === 0 && (
                        <tbody>
                            <TableRow>
                                <TableCell
                                    colSpan={COLUMN_COUNT}
                                    className="py-6 text-center text-muted-foreground"
                                >
                                    No records yet.
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
                        const i = interests[virtualRow.index];
                        const selected = selectedIds.has(i.id);
                        const expanded = expandedIds.has(i.id);
                        return (
                            <tbody
                                key={i.id}
                                data-index={virtualRow.index}
                                ref={rowVirtualizer.measureElement}
                            >
                                <TableRow
                                    data-state={
                                        selected ? "selected" : undefined
                                    }
                                    className="cursor-pointer"
                                    onClick={() => toggleRow(i.id)}
                                >
                                    <TableCell
                                        className="font-mono text-xs text-muted-foreground"
                                        title={i.id}
                                    >
                                        {i.id.slice(0, 8)}
                                    </TableCell>
                                    <TableCell>{i.date}</TableCell>
                                    <TableCell className="text-right font-mono">
                                        <CurrencyValue
                                            value={i.value.value}
                                            currency={i.value.currency}
                                        />
                                    </TableCell>
                                    <TableCell className="text-right font-mono text-xs font-semibold uppercase tracking-widest">
                                        {i.provider}
                                    </TableCell>
                                    <TableCell className="text-right">
                                        <div className="flex justify-end gap-1">
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    toggleExpanded(i.id);
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
                                                    setEditing(i);
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
                                                    handleDelete(i.id);
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
                                                <div className="grid grid-cols-2 gap-3">
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Calculated value
                                                            (PLN)
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    i.calculated_value
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
                                                            {i.nbp_date}
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
                <InterestFormModal
                    onClose={() => {
                        setModalOpen(false);
                        setEditing(null);
                    }}
                    onCreated={refresh}
                    interest={editing}
                />
            )}
        </div>
    );
}

export { InterestPage };
