import { writable } from "svelte/store";

export const socket = writable(null as WebSocket | null);
