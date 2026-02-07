import { Fetcher } from "openapi-typescript-fetch";
import type { paths } from "./schema";
import { getToken } from "@/lib/auth";

const fetcher = Fetcher.for<paths>();

fetcher.configure({
  baseUrl: import.meta.env.VITE_API_URL,
  init: {
    headers: {},
  },
  use: [
    async (url, init, next) => {
      const token = getToken();
      if (token) {
        const headers = new Headers(init.headers);
        headers.set("Authorization", `Bearer ${token}`);
        return next(url, { ...init, headers });
      }
      return next(url, init);
    },
  ],
});

export default fetcher;
