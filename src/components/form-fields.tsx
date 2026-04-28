import { type Currency } from "@/bindings";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { currencyDisplay } from "@/lib/display";

type LabeledInputProps = React.ComponentProps<typeof Input> & {
    id: string;
    label: string;
};

function LabeledInput({ id, label, ...props }: LabeledInputProps) {
    return (
        <div className="flex flex-col gap-1">
            <Label htmlFor={id}>{label}</Label>
            <Input id={id} {...props} />
        </div>
    );
}

type IdFieldProps = {
    id: string;
    value: string;
    onChange: (value: string) => void;
    isEdit: boolean;
};

function IdField({ id, value, onChange, isEdit }: IdFieldProps) {
    return (
        <LabeledInput
            id={id}
            label="ID"
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={isEdit ? "" : "Leave blank to auto-generate"}
            disabled={isEdit}
            readOnly={isEdit}
        />
    );
}

type EnumSelectProps<T extends string> = {
    id?: string;
    value: T;
    onChange: (value: T) => void;
    options: Record<T, string>;
    disabled?: boolean;
};

function EnumSelect<T extends string>({
    id,
    value,
    onChange,
    options,
    disabled,
}: EnumSelectProps<T>) {
    return (
        <Select
            value={value}
            onValueChange={(v) => onChange(v as T)}
            disabled={disabled}
        >
            <SelectTrigger id={id} className="w-full">
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    {(Object.entries(options) as [T, string][]).map(
                        ([val, label]) => (
                            <SelectItem key={val} value={val}>
                                {label}
                            </SelectItem>
                        ),
                    )}
                </SelectGroup>
            </SelectContent>
        </Select>
    );
}

type CurrencyAmountFieldProps = {
    id: string;
    label: string;
    value: string;
    onValueChange: (value: string) => void;
    currency: Currency;
    onCurrencyChange: (value: Currency) => void;
    currencies?: Record<Currency, string>;
};

function CurrencyAmountField({
    id,
    label,
    value,
    onValueChange,
    currency,
    onCurrencyChange,
    currencies = currencyDisplay,
}: CurrencyAmountFieldProps) {
    const currencyId = `${id}-currency`;
    return (
        <div className="grid grid-cols-[1fr_auto] gap-2">
            <LabeledInput
                id={id}
                label={label}
                inputMode="decimal"
                value={value}
                onChange={(e) => onValueChange(e.target.value)}
                placeholder="0.00"
                required
            />
            <div className="flex flex-col gap-1">
                <Label htmlFor={currencyId}>Currency</Label>
                <EnumSelect
                    id={currencyId}
                    value={currency}
                    onChange={onCurrencyChange}
                    options={currencies}
                />
            </div>
        </div>
    );
}

type LabeledSelectProps<T extends string> = EnumSelectProps<T> & {
    label: string;
};

function LabeledSelect<T extends string>({
    id,
    label,
    ...rest
}: LabeledSelectProps<T>) {
    return (
        <div className="flex flex-col gap-1">
            <Label htmlFor={id}>{label}</Label>
            <EnumSelect id={id} {...rest} />
        </div>
    );
}

export {
    CurrencyAmountField,
    EnumSelect,
    IdField,
    LabeledInput,
    LabeledSelect,
};
