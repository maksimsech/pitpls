import { useState } from "react";

import { commands, type CalculatedCrypto } from "@/bindings";
import {
    CurrencyAmountField,
    IdField,
    LabeledInput,
    LabeledSelect,
} from "@/components/form-fields";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { cryptoActionDisplay, type Action } from "@/lib/display";
import { formatError, today } from "@/lib/utils";

type Props = {
    onClose: () => void;
    onCreated: () => void;
    crypto?: CalculatedCrypto | null;
};

function CryptoFormModal({ onClose, onCreated, crypto }: Props) {
    const isEdit = !!crypto;
    const [id, setId] = useState(crypto?.id ?? "");
    const [date, setDate] = useState(crypto?.date ?? today());
    const [action, setAction] = useState<Action>(crypto?.action ?? "FiatBuy");
    const [value, setValue] = useState(crypto?.value.value ?? "");
    const [valueCurrency, setValueCurrency] = useState(
        crypto?.value.currency ?? "USD",
    );
    const [fee, setFee] = useState(crypto?.fee.value ?? "");
    const [feeCurrency, setFeeCurrency] = useState(
        crypto?.fee.currency ?? "USD",
    );
    const [provider, setProvider] = useState(crypto?.provider ?? "");
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(e: React.FormEvent) {
        e.preventDefault();
        setSubmitting(true);
        setError(null);
        try {
            const base = {
                date,
                action,
                value: value.trim(),
                value_currency: valueCurrency,
                fee: fee.trim(),
                fee_currency: feeCurrency,
                provider: provider.trim(),
            };
            if (isEdit) {
                const result = await commands.updateCrypto({
                    ...base,
                    id: id.trim(),
                });
                if (result.status === "error") {
                    setError(result.error);
                    return;
                }
            } else {
                const result = await commands.createCrypto({
                    ...base,
                    id: id.trim() === "" ? null : id.trim(),
                });
                if (result.status === "error") {
                    setError(result.error);
                    return;
                }
            }
            onCreated();
            onClose();
        } catch (e) {
            setError(formatError(e));
        } finally {
            setSubmitting(false);
        }
    }

    return (
        <Dialog open onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="sm:max-w-lg">
                <DialogHeader>
                    <DialogTitle>
                        {isEdit ? "Edit crypto" : "Add crypto"}
                    </DialogTitle>
                </DialogHeader>
                <form onSubmit={handleSubmit} className="flex flex-col gap-3">
                    <IdField
                        id="crypto-id"
                        value={id}
                        onChange={setId}
                        isEdit={isEdit}
                    />
                    <LabeledInput
                        id="crypto-date"
                        label="Date"
                        type="date"
                        value={date}
                        onChange={(e) => setDate(e.target.value)}
                        required
                    />
                    <LabeledSelect
                        id="crypto-action"
                        label="Action"
                        value={action}
                        onChange={setAction}
                        options={cryptoActionDisplay}
                    />
                    <CurrencyAmountField
                        id="crypto-value"
                        label="Value"
                        value={value}
                        onValueChange={setValue}
                        currency={valueCurrency}
                        onCurrencyChange={setValueCurrency}
                    />
                    <CurrencyAmountField
                        id="crypto-fee"
                        label="Fee"
                        value={fee}
                        onValueChange={setFee}
                        currency={feeCurrency}
                        onCurrencyChange={setFeeCurrency}
                    />
                    <LabeledInput
                        id="crypto-provider"
                        label="Provider"
                        value={provider}
                        onChange={(e) => setProvider(e.target.value)}
                        required
                    />

                    {error && (
                        <p className="text-sm wrap-break-word text-destructive">
                            {error}
                        </p>
                    )}

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
                            {submitting ? "Saving…" : "Save"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}

export { CryptoFormModal };
