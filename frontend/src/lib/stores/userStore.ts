import { writable } from "@macfja/svelte-persistent-store";
import type { UserAndCameras } from "../../types";

export const user = writable("user", null as UserAndCameras | null);
