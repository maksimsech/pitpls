import * as React from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface FileUploadProps extends Omit<
    React.ComponentProps<"input">,
    "type" | "onChange"
> {
    label?: string;
    onFilesChange?: (files: FileList | null) => void;
}

function FileUpload({
    id,
    className,
    label = "Choose file",
    multiple,
    accept,
    disabled,
    onFilesChange,
    ...props
}: FileUploadProps) {
    const inputRef = React.useRef<HTMLInputElement>(null);
    const [fileName, setFileName] = React.useState<string>("");

    function handleChange(event: React.ChangeEvent<HTMLInputElement>) {
        const files = event.target.files;
        if (!files || files.length === 0) {
            setFileName("");
        } else if (files.length === 1) {
            setFileName(files[0].name);
        } else {
            setFileName(`${files.length} files selected`);
        }
        onFilesChange?.(files);
    }

    return (
        <div
            data-slot="file-upload"
            className={cn("flex items-center gap-2", className)}
        >
            <input
                id={id}
                ref={inputRef}
                type="file"
                className="sr-only"
                multiple={multiple}
                accept={accept}
                disabled={disabled}
                onChange={handleChange}
                {...props}
            />
            <Button
                type="button"
                variant="outline"
                disabled={disabled}
                onClick={() => inputRef.current?.click()}
            >
                {label}
            </Button>
            <span className="truncate text-sm text-muted-foreground">
                {fileName || "No file selected"}
            </span>
        </div>
    );
}

export { FileUpload };
