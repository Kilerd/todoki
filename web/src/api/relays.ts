import { fetcher } from "./fetcher";

// Relay API endpoints using openapi-typescript-fetch
// Note: These paths need to be added to schema.d.ts when running npm run api
export const fetchRelays = fetcher.path("/api/relays" as any).method("get").create();
export const fetchRelay = fetcher.path("/api/relays/{relay_id}" as any).method("get").create();
export const fetchProjectRelays = fetcher.path("/api/projects/{project_id}/relays" as any).method("get").create();
