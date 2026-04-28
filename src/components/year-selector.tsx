import { useCallback, useEffect, useState } from "react";
import { Plus, Trash2 } from "lucide-react";

import { commands } from "@/bindings";
import { useYear } from "@/components/year-provider";
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
    Select,
    SelectContent,
    SelectItem,
    SelectSeparator,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { formatError } from "@/lib/utils";

const ALL = "all";
const YEAR_MIN = 1900;
const YEAR_MAX = 2100;

function YearSelector() {
    const { year, setYear } = useYear();
    const [options, setOptions] = useState<number[]>([]);
    const [open, setOpen] = useState(false);
    const [addOpen, setAddOpen] = useState(false);

    const refresh = useCallback(async () => {
        const result = await commands.listYears();
        if (result.status === "ok") {
            setOptions(result.data);
        }
    }, []);

    useEffect(() => {
        void refresh();
    }, [refresh]);

    const handleValueChange = (value: string) => {
        if (value === ALL) {
            setYear(null);
        } else {
            setYear(parseInt(value, 10));
        }
    };

    const handleDelete = async (value: number) => {
        await commands.deleteYear(value);
        if (year === value) setYear(null);
        await refresh();
    };

    const openAddDialog = () => {
        setOpen(false);
        setTimeout(() => setAddOpen(true), 0);
    };

    const handleAdded = async (newYear: number) => {
        await refresh();
        setYear(newYear);
    };

    const currentValue = year === null ? ALL : String(year);

    return (
        <>
            <Select
                value={currentValue}
                onValueChange={handleValueChange}
                open={open}
                onOpenChange={setOpen}
            >
                <SelectTrigger className="min-w-28">
                    <SelectValue>
                        {year === null ? "All" : String(year)}
                    </SelectValue>
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value={ALL}>All</SelectItem>
                    {options.map((y) => (
                        <SelectItem
                            key={y}
                            value={String(y)}
                            className="pr-10"
                        >
                            <span className="flex w-full items-center justify-between gap-2">
                                <span>{y}</span>
                                <span
                                    role="button"
                                    aria-label={`Remove ${y} from list`}
                                    className="ml-auto inline-flex size-5 items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                                    onPointerDown={(e) => {
                                        e.preventDefault();
                                        e.stopPropagation();
                                    }}
                                    onPointerUp={(e) => {
                                        e.preventDefault();
                                        e.stopPropagation();
                                        void handleDelete(y);
                                    }}
                                    onClick={(e) => {
                                        e.preventDefault();
                                        e.stopPropagation();
                                    }}
                                >
                                    <Trash2 className="size-3" />
                                </span>
                            </span>
                        </SelectItem>
                    ))}
                    <SelectSeparator />
                    <Button
                        variant="ghost"
                        size="sm"
                        className="w-full justify-start"
                        onClick={openAddDialog}
                    >
                        <Plus className="size-3.5" />
                        Add custom year…
                    </Button>
                </SelectContent>
            </Select>

            {addOpen && (
                <AddYearDialog
                    open={addOpen}
                    onOpenChange={setAddOpen}
                    onAdded={handleAdded}
                />
            )}
        </>
    );
}

function AddYearDialog({
    open,
    onOpenChange,
    onAdded,
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onAdded: (year: number) => void | Promise<void>;
}) {
    const [value, setValue] = useState("");
    const [error, setError] = useState<string | null>(null);
    const [submitting, setSubmitting] = useState(false);

    const submit = async (e: React.FormEvent) => {
        e.preventDefault();
        const parsed = parseInt(value, 10);
        if (
            !Number.isFinite(parsed) ||
            parsed < YEAR_MIN ||
            parsed > YEAR_MAX
        ) {
            setError(`Enter a year between ${YEAR_MIN} and ${YEAR_MAX}`);
            return;
        }
        setSubmitting(true);
        setError(null);
        try {
            const result = await commands.addYear(parsed);
            if (result.status === "error") {
                setError(result.error);
                return;
            }
            await onAdded(parsed);
            onOpenChange(false);
        } catch (err) {
            setError(formatError(err));
        } finally {
            setSubmitting(false);
        }
    };

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <DialogHeader>
                    <DialogTitle>Add custom year</DialogTitle>
                </DialogHeader>
                <form onSubmit={submit} className="flex flex-col gap-3">
                    <div className="flex flex-col gap-1.5">
                        <Label htmlFor="year-input">Year</Label>
                        <Input
                            id="year-input"
                            type="number"
                            min={YEAR_MIN}
                            max={YEAR_MAX}
                            value={value}
                            onChange={(e) => setValue(e.target.value)}
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
                            onClick={() => onOpenChange(false)}
                        >
                            Cancel
                        </Button>
                        <Button type="submit" disabled={submitting}>
                            Add
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}

export { YearSelector };
