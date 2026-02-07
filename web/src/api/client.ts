import { Fetcher } from "openapi-typescript-fetch";
import type { paths } from "./schema";

const fetcher = Fetcher.for<paths>();

fetcher.configure({
  baseUrl: import.meta.env.VITE_API_URL,
  init: {
    headers: {
      Authorization: `Bearer ${import.meta.env.VITE_USER_TOKEN}`,
    },
  },
});

export default fetcher;
