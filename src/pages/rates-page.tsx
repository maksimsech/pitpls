import { Suspense, use, useCallback, useState, useTransition } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import { commands, type Rate, type Result } from "@/bindings";
import { ErrorState } from "@/components/error-state";
import { LoadingState } from "@/components/loading-state";
import { Button } from "@/components/ui/button";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { formatError } from "@/lib/utils";

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
    ratesPromise: Promise<Result<Rate[], string>>;
    refresh: () => void;
}) {
    const result = use(ratesPromise);
    const [uploading, setUploading] = useState(false);
    const [resetting, setResetting] = useState(false);
    const [error, setError] = useState<string | null>(null);

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

    const rates = result.data;

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
            const result = await commands.uploadRates(file);
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

    return (
        <div className="flex flex-col gap-3">
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
            </div>

            {error && (
                <ErrorState title="Couldn't update rates" message={error} />
            )}

            <div className="min-h-0 flex-1 overflow-y-auto rounded-lg border">
                <Table>
                    <TableHeader className="sticky top-0 z-10 bg-muted">
                        <TableRow>
                            <TableHead>Date</TableHead>
                            <TableHead>Currency</TableHead>
                            <TableHead className="text-right">Rate</TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {rates.length === 0 && (
                            <TableRow>
                                <TableCell
                                    colSpan={3}
                                    className="py-6 text-center text-muted-foreground"
                                >
                                    No rates yet. Upload a CSV to get started.
                                </TableCell>
                            </TableRow>
                        )}
                        {rates.map((r) => (
                            <TableRow key={`${r.date}-${r.currency}`}>
                                <TableCell>{r.date}</TableCell>
                                <TableCell>{r.currency}</TableCell>
                                <TableCell className="text-right font-mono">
                                    {r.rate}
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </div>
        </div>
    );
}

export { RatesPage };
