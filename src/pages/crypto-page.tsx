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
    type CalculatedCrypto,
    type CryptoTaxData,
    type Result,
} from "@/bindings";
import { CryptoFormModal } from "@/components/crypto-form-modal";
import { CurrencyValue } from "@/components/currency-value";
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
import { cryptoActionDisplay } from "@/lib/display";

const COLUMN_COUNT = 7;
const ROW_ESTIMATE_PX = 41;

function loadCryptos(year: number | null) {
    return commands.loadCryptos(year);
}

function CryptoPage() {
    const { year } = useYear();

    return <CryptoPageForYear key={year ?? "all"} year={year} />;
}

function CryptoPageForYear({ year }: { year: number | null }) {
    const [taxPromise, setTaxPromise] = useState(() => loadCryptos(year));
    const [, startTransition] = useTransition();

    const refresh = useCallback(() => {
        startTransition(() => {
            setTaxPromise(loadCryptos(year));
        });
    }, [year]);

    return (
        <Suspense
            fallback={
                <LoadingState
                    title="Loading crypto transactions"
                    message="Preparing your tax calculations."
                />
            }
        >
            <CryptoContent taxPromise={taxPromise} refresh={refresh} />
        </Suspense>
    );
}

function CryptoContent({
    taxPromise,
    refresh,
}: {
    taxPromise: Promise<Result<CryptoTaxData, string>>;
    refresh: () => void;
}) {
    const result = use(taxPromise);

    if (result.status === "error") {
        return (
            <ErrorState
                centered
                title="Couldn't load crypto transactions"
                message={result.error}
                onAction={refresh}
                actionLabel="Retry"
            />
        );
    }

    return <CryptoDataContent tax={result.data} refresh={refresh} />;
}

function CryptoDataContent({
    tax,
    refresh,
}: {
    tax: CryptoTaxData;
    refresh: () => void;
}) {
    const cryptos = tax.calculated;
    const [deleting, setDeleting] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [modalOpen, setModalOpen] = useState(false);
    const [editing, setEditing] = useState<CalculatedCrypto | null>(null);
    const [expandedIds, setExpandedIds] = useState<Set<string>>(
        () => new Set(),
    );
    const { selectMode, selectedIds, toggleRow, selectAll, clear } =
        useSelection(cryptos, (c) => c.id);

    const scrollRef = useRef<HTMLDivElement>(null);
    const rowVirtualizer = useVirtualizer({
        count: cryptos.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => ROW_ESTIMATE_PX,
        getItemKey: (index) => cryptos[index].id,
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
            const result = await commands.deleteCryptos(
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
            const result = await commands.deleteCryptos([id]);
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
                        totalCount={cryptos.length}
                        onSelectAll={selectAll}
                        onClean={clear}
                        onRemove={handleRemoveSelected}
                        deleting={deleting}
                    />
                </div>
            </div>

            {error && (
                <ErrorState
                    title="Couldn't update crypto entries"
                    message={error}
                />
            )}

            <div className="grid grid-cols-2 gap-3">
                <div className="rounded-lg border p-3">
                    <div className="text-xs text-muted-foreground">
                        Income (E-36)
                    </div>
                    <div className="font-mono text-lg">
                        <CurrencyValue value={tax.income} currency={"pln"} />
                    </div>
                </div>
                <div className="rounded-lg border p-3">
                    <div className="text-xs text-muted-foreground">
                        Costs (E-37)
                    </div>
                    <div className="font-mono text-lg">
                        <CurrencyValue value={tax.costs} currency={"pln"} />
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
                            <TableHead>Action</TableHead>
                            <TableHead className="text-right">Value</TableHead>
                            <TableHead className="text-right">Fee</TableHead>
                            <TableHead className="text-right">
                                Provider
                            </TableHead>
                            <TableHead className="text-right">
                                Actions
                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    {cryptos.length === 0 && (
                        <tbody>
                            <TableRow>
                                <TableCell
                                    colSpan={COLUMN_COUNT}
                                    className="py-6 text-center text-muted-foreground"
                                >
                                    No records yet. Use the Imports page to add
                                    crypto transactions.
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
                        const c = cryptos[virtualRow.index];
                        const selected = selectedIds.has(c.id);
                        const expanded = expandedIds.has(c.id);
                        return (
                            <tbody
                                key={c.id}
                                data-index={virtualRow.index}
                                ref={rowVirtualizer.measureElement}
                            >
                                <TableRow
                                    data-state={
                                        selected ? "selected" : undefined
                                    }
                                    className="cursor-pointer"
                                    onClick={() => toggleRow(c.id)}
                                >
                                    <TableCell
                                        className="font-mono text-xs text-muted-foreground"
                                        title={c.id}
                                    >
                                        {c.id.slice(0, 8)}
                                    </TableCell>
                                    <TableCell>{c.date}</TableCell>
                                    <TableCell>
                                        {cryptoActionDisplay[c.action]}
                                    </TableCell>
                                    <TableCell className="text-right font-mono">
                                        <CurrencyValue
                                            value={c.value.value}
                                            currency={c.value.currency}
                                        />
                                    </TableCell>
                                    <TableCell className="text-right font-mono">
                                        <CurrencyValue
                                            value={c.fee.value}
                                            currency={c.fee.currency}
                                        />
                                    </TableCell>
                                    <TableCell className="text-right font-mono text-xs font-semibold uppercase tracking-widest">
                                        {c.provider}
                                    </TableCell>
                                    <TableCell className="text-right">
                                        <div className="flex justify-end gap-1">
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    toggleExpanded(c.id);
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
                                                    setEditing(c);
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
                                                    handleDelete(c.id);
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
                                                <div className="grid grid-cols-3 gap-3">
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Calculated value
                                                            (PLN)
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    c.calculated_value
                                                                }
                                                                currency="pln"
                                                            />
                                                        </div>
                                                    </div>
                                                    <div>
                                                        <div className="text-xs text-muted-foreground">
                                                            Calculated fee (PLN)
                                                        </div>
                                                        <div className="font-mono">
                                                            <CurrencyValue
                                                                value={
                                                                    c.calculated_fee
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
                                                            {c.nbp_date}
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
                <CryptoFormModal
                    onClose={() => {
                        setModalOpen(false);
                        setEditing(null);
                    }}
                    onCreated={refresh}
                    crypto={editing}
                />
            )}
        </div>
    );
}

export { CryptoPage };
