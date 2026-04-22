import { LoaderCircle } from "lucide-react";

import { cn } from "@/lib/utils";

function LoadingState({
    title = "Loading data",
    message = "Preparing this view.",
    className,
}: {
    title?: string;
    message?: string;
    className?: string;
}) {
    return (
        <div
            role="status"
            aria-live="polite"
            aria-busy="true"
            className={cn(
                "flex min-h-[calc(100vh-9rem)] flex-col items-center justify-center gap-3 px-6 py-10 text-center",
                className,
            )}
        >
            <LoaderCircle className="size-5 animate-spin text-muted-foreground" />
            <div className="flex max-w-sm flex-col gap-1">
                <p className="text-sm font-medium">{title}</p>
                <p className="text-sm text-muted-foreground">{message}</p>
            </div>
        </div>
    );
}

export { LoadingState };
