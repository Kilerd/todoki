import {Fetcher} from "openapi-typescript-fetch";
import {paths} from "../api/schema";
import {getToken} from "@/lib/auth.ts";

const API_BASE_URL =
    import.meta.env.VITE_API_BASE_URL || "http://localhost:8201";

if (import.meta.env.PROD && !import.meta.env.VITE_API_BASE_URL) {
    console.warn("VITE_API_BASE_URL is not set in production environment");
}

// Create fetcher instance
export const fetcher = Fetcher.for<paths>();

fetcher.configure({
    baseUrl: API_BASE_URL,
    init: {
        headers: {
            "Content-Type": "application/json",
        },
    },
    use: [
        // Auth middleware - add token to requests
        async (url, init, next) => {
            const token = getToken();
            if (token) {
                init.headers.set("Authorization", `Bearer ${token}`);
            }
            return next(url, init);
        },
    ],
});