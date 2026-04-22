import React from "react";
import ReactDOM from "react-dom/client";
import { createHashRouter, RouterProvider } from "react-router-dom";

import { AppLayout } from "./components/app-layout";
import { MainPage } from "./pages/main-page";
import { RatesPage } from "./pages/rates-page";
import { ThemeProvider } from "./components/theme-provider";
import { YearProvider } from "./components/year-provider";
import { TooltipProvider } from "./components/ui/tooltip";
import "./index.css";
import { CryptoPage } from "./pages/crypto-page";
import { DividendPage } from "./pages/dividend-page";
import { ImportsPage } from "./pages/imports-page";
import { InterestPage } from "./pages/interest-page";

const router = createHashRouter([
    {
        element: <AppLayout />,
        children: [
            { path: "/", element: <MainPage /> },
            { path: "/imports", element: <ImportsPage /> },
            { path: "/rates", element: <RatesPage /> },
            { path: "/crypto", element: <CryptoPage /> },
            { path: "/dividends", element: <DividendPage /> },
            { path: "/interests", element: <InterestPage /> },
        ],
    },
]);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <ThemeProvider>
            <YearProvider>
                <TooltipProvider>
                    <RouterProvider router={router} />
                </TooltipProvider>
            </YearProvider>
        </ThemeProvider>
    </React.StrictMode>,
);
